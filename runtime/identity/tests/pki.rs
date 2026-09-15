use intent_identity::{
    pki::{renewal_message, PkiError, PkiStore, MAX_LEAF_LIFETIME_MS},
    tls::{client_config, server_config, TrustPolicy},
    AgentIdentity,
};
use rcgen::{CertificateParams, KeyPair, SigningKey, PKCS_ED25519};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::{io::Cursor, sync::Arc};
use tempfile::tempdir;

const NOW: u64 = 1_700_000_000_000;

fn identity(role: &str) -> AgentIdentity {
    AgentIdentity::parse(&format!(
        "spiffe://lab.local/agent/{role}/router-a/instance-a"
    ))
    .unwrap()
}

fn csr() -> (KeyPair, Vec<u8>) {
    let key = KeyPair::generate_for(&PKCS_ED25519).unwrap();
    // This malicious self-declared SAN must never control the issued identity.
    let params = CertificateParams::new(vec!["untrusted.example".to_owned()]).unwrap();
    let request = params.serialize_request(&key).unwrap();
    (key, request.der().as_ref().to_vec())
}

fn store() -> (tempfile::TempDir, PkiStore) {
    let directory = tempdir().unwrap();
    let store = PkiStore::open(directory.path().join("pki.sqlite"), "lab.local", NOW).unwrap();
    (directory, store)
}

fn handshake(client: Arc<rustls::ClientConfig>, server: Arc<rustls::ServerConfig>) -> Result<(), String> {
    let mut client = rustls::ClientConnection::new(client, "loopback.invalid".try_into().unwrap()).map_err(|e| e.to_string())?;
    let mut server = rustls::ServerConnection::new(server).map_err(|e| e.to_string())?;
    for _ in 0..16 {
        let mut outbound = Vec::new();
        client.write_tls(&mut outbound).map_err(|e| e.to_string())?;
        if !outbound.is_empty() {
            server.read_tls(&mut Cursor::new(outbound)).map_err(|e| e.to_string())?;
            server.process_new_packets().map_err(|e| e.to_string())?;
        }
        let mut outbound = Vec::new();
        server.write_tls(&mut outbound).map_err(|e| e.to_string())?;
        if !outbound.is_empty() {
            client.read_tls(&mut Cursor::new(outbound)).map_err(|e| e.to_string())?;
            client.process_new_packets().map_err(|e| e.to_string())?;
        }
        if !client.is_handshaking() && !server.is_handshaking() { return Ok(()); }
    }
    Err("TLS handshake did not complete".into())
}

#[derive(Debug)]
struct NoClientCertificate;
impl rustls::client::ResolvesClientCert for NoClientCertificate {
    fn resolve(&self, _: &[&[u8]], _: &[rustls::SignatureScheme]) -> Option<Arc<rustls::sign::CertifiedKey>> { None }
    fn has_certs(&self) -> bool { false }
}

#[test]
fn enrollment_uses_ticket_identity_and_rejects_replay_or_bad_csr() {
    let (_directory, store) = store();
    let (_key, csr_der) = csr();
    let ticket = store
        .create_enrollment_ticket(identity("witness"), &csr_der, NOW + 1_000, NOW)
        .unwrap();
    let issued = store
        .enroll(ticket.expose_to_enrollee(), &csr_der, 60_000, NOW)
        .unwrap();
    assert_eq!(issued.identity, identity("witness"));
    assert!(matches!(
        store.enroll(ticket.expose_to_enrollee(), &csr_der, 60_000, NOW),
        Err(PkiError::InvalidTicket)
    ));
    let mut bad = csr_der.clone();
    *bad.last_mut().unwrap() ^= 1;
    assert!(matches!(
        store.create_enrollment_ticket(identity("apply"), &bad, NOW + 1_000, NOW),
        Err(PkiError::InvalidCsr)
    ));
    let mut trailing = csr_der.clone();
    trailing.push(0);
    assert!(matches!(
        store.create_enrollment_ticket(identity("apply"), &trailing, NOW + 1_000, NOW),
        Err(PkiError::InvalidCsr)
    ));
}

