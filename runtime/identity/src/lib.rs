//! Identity parsing and DSSE cryptography. No agent lifecycle or execution code.
//! Trusted keys must come from authenticated enrollment/certificate validation;
//! a key or identity supplied in an incoming message is never a trust anchor.
use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use intent_protocol::{payload_digest, AgentRole};
use ring::{
    rand::SystemRandom,
    signature::{self, Ed25519KeyPair, KeyPair},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
pub mod evidence;
pub mod pki;
pub mod tls;

pub const DSSE_PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";
pub const MAX_PAYLOAD_BYTES: usize = 1_048_576;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("identity must be a canonical Network Intent SPIFFE ID")]
    InvalidIdentity,
    #[error("invalid signing key or random source unavailable")]
    Key,
    #[error("unsupported or oversized DSSE envelope")]
    Envelope,
    #[error("no valid signature by the required enrolled identity and role")]
    Signature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentIdentity {
    trust_domain: String,
    role: AgentRole,
    device: String,
    instance: String,
}

impl AgentIdentity {
    pub fn parse(id: &str) -> Result<Self, IdentityError> {
        if id.len() > 2048 {
            return Err(IdentityError::InvalidIdentity);
        }
        let path: Vec<_> = id
            .strip_prefix("spiffe://")
            .ok_or(IdentityError::InvalidIdentity)?
            .split('/')
            .collect();
        if path.len() != 5
            || path[1] != "agent"
            || !valid_domain(path[0])
            || !valid_segment(path[3])
            || !valid_segment(path[4])
        {
            return Err(IdentityError::InvalidIdentity);
        }
        let role = match path[2] {
            "controller" => AgentRole::Controller,
            "witness" => AgentRole::Witness,
            "apply" => AgentRole::Apply,
            _ => return Err(IdentityError::InvalidIdentity),
        };
        Ok(Self {
            trust_domain: path[0].into(),
            role,
            device: path[3].into(),
            instance: path[4].into(),
        })
    }
    pub fn role(&self) -> AgentRole {
        self.role
    }
    pub fn device(&self) -> &str {
        &self.device
    }
    pub fn trust_domain(&self) -> &str {
        &self.trust_domain
    }
}

impl fmt::Display for AgentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let role = match self.role {
            AgentRole::Controller => "controller",
            AgentRole::Witness => "witness",
            AgentRole::Apply => "apply",
        };
        write!(
            f,
            "spiffe://{}/agent/{}/{}/{}",
            self.trust_domain, role, self.device, self.instance
        )
    }
}

