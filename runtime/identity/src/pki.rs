//! Controller-local certificate authority and enrollment registry.
//! This module issues credentials; it does not implement a TLS transport.

use crate::{AgentIdentity, TrustedKey};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use intent_protocol::payload_digest;
use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequestParams, DnType,
    ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair, KeyUsagePurpose, PublicKeyData, SanType,
    PKCS_ED25519,
};
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use rustls_pki_types::{CertificateDer, CertificateSigningRequestDer};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{fs, fs::OpenOptions, path::Path, sync::Mutex};
use thiserror::Error;
use time::OffsetDateTime;
use x509_parser::{extensions::GeneralName, prelude::{parse_x509_certificate, FromDer}};

pub const MAX_LEAF_LIFETIME_MS: u64 = 24 * 60 * 60 * 1_000;
const ROOT_LIFETIME_MS: u64 = 365 * 24 * 60 * 60 * 1_000;
const MAX_CSR_BYTES: usize = 64 * 1024;
const MAX_TOKEN_BYTES: usize = 128;
const MAX_PROOF_BYTES: usize = 128;

#[derive(Debug, Error)]
pub enum PkiError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("certificate or CSR is malformed, unsupported, or has an invalid signature")]
    InvalidCsr,
    #[error("certificate is malformed or does not meet Network Intent constraints")]
    InvalidCertificate,
    #[error("enrollment ticket is invalid, expired, or already used")]
    InvalidTicket,
    #[error("certificate is expired, revoked, or unknown")]
    InvalidCredential,
    #[error("certificate lifetime must be between one second and 24 hours")]
    InvalidLifetime,
    #[error("root generation must increase monotonically")]
    InvalidGeneration,
    #[error("identity or root trust domain does not match this PKI authority")]
    TrustDomainMismatch,
    #[error("a trust root cannot be removed while it is the only active root")]
    LastRoot,
    #[error("randomness or X.509 issuance failed")]
    Crypto,
    #[error("PKI mutex poisoned")]
    Poisoned,
}

pub type Result<T> = std::result::Result<T, PkiError>;

/// A bearer enrollment token. It intentionally has no Debug or serialization
/// implementation; the SQLite registry stores only its SHA-256 digest.
pub struct EnrollmentTicket {
    token: String,
}