#[test]
fn renewal_requires_current_key_proof_and_revocation_survives_restart() {
    let (directory, store) = store();
    let (key, csr_der) = csr();
    let ticket = store
        .create_enrollment_ticket(identity("apply"), &csr_der, NOW + 1_000, NOW)
        .unwrap();
    let issued = store
        .enroll(ticket.expose_to_enrollee(), &csr_der, 60_000, NOW)
        .unwrap();
    let (_fresh_key, fresh_csr) = csr();
    let proof = key.sign(&renewal_message(&fresh_csr)).unwrap();
    let renewed = store
        .renew(&issued.certificate_der, &proof, &fresh_csr, 60_000, NOW + 1)
        .unwrap();
    assert_eq!(renewed.identity, identity("apply"));
    assert!(matches!(
        store.renew(&issued.certificate_der, &proof, &fresh_csr, 60_000, NOW + 2),
        Err(PkiError::InvalidCredential)
    ));
    assert!(matches!(
        store.renew(&issued.certificate_der, b"bad", &fresh_csr, 60_000, NOW + 3),
        Err(PkiError::InvalidCredential)
    ));
    assert!(store.revoke(&renewed.certificate_der, NOW + 4).unwrap());
    drop(store);
    let store = PkiStore::open(directory.path().join("pki.sqlite"), "lab.local", NOW + 5).unwrap();
    assert!(matches!(
        store.renew(
            &renewed.certificate_der,
            &proof,
            &fresh_csr,
            60_000,
            NOW + 5
        ),
        Err(PkiError::InvalidCredential)
    ));
}

#[test]
fn roots_overlap_rotate_monotonically_and_keep_a_root() {
    let (_directory, store) = store();
    assert_eq!(store.trust_roots().unwrap().len(), 1);
    store.rotate_root(2, "lab.local", NOW + 1).unwrap();
    assert_eq!(store.trust_roots().unwrap().len(), 2);
    assert!(matches!(
        store.rotate_root(2, "lab.local", NOW + 2),
        Err(PkiError::InvalidGeneration)
    ));
    assert!(store.remove_trust_root(1).unwrap());
    assert!(matches!(
        store.remove_trust_root(2),
        Err(PkiError::LastRoot)
    ));
}

#[test]
fn issued_validity_matches_der_and_expiry_boundary() {
    let (_directory, store) = store();
    let (key, request) = csr();
    let at = NOW + 789;
    let ticket = store.create_enrollment_ticket(identity("apply"), &request, at + 10_000, at).unwrap();
    let issued = store.enroll(ticket.expose_to_enrollee(), &request, 2_999, at).unwrap();
    assert_eq!(issued.not_before_ms % 1_000, 0);
    assert_eq!(issued.not_after_ms - issued.not_before_ms, 2_000);
    let (_, fresh) = csr();
    let proof = key.sign(&renewal_message(&fresh)).unwrap();
    assert!(matches!(
        store.renew(&issued.certificate_der, &proof, &fresh, 1_000, issued.not_after_ms),
        Err(PkiError::InvalidCredential)
    ));
}

#[test]
fn root_domain_and_removed_root_cannot_authorize_renewal() {
    let (_directory, store) = store();
    assert!(matches!(store.rotate_root(2, "other.local", NOW), Err(PkiError::TrustDomainMismatch)));
    let (key, request) = csr();
    let ticket = store.create_enrollment_ticket(identity("witness"), &request, NOW + 10_000, NOW).unwrap();
    let issued = store.enroll(ticket.expose_to_enrollee(), &request, 1_000, NOW).unwrap();
    store.rotate_root(2, "lab.local", NOW + 1).unwrap();
    assert!(store.remove_trust_root(1).unwrap());
    let (_, fresh) = csr();
    let proof = key.sign(&renewal_message(&fresh)).unwrap();
    assert!(matches!(
        store.renew(&issued.certificate_der, &proof, &fresh, 1_000, NOW + 2),
        Err(PkiError::InvalidCredential)
    ));
}

#[test]
fn rejects_expired_tickets_and_leaf_lifetimes_over_a_day() {
    let (_directory, store) = store();
    let (_key, csr_der) = csr();
    assert!(matches!(
        store.create_enrollment_ticket(identity("witness"), &csr_der, NOW, NOW),
        Err(PkiError::InvalidTicket)
    ));
    let ticket = store
        .create_enrollment_ticket(identity("witness"), &csr_der, NOW + 1_000, NOW)
        .unwrap();
    assert!(matches!(
        store.enroll(
            ticket.expose_to_enrollee(),
            &csr_der,
            MAX_LEAF_LIFETIME_MS + 1,
            NOW
        ),
        Err(PkiError::InvalidLifetime)
    ));
    assert!(matches!(
        store.enroll(ticket.expose_to_enrollee(), &csr_der, 1, NOW + 1_000),
        Err(PkiError::InvalidLifetime)
    ));
}

