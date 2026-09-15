//! Verify signatures first, then bind the exact parsed statement to the local
//! authorized assignment. Freshness is evaluated independently for health.
use crate::{verify, AgentIdentity, DsseEnvelope, TrustedKey};
use intent_protocol::{
    assurance::{Outcome, ProbeSpec},
    check::validate_probe,
    evidence::*,
    payload_digest,
    state::Completeness,
    AgentRole, DeviceBinding, RevisionRef, PROTOCOL_VERSION,
};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvidenceError {
    #[error(transparent)]
    Identity(#[from] crate::IdentityError),
    #[error("invalid statement encoding")]
    Json(#[from] serde_json::Error),
    #[error("statement does not match authorized {0}")]
    Binding(&'static str),
}

/// Read from the runtime's own authenticated configuration and current fencing
/// record, never constructed by copying fields from an incoming assignment.
pub struct AssignmentScope<'a> {
    pub controller: &'a AgentIdentity,
    pub witness: &'a AgentIdentity,
    pub device: &'a DeviceBinding,
    pub revision: &'a RevisionRef,
    pub plan_id: &'a str,
    pub plan_epoch: u64,
    pub graph_version: u64,
    pub location: &'a str,
    pub bind_addresses: &'a [std::net::IpAddr],
}

pub struct VerifiedAssignment {
    assignment: WitnessAssignment,
    envelope: DsseEnvelope,
}
impl VerifiedAssignment {
    pub fn envelope(&self) -> &DsseEnvelope { &self.envelope }
    pub fn assignment(&self) -> &WitnessAssignment {
        &self.assignment
    }
}

pub fn verify_assignment(
    envelope: &DsseEnvelope,
    keys: &[TrustedKey],
    scope: &AssignmentScope<'_>,
    now_ms: u64,
) -> Result<VerifiedAssignment, EvidenceError> {
    let payload = verify(
        envelope,
        keys,
        scope.controller,
        AgentRole::Controller,
        now_ms,
    )?;
    let statement: Statement<WitnessAssignment> = serde_json::from_slice(payload.bytes())?;
    let a = statement.predicate;
    if statement.statement_type != STATEMENT_TYPE
        || statement.predicate_type != WITNESS_ASSIGNMENT_TYPE
        || statement.subject != revision_subject(scope.revision)
        || a.version != PROTOCOL_VERSION
        || a.assignment_id.is_empty()
        || a.assignment_id.len() > 256
        || a.issuer != scope.controller.to_string()
        || scope.witness.role() != AgentRole::Witness
        || a.recipient != scope.witness.to_string()
        || a.device != *scope.device
        || a.revision != *scope.revision
        || a.plan_id != scope.plan_id
        || a.plan_epoch != scope.plan_epoch
        || a.graph_version != scope.graph_version
        || a.issued_at_ms > now_ms
        || now_ms >= a.expires_at_ms
        || a.expires_at_ms.saturating_sub(a.issued_at_ms) > 86_400_000
        || a.probes.is_empty()
        || a.probes.len() > 1024
    {
        return Err(EvidenceError::Binding(
            "assignment scope, revision, recipient or validity",
        ));
    }
    let mut ids = HashSet::new();
    for p in &a.probes {
        if !ids.insert(&p.id)
            || p.device != *scope.device
            || p.source.witness_id != a.recipient
            || p.source.location != scope.location
            || !scope.bind_addresses.contains(&p.source.bind_address)
            || validate_probe(p).is_err()
        {
            return Err(EvidenceError::Binding("probe source, device or limits"));
        }
    }
    Ok(VerifiedAssignment { assignment: a, envelope: envelope.clone() })
}

pub struct VerifiedEvidence {
    evidence: WitnessEvidence,
    envelope: DsseEnvelope,
    assignment_envelope: DsseEnvelope,
    interval_ms: u64,
    timeout_ms: u64,
}
impl VerifiedEvidence {
    /// Keep the signed bytes and signatures for later audit verification.
    pub fn envelope(&self) -> &DsseEnvelope { &self.envelope }
    pub fn assignment_envelope(&self) -> &DsseEnvelope { &self.assignment_envelope }
    pub fn evidence(&self) -> &WitnessEvidence {
        &self.evidence
    }
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
    pub fn outcome_at(&self, now_ms: u64) -> Outcome {
        let r = &self.evidence.result;
        if !matches!(self.evidence.completeness, Completeness::Complete)
            || r.finished_at_ms > now_ms
            || now_ms.saturating_sub(r.finished_at_ms)
                > self
                    .interval_ms
                    .saturating_mul(3)
                    .saturating_add(self.timeout_ms)
        {
            Outcome::Unavailable
        } else if r.outcome == Outcome::Success
            && r.finished_at_ms.saturating_sub(r.started_at_ms) > self.timeout_ms
        {
            Outcome::Timeout
        } else {
            r.outcome
        }
    }
}

pub fn verify_evidence(
    envelope: &DsseEnvelope,
    keys: &[TrustedKey],
    assignment: &VerifiedAssignment,
    now_ms: u64,
) -> Result<VerifiedEvidence, EvidenceError> {
    let a = assignment.assignment();
    let witness = AgentIdentity::parse(&a.recipient)?;
    let payload = verify(envelope, keys, &witness, AgentRole::Witness, now_ms)?;
    let statement: Statement<WitnessEvidence> = serde_json::from_slice(payload.bytes())?;
    let e = statement.predicate;
    if statement.statement_type != STATEMENT_TYPE
        || statement.predicate_type != WITNESS_EVIDENCE_TYPE
        || statement.subject != revision_subject(&a.revision)
        || e.version != PROTOCOL_VERSION
        || e.evidence_id.is_empty()
        || e.evidence_id.len() > 256
        || e.assignment_id != a.assignment_id
        || e.plan_id != a.plan_id
        || e.plan_epoch != a.plan_epoch
        || e.graph_version != a.graph_version
        || e.revision != a.revision
        || e.device != a.device
        || e.witness != a.recipient
        || e.result.started_at_ms < a.issued_at_ms
        || e.result.started_at_ms > e.result.finished_at_ms
        || e.result.finished_at_ms > now_ms
        || e.result.finished_at_ms >= a.expires_at_ms
        || e.result.detail.len() > 4096
    {
        return Err(EvidenceError::Binding("evidence scope or timestamps"));
    }
    let probe = a
        .probes
        .iter()
        .find(|p| p.id == e.result.probe_id)
        .ok_or(EvidenceError::Binding("probe ID"))?;
    if e.probe_digest != probe_digest(probe)? {
        return Err(EvidenceError::Binding("exact assigned probe"));
    }
    Ok(VerifiedEvidence {
        evidence: e,
        envelope: envelope.clone(),
        assignment_envelope: assignment.envelope().clone(),
        interval_ms: probe.interval_ms,
        timeout_ms: probe.limits.timeout_ms,
    })
}

/// The typed assigned probe is serialized once with this version's serde wire
/// encoding. This digest is distinct from the digest of exact DSSE payload bytes.
pub fn probe_digest(probe: &ProbeSpec) -> Result<String, serde_json::Error> {
    Ok(payload_digest(&serde_json::to_vec(probe)?))
}
