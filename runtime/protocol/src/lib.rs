//! Versioned wire data only. Runtime agents do not share an execution runtime.
pub mod assurance;
pub mod check;
pub mod evidence;
pub mod state;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROTOCOL_VERSION: u32 = 1;

/// Digest the exact transmitted bytes; JSON reserialization is not canonicalization.
pub fn payload_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Controller,
    Witness,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRef {
    pub id: String,
    pub source_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceBinding {
    pub device_id: String,
    pub profile_digest: String,
    pub fencing_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageEnvelope {
    pub version: u32,
    pub message_id: String,
    pub sender: String,
    pub recipient: String,
    pub device: DeviceBinding,
    pub plan_epoch: u64,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub payload_digest: String,
    pub payload: Vec<u8>,
}
