//! Mutual TLS configurations for Network Intent's URI-only SPIFFE identities.
//!
//! Rustls' stock verifier validates DNS/IP names.  These certificates carry a
//! single URI SAN, so the verifier below retains WebPKI's chain, time, EKU and
//! signature checks and adds an exact URI check; it never treats a DNS name as
//! an identity fallback.

use crate::AgentIdentity;
use intent_protocol::payload_digest;
use rustls::{
    client::danger::{ServerCertVerified, ServerCertVerifier},
    server::danger::{ClientCertVerified, ClientCertVerifier},
    ClientConfig, CertificateError, DigitallySignedStruct, Error, RootCertStore, ServerConfig,
    SignatureScheme,
};
use rustls::client::WebPkiServerVerifier;
use rustls::server::WebPkiClientVerifier;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::{collections::HashSet, fmt, sync::Arc};
use thiserror::Error;
use x509_parser::{extensions::GeneralName, prelude::parse_x509_certificate};

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("TLS identity material is malformed or inconsistent")]
    InvalidMaterial,
    #[error("the trust-root bundle is empty or invalid")]
    InvalidRoots,
    #[error("Rustls configuration failed: {0}")]
    Rustls(#[from] Error),
}

/// The controller distributes this atomically with revocation state.  A caller
/// must replace the entire policy when either field changes, rather than merge
/// untrusted data from a peer certificate.
#[derive(Clone, Debug, Default)]
pub struct TrustPolicy {
    pub roots: Vec<CertificateDer<'static>>,
    pub trust_generation: u64,
    pub revoked_certificate_digests: HashSet<String>,
}

impl TrustPolicy {
    fn roots(&self) -> Result<Arc<RootCertStore>, TlsError> {
        if self.roots.is_empty() { return Err(TlsError::InvalidRoots); }
        let mut roots = RootCertStore::empty();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        for root in &self.roots {
            let (rest, parsed) = parse_x509_certificate(root.as_ref()).map_err(|_| TlsError::InvalidRoots)?;
            if !rest.is_empty() || now < parsed.validity().not_before.timestamp() || now >= parsed.validity().not_after.timestamp() {
                return Err(TlsError::InvalidRoots);
            }
            roots.add(root.clone()).map_err(|_| TlsError::InvalidRoots)?;
        }
        Ok(Arc::new(roots))
    }
}

/// Build a client configuration that presents `identity` and accepts only the
/// peer URI requested by the caller. `server_name` is transport routing only;
/// it is never used to authenticate a SPIFFE peer.
pub fn client_config(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    identity: &AgentIdentity,
    expected_peer: AgentIdentity,
    policy: TrustPolicy,
) -> Result<Arc<ClientConfig>, TlsError> {
    require_leaf_identity(&chain, identity)?;
    let roots = policy.roots()?;
    let root_certificates = policy.roots.clone();
    let signatures = WebPkiServerVerifier::builder(roots.clone()).build().map_err(|_| TlsError::InvalidRoots)?;
    let verifier = Arc::new(UriServerVerifier { signatures, roots, root_certificates, expected: expected_peer.to_string(), revoked: policy.revoked_certificate_digests });
    let mut config = ClientConfig::builder().dangerous().with_custom_certificate_verifier(verifier).with_client_auth_cert(chain, key)?;
    // A resumed session does not repeat peer-certificate verification, so it
    // cannot be used while revocation is an in-memory policy input.
    config.resumption = rustls::client::Resumption::disabled();
    Ok(Arc::new(config))
}

/// Build a server configuration that requires a client certificate carrying
/// exactly `expected_peer` as its sole URI SAN.
pub fn server_config(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    identity: &AgentIdentity,
    expected_peer: AgentIdentity,
    policy: TrustPolicy,
) -> Result<Arc<ServerConfig>, TlsError> {
    require_leaf_identity(&chain, identity)?;
    let roots = policy.roots()?;
    let root_certificates = policy.roots.clone();
    let signatures = WebPkiClientVerifier::builder(roots.clone()).build().map_err(|_| TlsError::InvalidRoots)?;
    let verifier = Arc::new(UriClientVerifier { signatures, roots, root_certificates, expected: expected_peer.to_string(), revoked: policy.revoked_certificate_digests });
    let mut config = ServerConfig::builder().with_client_cert_verifier(verifier).with_single_cert(chain, key)?;
    config.send_tls13_tickets = 0;
    config.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
    Ok(Arc::new(config))
}

