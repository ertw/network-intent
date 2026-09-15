use crate::{DeviceBinding, RevisionRef, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentRevision {
    pub version: u32,
    pub revision: RevisionRef,
    pub parent: Option<RevisionRef>,
    pub source: String,
    pub accepted_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Datastore {
    Committed,
    SessionStaging,
    ConfigurationInUse,
    Operational,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Completeness {
    Complete,
    Partial { missing: Vec<String> },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionScope {
    pub objects: Vec<String>,
    pub datastore: Datastore,
    pub session_id: Option<String>,
    pub completeness: Completeness,
}

/// The array order of sections, fields and list entries is meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSection {
    pub package: String,
    pub section: String,
    pub kind: String,
    pub fields: Vec<ConfigField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigField {
    Option { name: String, value: String },
    List { name: String, values: Vec<String> },
    Redacted { name: String },
}

/// Conservative shared boundary for values that must never enter evidence,
/// plans, history, or telemetry. Unknown credential fields stay redacted.
pub fn sensitive_field(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["password", "passwd", "secret", "key", "token", "credential"]
        .iter().any(|part| lower.contains(part))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationMetadata {
    pub version: u32,
    pub id: String,
    pub device: DeviceBinding,
    pub collector_identity: String,
    pub collected_at_ms: u64,
    pub received_at_ms: u64,
    pub interval_ms: u64,
    pub invocation_timeout_ms: u64,
    pub scope: CollectionScope,
    pub origin: String,
}

impl ObservationMetadata {
    pub fn fresh_at(&self, now_ms: u64) -> bool {
        self.version == crate::PROTOCOL_VERSION
            && self.collected_at_ms <= now_ms
            && self.received_at_ms <= now_ms
            && self.interval_ms > 0
            && self.invocation_timeout_ms > 0
            && now_ms.saturating_sub(self.collected_at_ms)
                <= self
                    .interval_ms
                    .saturating_mul(3)
                    .saturating_add(self.invocation_timeout_ms)
            && matches!(self.scope.completeness, Completeness::Complete)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationSnapshot {
    pub metadata: ObservationMetadata,
    pub sections: Vec<ConfigSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalSnapshot {
    pub metadata: ObservationMetadata,
    pub facts: Vec<OperationalFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationalFact {
    Interface {
        name: String,
        up: bool,
        device: Option<String>,
        addresses: Vec<String>,
    },
    Device {
        name: String,
        present: bool,
        carrier: Option<bool>,
        members: Vec<String>,
    },
    Route {
        interface: String,
        destination: String,
        gateway: Option<String>,
        active: bool,
    },
    Wireless {
        name: String,
        up: bool,
        pending: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ownership {
    pub version: u32,
    pub device: DeviceBinding,
    pub adopted_revision: RevisionRef,
    pub baseline_snapshot: String,
    pub fields: Vec<OwnedField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedField {
    pub package: String,
    pub section: String,
    pub field: String,
    pub source: SourceSpan,
}

/// An apply receipt records execution, never independent verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReceipt {
    pub version: u32,
    pub operation_id: String,
    pub device: DeviceBinding,
    pub revision: RevisionRef,
    pub phase: String,
    pub agent_identity: String,
    pub recorded_at_ms: u64,
}