#[test]
fn tls_configuration_binds_exact_spiffe_peer_uris() {
    let directory = tempdir().unwrap();
    let now = (time::OffsetDateTime::now_utc().unix_timestamp() as u64) * 1_000;
    let store = PkiStore::open(directory.path().join("pki.sqlite"), "lab.local", now).unwrap();
    let (server_key, server_csr) = csr();
    let (client_key, client_csr) = csr();
    let server_id = identity("controller");
    let client_id = identity("witness");
    let server_ticket = store.create_enrollment_ticket(server_id.clone(), &server_csr, now + 10_000, now).unwrap();
    let client_ticket = store.create_enrollment_ticket(client_id.clone(), &client_csr, now + 10_000, now).unwrap();
    let server_cert = store.enroll(server_ticket.expose_to_enrollee(), &server_csr, 60_000, now).unwrap();
    let client_cert = store.enroll(client_ticket.expose_to_enrollee(), &client_csr, 60_000, now).unwrap();
    let policy = TrustPolicy { roots: store.trust_roots().unwrap().into_iter().map(|r| CertificateDer::from(r.certificate_der)).collect(), trust_generation: 1, revoked_certificate_digests: Default::default() };
    let server_chain = vec![CertificateDer::from(server_cert.certificate_der.clone())];
    let client_chain = vec![CertificateDer::from(client_cert.certificate_der.clone())];
    let server_private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der()));
    let client_private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(client_key.serialize_der()));
    let server = server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), policy.clone()).unwrap();
    let client = client_config(client_chain.clone(), client_private.clone_key(), &client_id, server_id.clone(), policy.clone()).unwrap();
    assert!(handshake(client, server).is_ok());
    let mut no_certificate = (*client_config(client_chain.clone(), client_private.clone_key(), &client_id, server_id.clone(), policy.clone()).unwrap()).clone();
    no_certificate.client_auth_cert_resolver = Arc::new(NoClientCertificate);
    assert!(handshake(
        Arc::new(no_certificate),
        server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), policy.clone()).unwrap(),
    ).is_err());
    // These fail after the peer presents a chain that WebPKI accepts: the URI
    // verifier must reject the exact role and trust-domain mismatch.
    assert!(handshake(
        client_config(client_chain.clone(), client_private.clone_key(), &client_id, identity("apply"), policy.clone()).unwrap(),
        server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), policy.clone()).unwrap(),
    ).is_err());
    let other_domain = AgentIdentity::parse("spiffe://other.local/agent/controller/router-a/instance-a").unwrap();
    assert!(handshake(
        client_config(client_chain.clone(), client_private.clone_key(), &client_id, other_domain, policy.clone()).unwrap(),
        server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), policy.clone()).unwrap(),
    ).is_err());
    let other_directory = tempdir().unwrap();
    let other_store = PkiStore::open(other_directory.path().join("pki.sqlite"), "lab.local", now).unwrap();
    let untrusted = TrustPolicy { roots: other_store.trust_roots().unwrap().into_iter().map(|r| CertificateDer::from(r.certificate_der)).collect(), trust_generation: 1, revoked_certificate_digests: Default::default() };
    assert!(handshake(
        client_config(client_chain.clone(), client_private.clone_key(), &client_id, server_id.clone(), untrusted).unwrap(),
        server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), policy.clone()).unwrap(),
    ).is_err());
    let mut revoked_server = policy.clone();
    revoked_server.revoked_certificate_digests.insert(intent_protocol::payload_digest(&server_cert.certificate_der));
    assert!(handshake(
        client_config(client_chain.clone(), client_private.clone_key(), &client_id, server_id.clone(), revoked_server).unwrap(),
        server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), policy.clone()).unwrap(),
    ).is_err());
    let mut revoked_client = policy.clone();
    revoked_client.revoked_certificate_digests.insert(intent_protocol::payload_digest(&client_cert.certificate_der));
    assert!(handshake(
        client_config(client_chain.clone(), client_private.clone_key(), &client_id, server_id.clone(), policy.clone()).unwrap(),
        server_config(server_chain.clone(), server_private.clone_key(), &server_id, client_id.clone(), revoked_client).unwrap(),
    ).is_err());
    // A certificate and private key are bound by Rustls before a connection is made.
    assert!(client_config(client_chain.clone(), server_private.clone_key(), &client_id, server_id.clone(), policy.clone()).is_err());
    // A role/name mismatch in local material is rejected before a configuration can be used.
    let wrong = identity("apply");
    assert!(client_config(vec![CertificateDer::from(client_cert.certificate_der)], client_private, &wrong, server_id, TrustPolicy::default()).is_err());
}