fn valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn valid_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// Intentionally not Debug or Serialize. Private key bytes are never telemetry.
pub struct SigningKey(Ed25519KeyPair);
impl SigningKey {
    /// Returned PKCS#8 must be stored in the agent's private, permissioned store.
    pub fn generate() -> Result<(Self, Vec<u8>), IdentityError> {
        let document =
            Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).map_err(|_| IdentityError::Key)?;
        let encoded = document.as_ref().to_vec();
        Ok((Self::from_pkcs8(&encoded)?, encoded))
    }
    pub fn from_pkcs8(bytes: &[u8]) -> Result<Self, IdentityError> {
        Ok(Self(
            Ed25519KeyPair::from_pkcs8(bytes).map_err(|_| IdentityError::Key)?,
        ))
    }
    pub fn public_key(&self) -> Vec<u8> {
        self.0.public_key().as_ref().to_vec()
    }
    pub fn sign(&self, payload: &[u8]) -> Result<DsseEnvelope, IdentityError> {
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(IdentityError::Envelope);
        }
        let signature = self.0.sign(&pae(DSSE_PAYLOAD_TYPE.as_bytes(), payload));
        Ok(DsseEnvelope {
            payload_type: DSSE_PAYLOAD_TYPE.into(),
            payload: STANDARD.encode(payload),
            signatures: vec![DsseSignature {
                keyid: payload_digest(self.0.public_key().as_ref()),
                sig: STANDARD.encode(signature.as_ref()),
            }],
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DsseEnvelope {
    #[serde(rename = "payloadType")]
    pub payload_type: String,
    pub payload: String,
    pub signatures: Vec<DsseSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DsseSignature {
    #[serde(default)]
    pub keyid: String,
    pub sig: String,
}

/// A controller-owned enrollment/certificate record. Not deserializable from
/// incoming envelopes. Revocation and validity are checked on every use.
#[derive(Clone)]
pub struct TrustedKey {
    pub identity: AgentIdentity,
    pub public_key: Vec<u8>,
    pub not_before_ms: u64,
    pub not_after_ms: u64,
    pub revoked: bool,
}

/// Constructible only after signature verification. Application parsing consumes
/// these exact verified bytes; it must not decode the input envelope again.
pub struct AuthenticatedPayload {
    bytes: Vec<u8>,
    signer: AgentIdentity,
    key_digest: String,
}
impl AuthenticatedPayload {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn signer(&self) -> &AgentIdentity {
        &self.signer
    }
    pub fn key_digest(&self) -> &str {
        &self.key_digest
    }
}

pub fn verify(
    envelope: &DsseEnvelope,
    trusted: &[TrustedKey],
    expected_identity: &AgentIdentity,
    expected_role: AgentRole,
    now_ms: u64,
) -> Result<AuthenticatedPayload, IdentityError> {
    if envelope.payload_type != DSSE_PAYLOAD_TYPE
        || envelope.payload.len() > (MAX_PAYLOAD_BYTES + 2) / 3 * 4
        || envelope.signatures.is_empty()
        || envelope.signatures.len() > 8
        || expected_identity.role() != expected_role
    {
        return Err(IdentityError::Envelope);
    }
    let bytes = decode(&envelope.payload)?;
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(IdentityError::Envelope);
    }
    let message = pae(envelope.payload_type.as_bytes(), &bytes);
    for signed in &envelope.signatures {
        if signed.sig.len() > 128 || signed.keyid.len() > 256 {
            return Err(IdentityError::Envelope);
        }
        let signature = decode(&signed.sig)?;
        for key in trusted {
            if key.identity != *expected_identity
                || key.identity.role() != expected_role
                || key.revoked
                || key.not_before_ms > now_ms
                || now_ms >= key.not_after_ms
                || key.public_key.len() != 32
            {
                continue;
            }
            // keyid is an unauthenticated hint, never authority or signed identity.
            if signature::UnparsedPublicKey::new(&signature::ED25519, &key.public_key)
                .verify(&message, &signature)
                .is_ok()
            {
                return Ok(AuthenticatedPayload {
                    bytes,
                    signer: key.identity.clone(),
                    key_digest: payload_digest(&key.public_key),
                });
            }
        }
    }
    Err(IdentityError::Signature)
}

fn decode(encoded: &str) -> Result<Vec<u8>, IdentityError> {
    STANDARD
        .decode(encoded)
        .or_else(|_| STANDARD_NO_PAD.decode(encoded))
        .or_else(|_| URL_SAFE.decode(encoded))
        .or_else(|_| URL_SAFE_NO_PAD.decode(encoded))
        .map_err(|_| IdentityError::Envelope)
}

/// DSSE v1 PAE: lengths are decimal UTF-8 byte lengths, not character counts.
pub fn pae(payload_type: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut bytes = format!("DSSEv1 {} ", payload_type.len()).into_bytes();
    bytes.extend_from_slice(payload_type);
    bytes.extend_from_slice(format!(" {} ", payload.len()).as_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity(role: &str) -> AgentIdentity {
        AgentIdentity::parse(&format!("spiffe://lab.local/agent/{role}/router-a/agent-1")).unwrap()
    }
    fn fixture(role: &str) -> (SigningKey, TrustedKey) {
        let (key, _) = SigningKey::generate().unwrap();
        let trusted = TrustedKey {
            identity: identity(role),
            public_key: key.public_key(),
            not_before_ms: 10,
            not_after_ms: 100,
            revoked: false,
        };
        (key, trusted)
    }
    #[test]
    fn dsse_reference_pae_and_unicode_bytes() {
        assert_eq!(
            pae(b"http://example.com/HelloWorld", b"hello world"),
            b"DSSEv1 29 http://example.com/HelloWorld 11 hello world"
        );
        assert_eq!(
            pae("é".as_bytes(), "✓".as_bytes()),
            "DSSEv1 2 é 3 ✓".as_bytes()
        );
    }
    #[test]
    fn canonical_identity_separates_agents() {
        assert_ne!(identity("witness"), identity("apply"));
        for bad in [
            "spiffe://lab.local/agent/witness/router/../apply",
            "spiffe://LAB.local/agent/witness/router/one",
            "spiffe://lab.local:80/agent/witness/router/one",
            "spiffe://lab.local/agent/witness/router/one?admin=true",
            "spiffe://lab.local/agent/witness/router/%61",
            "spiffe://lab..local/agent/witness/router/one",
        ] {
            assert!(AgentIdentity::parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn exact_payload_signature_and_rotation() {
        let (key, old) = fixture("witness");
        let (_, new) = fixture("witness");
        let env = key.sign(b"{\"observation\":42}").unwrap();
        let verified = verify(
            &env,
            &[new.clone(), old.clone()],
            &identity("witness"),
            AgentRole::Witness,
            20,
        )
        .unwrap();
        assert_eq!(verified.bytes(), b"{\"observation\":42}");
        assert!(verify(&env, &[new], &identity("witness"), AgentRole::Witness, 20).is_err());
        let mut changed = env.clone();
        changed.payload = STANDARD.encode(b"{\"observation\":43}");
        assert!(verify(
            &changed,
            &[old],
            &identity("witness"),
            AgentRole::Witness,
            20
        )
        .is_err());
    }
    #[test]
    fn enforces_role_revocation_validity_and_payload_type() {
        let (key, mut trusted) = fixture("apply");
        let mut env = key.sign(b"payload").unwrap();
        assert!(verify(
            &env,
            &[trusted.clone()],
            &identity("witness"),
            AgentRole::Witness,
            20
        )
        .is_err());
        for now in [9, 100, 101] {
            assert!(verify(
                &env,
                &[trusted.clone()],
                &identity("apply"),
                AgentRole::Apply,
                now
            )
            .is_err());
        }
        trusted.revoked = true;
        assert!(verify(
            &env,
            &[trusted.clone()],
            &identity("apply"),
            AgentRole::Apply,
            20
        )
        .is_err());
        trusted.revoked = false;
        env.payload_type = "application/json".into();
        assert!(verify(&env, &[trusted], &identity("apply"), AgentRole::Apply, 20).is_err());
    }
    #[test]
    fn key_hint_is_not_authority_and_urlsafe_base64_is_accepted() {
        let (key, trusted) = fixture("witness");
        let mut env = key.sign(&[255, 254, 253]).unwrap();
        env.payload = URL_SAFE_NO_PAD.encode([255, 254, 253]);
        env.signatures[0].keyid = "untrusted hint".into();
        env.signatures[0].sig = URL_SAFE.encode(decode(&env.signatures[0].sig).unwrap());
        assert!(verify(
            &env,
            &[trusted],
            &identity("witness"),
            AgentRole::Witness,
            20
        )
        .is_ok());
    }
}
