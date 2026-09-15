use crate::{
    assurance::{ProbeResult, ProbeSpec},
    state::Completeness,
    DeviceBinding, RevisionRef,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
pub const WITNESS_ASSIGNMENT_TYPE: &str = "urn:network-intent:assignment:witness:v1";
pub const WITNESS_EVIDENCE_TYPE: &str = "urn:network-intent:evidence:witness:v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement<T> {
    #[serde(rename = "_type")]
    pub statement_type: String,
    pub subject: Vec<Subject>,
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    pub predicate: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub name: String,
    pub digest: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessAssignment {
    pub version: u32,
    pub assignment_id: String,
    pub issuer: String,
    pub recipient: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub plan_id: String,
    pub plan_epoch: u64,
    pub graph_version: u64,
    pub revision: RevisionRef,
    pub device: DeviceBinding,
    pub probes: Vec<ProbeSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessEvidence {
    pub version: u32,
    pub evidence_id: String,
    pub assignment_id: String,
    pub plan_id: String,
    pub plan_epoch: u64,
    pub graph_version: u64,
    pub revision: RevisionRef,
    pub device: DeviceBinding,
    pub witness: String,
    pub probe_digest: String,
    pub completeness: Completeness,
    pub result: ProbeResult,
}

pub fn revision_subject(revision: &RevisionRef) -> Vec<Subject> {
    vec![Subject {
        name: revision.id.clone(),
        digest: BTreeMap::from([("sha256".into(), revision.source_digest.clone())]),
    }]
}