fn require_leaf_identity(chain: &[CertificateDer<'static>], expected: &AgentIdentity) -> Result<(), TlsError> {
    let leaf = chain.first().ok_or(TlsError::InvalidMaterial)?;
    if exact_uri(leaf).as_deref() != Some(&expected.to_string()) { return Err(TlsError::InvalidMaterial); }
    Ok(())
}

fn exact_uri(cert: &CertificateDer<'_>) -> Option<String> {
    let (rest, cert) = parse_x509_certificate(cert.as_ref()).ok()?;
    if !rest.is_empty() { return None; }
    let san = cert.subject_alternative_name().ok()??;
    if san.value.general_names.len() != 1 { return None; }
    match &san.value.general_names[0] { GeneralName::URI(uri) => Some(uri.to_string()), _ => None }
}

fn rejected(cert: &CertificateDer<'_>, expected: &str, revoked: &HashSet<String>) -> Result<(), Error> {
    if revoked.contains(&payload_digest(cert.as_ref())) || exact_uri(cert).as_deref() != Some(expected) {
        return Err(Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure));
    }
    Ok(())
}

struct UriServerVerifier { signatures: Arc<WebPkiServerVerifier>, roots: Arc<RootCertStore>, root_certificates: Vec<CertificateDer<'static>>, expected: String, revoked: HashSet<String> }
impl fmt::Debug for UriServerVerifier { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("UriServerVerifier").field("expected", &self.expected).finish() } }
impl ServerCertVerifier for UriServerVerifier {
    fn verify_server_cert(&self, end: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], now: UnixTime) -> Result<ServerCertVerified, Error> {
        validate_root_times(&self.root_certificates, now)?;
        let cert = webpki::EndEntityCert::try_from(end).map_err(|_| Error::InvalidCertificate(CertificateError::BadEncoding))?;
        cert.verify_for_usage(webpki::ALL_VERIFICATION_ALGS, &self.roots.roots, intermediates, now, webpki::KeyUsage::server_auth(), None, None).map_err(|_| Error::InvalidCertificate(CertificateError::UnknownIssuer))?;
        rejected(end, &self.expected, &self.revoked)?;
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> { self.signatures.verify_tls12_signature(m,c,d) }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> { self.signatures.verify_tls13_signature(m,c,d) }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.signatures.supported_verify_schemes() }
}

struct UriClientVerifier { signatures: Arc<dyn ClientCertVerifier>, roots: Arc<RootCertStore>, root_certificates: Vec<CertificateDer<'static>>, expected: String, revoked: HashSet<String> }
impl fmt::Debug for UriClientVerifier { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("UriClientVerifier").field("expected", &self.expected).finish() } }
impl ClientCertVerifier for UriClientVerifier {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] { self.signatures.root_hint_subjects() }
    fn verify_client_cert(&self, end: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], now: UnixTime) -> Result<ClientCertVerified, Error> {
        validate_root_times(&self.root_certificates, now)?;
        let cert = webpki::EndEntityCert::try_from(end).map_err(|_| Error::InvalidCertificate(CertificateError::BadEncoding))?;
        cert.verify_for_usage(webpki::ALL_VERIFICATION_ALGS, &self.roots.roots, intermediates, now, webpki::KeyUsage::client_auth(), None, None).map_err(|_| Error::InvalidCertificate(CertificateError::UnknownIssuer))?;
        rejected(end, &self.expected, &self.revoked)?;
        Ok(ClientCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> { self.signatures.verify_tls12_signature(m,c,d) }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> { self.signatures.verify_tls13_signature(m,c,d) }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.signatures.supported_verify_schemes() }
}

// WebPKI treats configured roots as trust anchors and does not apply their
// certificate validity period. Recheck at each handshake: configs can outlive
// the root that was valid when they were constructed.
fn validate_root_times(roots: &[CertificateDer<'_>], now: UnixTime) -> Result<(), Error> {
    let now = i64::try_from(now.as_secs()).map_err(|_| Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure))?;
    for root in roots {
        let (rest, cert) = parse_x509_certificate(root.as_ref()).map_err(|_| Error::InvalidCertificate(CertificateError::BadEncoding))?;
        if !rest.is_empty() || now < cert.validity().not_before.timestamp() || now >= cert.validity().not_after.timestamp() {
            return Err(Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_expiration_is_checked_after_configuration_creation() {
        let key = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.not_before = time::OffsetDateTime::from_unix_timestamp(1_000).unwrap();
        params.not_after = time::OffsetDateTime::from_unix_timestamp(2_000).unwrap();
        let cert = params.self_signed(&key).unwrap();
        let roots = [cert.der().clone()];
        let at = |s| UnixTime::since_unix_epoch(std::time::Duration::from_secs(s));
        assert!(validate_root_times(&roots, at(1_500)).is_ok());
        assert!(validate_root_times(&roots, at(2_000)).is_err());
        assert!(validate_root_times(&roots, at(999)).is_err());
    }
}