impl EnrollmentTicket {
    pub fn expose_to_enrollee(&self) -> &str {
        &self.token
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuedCertificate {
    pub certificate_der: Vec<u8>,
    pub root_certificate_der: Vec<u8>,
    pub identity: AgentIdentity,
    pub not_before_ms: u64,
    pub not_after_ms: u64,
    pub root_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustRoot {
    pub generation: u64,
    pub certificate_der: Vec<u8>,
}

pub struct PkiStore {
    connection: Mutex<Connection>,
}

impl PkiStore {
    /// Opens the controller's dedicated PKI database, creating its first root
    /// only when there is no existing trust state.
    pub fn open(path: impl AsRef<Path>, trust_domain: &str, now_ms: u64) -> Result<Self> {
        let path = path.as_ref();
        if path.to_string_lossy() == ":memory:" || path.to_string_lossy().starts_with("file:") {
            return Err(PkiError::Crypto);
        }
        // SQLite follows symlinks. A controller private key database must never
        // be opened through one, nor be created with the process umask's mode.
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(PkiError::Crypto)
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                options.mode(0o600);
                match options.open(path) {
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(_) => return Err(PkiError::Crypto),
                }
            }
            Err(_) => return Err(PkiError::Crypto),
        }
        AgentIdentity::parse(&format!(
            "spiffe://{trust_domain}/agent/controller/bootstrap/bootstrap"
        ))
        .map_err(|_| PkiError::TrustDomainMismatch)?;
        let connection = Connection::open(path)?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| PkiError::Crypto)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        #[cfg(unix)]
        for sidecar in [path.with_extension("sqlite-wal"), path.with_extension("sqlite-shm")] {
            if sidecar.exists() {
                fs::set_permissions(sidecar, fs::Permissions::from_mode(0o600))
                    .map_err(|_| PkiError::Crypto)?;
            }
        }
        let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 1 {
            return Err(PkiError::Crypto);
        }
        connection.execute_batch("BEGIN;
            CREATE TABLE IF NOT EXISTS roots (generation INTEGER PRIMARY KEY, certificate_der BLOB NOT NULL, key_der BLOB NOT NULL, not_before_ms INTEGER NOT NULL, not_after_ms INTEGER NOT NULL, active INTEGER NOT NULL CHECK(active IN (0, 1)));
            CREATE TABLE IF NOT EXISTS authority (id INTEGER PRIMARY KEY CHECK(id = 1), trust_domain TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS tickets (token_digest TEXT PRIMARY KEY, identity TEXT NOT NULL, csr_key_digest TEXT NOT NULL, expires_at_ms INTEGER NOT NULL, used_at_ms INTEGER);
            CREATE TABLE IF NOT EXISTS credentials (certificate_digest TEXT PRIMARY KEY, certificate_der BLOB NOT NULL, identity TEXT NOT NULL, key_der BLOB NOT NULL, key_raw BLOB NOT NULL, not_before_ms INTEGER NOT NULL, not_after_ms INTEGER NOT NULL, root_generation INTEGER NOT NULL REFERENCES roots(generation), revoked_at_ms INTEGER);
            CREATE TABLE IF NOT EXISTS renewal_proofs (proof_digest TEXT PRIMARY KEY, certificate_digest TEXT NOT NULL, csr_digest TEXT NOT NULL, consumed_at_ms INTEGER NOT NULL);
            PRAGMA user_version = 1;
            COMMIT;")?;
        let store = Self {
            connection: Mutex::new(connection),
        };
        let existing_domain: Option<String> = store
            .connection
            .lock()
            .map_err(|_| PkiError::Poisoned)?
            .query_row("SELECT trust_domain FROM authority WHERE id = 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        match existing_domain {
            Some(domain) if domain != trust_domain => return Err(PkiError::TrustDomainMismatch),
            Some(_) => {}
            None => {
                store
                    .connection
                    .lock()
                    .map_err(|_| PkiError::Poisoned)?
                    .execute(
                        "INSERT INTO authority(id, trust_domain) VALUES (1, ?1)",
                        [trust_domain],
                    )?;
            }
        }
        if store.trust_roots()?.is_empty() {
            store.insert_root(1, trust_domain, now_ms)?;
        }
        Ok(store)
    }

    pub fn create_enrollment_ticket(
        &self,
        identity: AgentIdentity,
        csr_der: &[u8],
        expires_at_ms: u64,
        now_ms: u64,
    ) -> Result<EnrollmentTicket> {
        if now_ms >= expires_at_ms {
            return Err(PkiError::InvalidTicket);
        }
        if identity.trust_domain() != self.trust_domain()? {
            return Err(PkiError::TrustDomainMismatch);
        }
        let csr = parse_ed25519_csr(csr_der)?;
        let mut bytes = [0_u8; 32];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| PkiError::Crypto)?;
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let digest = payload_digest(token.as_bytes());
        let connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        connection.execute(
            "INSERT INTO tickets(token_digest, identity, csr_key_digest, expires_at_ms, used_at_ms) VALUES (?1, ?2, ?3, ?4, NULL)",
            params![digest, identity.to_string(), payload_digest(csr.public_key.der_bytes()), expires_at_ms],
        )?;
        Ok(EnrollmentTicket { token })
    }

    /// Verifies CSR proof-of-possession, then issues only the ticket's identity.
    /// CSR SANs, usages and subject are intentionally ignored.
    pub fn enroll(
        &self,
        token: &str,
        csr_der: &[u8],
        lifetime_ms: u64,
        now_ms: u64,
    ) -> Result<IssuedCertificate> {
        if token.len() > MAX_TOKEN_BYTES {
            return Err(PkiError::InvalidTicket);
        }
        validate_lifetime(lifetime_ms)?;
        let csr = parse_ed25519_csr(csr_der)?;
        let digest = payload_digest(token.as_bytes());
        let key_digest = payload_digest(csr.public_key.der_bytes());
        let mut connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ticket: Option<(String, String, u64, Option<u64>)> = tx.query_row(
            "SELECT identity, csr_key_digest, expires_at_ms, used_at_ms FROM tickets WHERE token_digest = ?1",
            [digest.clone()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).optional()?;
        let Some((identity, expected_key, expires, used)) = ticket else {
            return Err(PkiError::InvalidTicket);
        };
        if used.is_some() || now_ms >= expires || expected_key != key_digest {
            return Err(PkiError::InvalidTicket);
        }
        let identity = AgentIdentity::parse(&identity).map_err(|_| PkiError::InvalidTicket)?;
        let issued = issue_with_current_root(&tx, &identity, &csr, lifetime_ms, now_ms)?;
        let changed = tx.execute(
            "UPDATE tickets SET used_at_ms = ?2 WHERE token_digest = ?1 AND used_at_ms IS NULL",
            params![digest, now_ms],
        )?;
        if changed != 1 {
            return Err(PkiError::InvalidTicket);
        }
        insert_credential(
            &tx,
            &issued,
            csr.public_key.der_bytes(),
            &csr.public_key.subject_public_key_info(),
            now_ms,
        )?;
        tx.commit()?;
        Ok(issued)
    }

    /// Renews a credential after a signature by its currently valid certificate
    /// key over `renewal_message(fresh_csr_der)`. The fresh CSR may roll keys.
    pub fn renew(
        &self,
        current_cert_der: &[u8],
        renewal_proof: &[u8],
        fresh_csr_der: &[u8],
        lifetime_ms: u64,
        now_ms: u64,
    ) -> Result<IssuedCertificate> {
        if renewal_proof.len() > MAX_PROOF_BYTES {
            return Err(PkiError::InvalidCredential);
        }
        validate_lifetime(lifetime_ms)?;
        let csr = parse_ed25519_csr(fresh_csr_der)?;
        let cert_digest = payload_digest(current_cert_der);
        let mut connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let credential: Option<(String, Vec<u8>, u64, u64, Option<u64>)> = tx.query_row(
            "SELECT c.identity, c.key_raw, c.not_before_ms, c.not_after_ms, c.revoked_at_ms FROM credentials c JOIN roots r ON r.generation = c.root_generation WHERE c.certificate_digest = ?1 AND c.certificate_der = ?2 AND r.active = 1",
            params![cert_digest, current_cert_der], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        ).optional()?;
        let Some((identity, key_raw, not_before, not_after, revoked)) = credential else {
            return Err(PkiError::InvalidCredential);
        };
        if revoked.is_some() || now_ms < not_before || now_ms >= not_after || key_raw.len() != 32 {
            return Err(PkiError::InvalidCredential);
        }
        signature::UnparsedPublicKey::new(&signature::ED25519, &key_raw)
            .verify(&renewal_message(fresh_csr_der), renewal_proof)
            .map_err(|_| PkiError::InvalidCredential)?;
        if tx.execute("INSERT INTO renewal_proofs(proof_digest, certificate_digest, csr_digest, consumed_at_ms) VALUES (?1, ?2, ?3, ?4)", params![payload_digest(renewal_proof), cert_digest, payload_digest(fresh_csr_der), now_ms]).is_err() { return Err(PkiError::InvalidCredential); }
        let identity = AgentIdentity::parse(&identity).map_err(|_| PkiError::InvalidCredential)?;
        let issued = issue_with_current_root(&tx, &identity, &csr, lifetime_ms, now_ms)?;
        insert_credential(
            &tx,
            &issued,
            csr.public_key.der_bytes(),
            &csr.public_key.subject_public_key_info(),
            now_ms,
        )?;
        tx.commit()?;
        Ok(issued)
    }

    pub fn revoke(&self, certificate_der: &[u8], now_ms: u64) -> Result<bool> {
        let connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        Ok(connection.execute("UPDATE credentials SET revoked_at_ms = ?2 WHERE certificate_digest = ?1 AND certificate_der = ?3 AND revoked_at_ms IS NULL", params![payload_digest(certificate_der), now_ms, certificate_der])? != 0)
    }

    /// Produces a DSSE trust record only for an exact, active-root certificate
    /// currently registered by this authority.
    pub fn trusted_key(&self, certificate_der: &[u8], now_ms: u64) -> Result<TrustedKey> {
        let connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        let record: Option<(String, Vec<u8>, u64, u64, Option<u64>)> = connection.query_row(
            "SELECT c.identity, c.key_raw, c.not_before_ms, c.not_after_ms, c.revoked_at_ms FROM credentials c JOIN roots r ON r.generation = c.root_generation WHERE c.certificate_digest = ?1 AND c.certificate_der = ?2 AND r.active = 1",
            params![payload_digest(certificate_der), certificate_der], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        ).optional()?;
        let Some((identity, key, not_before, not_after, revoked)) = record else {
            return Err(PkiError::InvalidCredential);
        };
        if revoked.is_some() || now_ms < not_before || now_ms >= not_after {
            return Err(PkiError::InvalidCredential);
        }
        Ok(TrustedKey {
            identity: AgentIdentity::parse(&identity).map_err(|_| PkiError::InvalidCredential)?,
            public_key: key,
            not_before_ms: not_before,
            not_after_ms: not_after,
            revoked: false,
        })
    }

    pub fn rotate_root(&self, generation: u64, trust_domain: &str, now_ms: u64) -> Result<()> {
        if trust_domain != self.trust_domain()? {
            return Err(PkiError::TrustDomainMismatch);
        }
        self.insert_root(generation, trust_domain, now_ms)
    }

    pub fn remove_trust_root(&self, generation: u64) -> Result<bool> {
        let mut connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: u64 =
            tx.query_row("SELECT count(*) FROM roots WHERE active = 1", [], |r| {
                r.get(0)
            })?;
        let target_active: Option<u64> = tx
            .query_row(
                "SELECT active FROM roots WHERE generation = ?1",
                [generation],
                |r| r.get(0),
            )
            .optional()?;
        if target_active == Some(1) && active <= 1 {
            return Err(PkiError::LastRoot);
        }
        let removed = tx.execute(
            "UPDATE roots SET active = 0 WHERE generation = ?1 AND active = 1",
            [generation],
        )? != 0;
        tx.commit()?;
        Ok(removed)
    }

    pub fn trust_roots(&self) -> Result<Vec<TrustRoot>> {
        let connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        let mut statement = connection.prepare(
            "SELECT generation, certificate_der FROM roots WHERE active = 1 ORDER BY generation",
        )?;
        let roots = statement
            .query_map([], |r| {
                Ok(TrustRoot {
                    generation: r.get(0)?,
                    certificate_der: r.get(1)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(roots)
    }

    fn insert_root(&self, generation: u64, trust_domain: &str, now_ms: u64) -> Result<()> {
        let mut params = CertificateParams::default();
        params.distinguished_name.push(
            DnType::CommonName,
            format!("Network Intent Root {trust_domain}"),
        );
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let not_before_ms = normalize_ms(now_ms)?;
        params.not_before = as_time(not_before_ms)?;
        params.not_after = as_time(
            not_before_ms
                .checked_add(ROOT_LIFETIME_MS)
                .ok_or(PkiError::Crypto)?,
        )?;
        let key = KeyPair::generate_for(&PKCS_ED25519).map_err(|_| PkiError::Crypto)?;
        let cert = params.self_signed(&key).map_err(|_| PkiError::Crypto)?;
        let mut connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let highest: u64 = tx.query_row("SELECT COALESCE(MAX(generation), 0) FROM roots", [], |r| r.get(0))?;
        if generation <= highest {
            return Err(PkiError::InvalidGeneration);
        }
        tx.execute("INSERT INTO roots(generation, certificate_der, key_der, not_before_ms, not_after_ms, active) VALUES (?1, ?2, ?3, ?4, ?5, 1)", params![generation, cert.der().as_ref(), key.serialize_der(), not_before_ms, not_before_ms + ROOT_LIFETIME_MS])?;
        tx.commit()?;
        Ok(())
    }

    fn trust_domain(&self) -> Result<String> {
        let connection = self.connection.lock().map_err(|_| PkiError::Poisoned)?;
        connection
            .query_row("SELECT trust_domain FROM authority WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(PkiError::from)
    }
}

/// Exact bytes covered by a renewal proof, with an unambiguous prefix.
pub fn renewal_message(fresh_csr_der: &[u8]) -> Vec<u8> {
    let mut message = format!("network-intent-renew-v1 {} ", fresh_csr_der.len()).into_bytes();
    message.extend_from_slice(fresh_csr_der);
    message
}

fn validate_lifetime(lifetime_ms: u64) -> Result<()> {
    if lifetime_ms < 1_000 || lifetime_ms > MAX_LEAF_LIFETIME_MS {
        Err(PkiError::InvalidLifetime)
    } else {
        Ok(())
    }
}

fn as_time(milliseconds: u64) -> Result<OffsetDateTime> {
    let seconds = i64::try_from(milliseconds / 1_000).map_err(|_| PkiError::Crypto)?;
    OffsetDateTime::from_unix_timestamp(seconds).map_err(|_| PkiError::Crypto)
}

/// X.509 validity is DER-encoded at whole-second precision. Persist and return
/// that exact interval rather than claiming sub-second validity we did not issue.
fn normalize_ms(milliseconds: u64) -> Result<u64> {
    milliseconds
        .checked_div(1_000)
        .and_then(|seconds| seconds.checked_mul(1_000))
        .ok_or(PkiError::Crypto)
}

fn parse_ed25519_csr(csr_der: &[u8]) -> Result<CertificateSigningRequestParams> {
    if csr_der.is_empty() || csr_der.len() > MAX_CSR_BYTES {
        return Err(PkiError::InvalidCsr);
    }
    let der = CertificateSigningRequestDer::from(csr_der.to_vec());
    let csr = CertificateSigningRequestParams::from_der(&der).map_err(|_| PkiError::InvalidCsr)?;
    let (remaining, _) = x509_parser::certification_request::X509CertificationRequest::from_der(csr_der)
        .map_err(|_| PkiError::InvalidCsr)?;
    if !remaining.is_empty() {
        return Err(PkiError::InvalidCsr);
    }
    if csr.public_key.algorithm() != &PKCS_ED25519 {
        return Err(PkiError::InvalidCsr);
    }
    Ok(csr)
}

fn issue_with_current_root(
    tx: &rusqlite::Transaction<'_>,
    identity: &AgentIdentity,
    csr: &CertificateSigningRequestParams,
    lifetime_ms: u64,
    now_ms: u64,
) -> Result<IssuedCertificate> {
    let not_before_ms = normalize_ms(now_ms)?;
    let lifetime_ms = normalize_ms(lifetime_ms)?;
    if lifetime_ms < 1_000 {
        return Err(PkiError::InvalidLifetime);
    }
    let (generation, root_der, root_key, root_after): (u64, Vec<u8>, Vec<u8>, u64) = tx.query_row(
        "SELECT generation, certificate_der, key_der, not_after_ms FROM roots WHERE active = 1 AND not_before_ms <= ?1 AND ?1 < not_after_ms ORDER BY generation DESC LIMIT 1", [not_before_ms],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let key = KeyPair::try_from(root_key.as_slice()).map_err(|_| PkiError::Crypto)?;
    let root = CertificateDer::from(root_der.clone());
    let issuer = Issuer::from_ca_cert_der(&root, key).map_err(|_| PkiError::Crypto)?;
    let mut params = CertificateParams::default();
    params.subject_alt_names = vec![SanType::URI(
        identity
            .to_string()
            .try_into()
            .map_err(|_| PkiError::Crypto)?,
    )];
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth,
    ];
    params.not_before = as_time(not_before_ms)?;
    let not_after = not_before_ms
        .checked_add(lifetime_ms)
        .ok_or(PkiError::InvalidLifetime)?;
    if not_after > root_after {
        return Err(PkiError::InvalidLifetime);
    }
    params.not_after = as_time(not_after)?;
    let cert = params
        .signed_by(&csr.public_key, &issuer)
        .map_err(|_| PkiError::Crypto)?;
    let certificate_der = cert.der().as_ref().to_vec();
    validate_issued_certificate(&certificate_der, identity, not_before_ms, not_after)?;
    Ok(IssuedCertificate {
        certificate_der,
        root_certificate_der: root_der,
        identity: identity.clone(),
        not_before_ms,
        not_after_ms: not_after,
        root_generation: generation,
    })
}

fn insert_credential(
    tx: &rusqlite::Transaction<'_>,
    issued: &IssuedCertificate,
    key_der: &[u8],
    key_spki: &[u8],
    _now_ms: u64,
) -> Result<()> {
    let (_, cert) = parse_x509_certificate(&issued.certificate_der)
        .map_err(|_| PkiError::InvalidCertificate)?;
    let key_raw = cert.public_key().subject_public_key.data.to_vec();
    if key_raw.len() != 32 {
        return Err(PkiError::InvalidCertificate);
    }
    // The public SPKI from the CSR and issued certificate must be identical.
    if cert.public_key().raw != key_spki {
        return Err(PkiError::InvalidCertificate);
    }
    tx.execute("INSERT INTO credentials(certificate_digest, certificate_der, identity, key_der, key_raw, not_before_ms, not_after_ms, root_generation, revoked_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
        params![payload_digest(&issued.certificate_der), issued.certificate_der, issued.identity.to_string(), key_der, key_raw, issued.not_before_ms, issued.not_after_ms, issued.root_generation])?;
    Ok(())
}

fn validate_issued_certificate(
    cert_der: &[u8],
    identity: &AgentIdentity,
    not_before_ms: u64,
    not_after_ms: u64,
) -> Result<()> {
    let (remaining, cert) =
        parse_x509_certificate(cert_der).map_err(|_| PkiError::InvalidCertificate)?;
    if !remaining.is_empty() {
        return Err(PkiError::InvalidCertificate);
    }
    let san = cert
        .subject_alternative_name()
        .map_err(|_| PkiError::InvalidCertificate)?
        .ok_or(PkiError::InvalidCertificate)?;
    if san.value.general_names.len() != 1
        || !matches!(&san.value.general_names[0], GeneralName::URI(uri) if *uri == identity.to_string())
    {
        return Err(PkiError::InvalidCertificate);
    }
    let usage = cert
        .key_usage()
        .map_err(|_| PkiError::InvalidCertificate)?
        .ok_or(PkiError::InvalidCertificate)?;
    if !usage.value.digital_signature() {
        return Err(PkiError::InvalidCertificate);
    }
    let eku = cert
        .extended_key_usage()
        .map_err(|_| PkiError::InvalidCertificate)?
        .ok_or(PkiError::InvalidCertificate)?;
    if !eku.value.client_auth || !eku.value.server_auth {
        return Err(PkiError::InvalidCertificate);
    }
    let expected_before = as_time(not_before_ms)?.unix_timestamp();
    let expected_after = as_time(not_after_ms)?.unix_timestamp();
    if cert.validity().not_before.timestamp() != expected_before
        || cert.validity().not_after.timestamp() != expected_after
    {
        return Err(PkiError::InvalidCertificate);
    }
    Ok(())
}