#[test]
fn tls_rejects_expired_leaf_and_expired_root_bundle() {
    let directory = tempdir().unwrap();
    let now = (time::OffsetDateTime::now_utc().unix_timestamp() as u64) * 1_000;
    let store = PkiStore::open(directory.path().join("pki.sqlite"), "lab.local", now - 120_000).unwrap();
    let server_id = identity("controller");
    let client_id = identity("witness");
    let (expired_key, expired_csr) = csr();
    let expired_ticket = store.create_enrollment_ticket(server_id.clone(), &expired_csr, now - 100_000, now - 120_000).unwrap();
    let expired = store.enroll(expired_ticket.expose_to_enrollee(), &expired_csr, 1_000, now - 120_000).unwrap();
    let (client_key, client_csr) = csr();
    let client_ticket = store.create_enrollment_ticket(client_id.clone(), &client_csr, now + 10_000, now).unwrap();
    let client = store.enroll(client_ticket.expose_to_enrollee(), &client_csr, 60_000, now).unwrap();
    let policy = TrustPolicy { roots: store.trust_roots().unwrap().into_iter().map(|r| CertificateDer::from(r.certificate_der)).collect(), trust_generation: 1, revoked_certificate_digests: Default::default() };
    let server = server_config(vec![CertificateDer::from(expired.certificate_der.clone())], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(expired_key.serialize_der())), &server_id, client_id.clone(), policy.clone()).unwrap();
    let client = client_config(vec![CertificateDer::from(client.certificate_der.clone())], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(client_key.serialize_der())), &client_id, server_id.clone(), policy).unwrap();
    assert!(handshake(client, server).is_err());

    let expired_root_directory = tempdir().unwrap();
    let expired_root = PkiStore::open(expired_root_directory.path().join("pki.sqlite"), "lab.local", now - (366 * 24 * 60 * 60 * 1_000)).unwrap();
    let expired_bundle = TrustPolicy { roots: expired_root.trust_roots().unwrap().into_iter().map(|r| CertificateDer::from(r.certificate_der)).collect(), trust_generation: 1, revoked_certificate_digests: Default::default() };
    assert!(client_config(
        vec![CertificateDer::from(expired.certificate_der.clone())],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(expired_key.serialize_der())),
        &server_id,
        client_id,
        expired_bundle,
    ).is_err());
}

#[test]
fn tls_rotation_accepts_overlap_and_rejects_removed_root() {
    let directory = tempdir().unwrap();
    let now = (time::OffsetDateTime::now_utc().unix_timestamp() as u64) * 1_000;
    let store = PkiStore::open(directory.path().join("pki.sqlite"), "lab.local", now).unwrap();
    let server_id = identity("controller");
    let client_id = identity("witness");
    let (old_client_key, old_client_csr) = csr();
    let old_client_ticket = store.create_enrollment_ticket(client_id.clone(), &old_client_csr, now + 10_000, now).unwrap();
    let old_client = store.enroll(old_client_ticket.expose_to_enrollee(), &old_client_csr, 60_000, now).unwrap();
    store.rotate_root(2, "lab.local", now + 1).unwrap();
    let (new_server_key, new_server_csr) = csr();
    let new_server_ticket = store.create_enrollment_ticket(server_id.clone(), &new_server_csr, now + 10_000, now + 1).unwrap();
    let new_server = store.enroll(new_server_ticket.expose_to_enrollee(), &new_server_csr, 60_000, now + 1).unwrap();
    let overlap = TrustPolicy { roots: store.trust_roots().unwrap().into_iter().map(|r| CertificateDer::from(r.certificate_der)).collect(), trust_generation: 2, revoked_certificate_digests: Default::default() };
    assert!(handshake(
        client_config(vec![CertificateDer::from(old_client.certificate_der.clone())], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(old_client_key.serialize_der())), &client_id, server_id.clone(), overlap.clone()).unwrap(),
        server_config(vec![CertificateDer::from(new_server.certificate_der.clone())], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(new_server_key.serialize_der())), &server_id, client_id.clone(), overlap).unwrap(),
    ).is_ok());
    assert!(store.remove_trust_root(1).unwrap());
    let removed = TrustPolicy { roots: store.trust_roots().unwrap().into_iter().map(|r| CertificateDer::from(r.certificate_der)).collect(), trust_generation: 3, revoked_certificate_digests: Default::default() };
    assert!(handshake(
        client_config(vec![CertificateDer::from(old_client.certificate_der)], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(old_client_key.serialize_der())), &client_id, server_id.clone(), removed.clone()).unwrap(),
        server_config(vec![CertificateDer::from(new_server.certificate_der)], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(new_server_key.serialize_der())), &server_id, client_id, removed).unwrap(),
    ).is_err());
}
