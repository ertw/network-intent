//! Independent post-apply evidence gate. Expected probe bindings and LAN
//! placement come from the controller's admitted plan and trusted inventory.
use crate::{
    DsseEnvelope,
    evidence::{VerifiedEvidence, probe_digest},
};
use intent_protocol::{
    DeviceBinding, RevisionRef,
    assurance::{Outcome, Primitive, ProbeSource},
    evidence::DeploymentBinding,
};
use std::collections::HashSet;
use thiserror::Error;

pub struct RequiredVerification {
    pub probe_id: String,
    pub probe_digest: String,
    pub source: ProbeSource,
    /// Set only by trusted topology/inventory; never copied from evidence.
    pub lan_management_path: bool,
}
pub struct ConfirmationScope<'a> {
    pub deployment: &'a DeploymentBinding,
    pub device: &'a DeviceBinding,
    pub revision: &'a RevisionRef,
    pub plan_id: &'a str,
    pub plan_epoch: u64,
    pub graph_version: u64,
    pub provisional_started_at_ms: u64,
    pub deadline_ms: u64,
    pub required: &'a [RequiredVerification],
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfirmationError {
    #[error("confirmation scope lacks a bound LAN management-path contract")]
    IncompleteScope,
    #[error("confirmation requires two complete independent verification rounds")]
    MissingRounds,
    #[error(
        "evidence is stale, unsuccessful, duplicated, out of order, or outside deployment scope"
    )]
    InvalidEvidence,
    #[error("deployment confirmation window has expired or is invalid")]
    Expired,
}

/// Only this module can construct a successful confirmation. It is not
/// deserializable: received JSON is never an authorization to confirm.
pub struct VerifiedDeploymentConfirmation {
    deployment: DeploymentBinding,
    device: DeviceBinding,
    revision: RevisionRef,
    plan_id: String,
    plan_epoch: u64,
    graph_version: u64,
    confirmed_at_ms: u64,
    provisional_started_at_ms: u64,
    deadline_ms: u64,
    expires_at_ms: u64,
    proof_digest: String,
    proofs: Vec<Vec<SignedVerificationProof>>,
}

#[derive(Clone, serde::Serialize)]
pub struct SignedVerificationProof {
    pub assignment: DsseEnvelope,
    pub evidence: DsseEnvelope,
}
impl VerifiedDeploymentConfirmation {
    pub fn deployment(&self) -> &DeploymentBinding {
        &self.deployment
    }
    pub fn device(&self) -> &DeviceBinding {
        &self.device
    }
    pub fn revision(&self) -> &RevisionRef {
        &self.revision
    }
    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }
    pub fn plan_epoch(&self) -> u64 {
        self.plan_epoch
    }
    pub fn graph_version(&self) -> u64 {
        self.graph_version
    }
    pub fn confirmed_at_ms(&self) -> u64 {
        self.confirmed_at_ms
    }
    pub fn provisional_started_at_ms(&self) -> u64 {
        self.provisional_started_at_ms
    }
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
    pub fn proof_digest(&self) -> &str {
        &self.proof_digest
    }
    pub fn proofs(&self) -> &[Vec<SignedVerificationProof>] {
        &self.proofs
    }
}

pub fn verify_confirmation(
    rounds: &[Vec<&VerifiedEvidence>],
    scope: &ConfirmationScope<'_>,
    now_ms: u64,
) -> Result<VerifiedDeploymentConfirmation, ConfirmationError> {
    if scope.deadline_ms <= scope.provisional_started_at_ms
        || now_ms >= scope.deadline_ms
        || now_ms < scope.provisional_started_at_ms
    {
        return Err(ConfirmationError::Expired);
    }
    let required_ids: HashSet<_> = scope.required.iter().map(|r| r.probe_id.as_str()).collect();
    if scope.required.is_empty()
        || required_ids.len() != scope.required.len()
        || !scope.required.iter().any(|r| r.lan_management_path)
    {
        return Err(ConfirmationError::IncompleteScope);
    }
    if rounds.len() != 2 {
        return Err(ConfirmationError::MissingRounds);
    }
    let mut evidence_ids = HashSet::new();
    let mut previous_round_end = None;
    let mut expires_at_ms = scope.deadline_ms;
    for round in rounds {
        if round.len() != scope.required.len() {
            return Err(ConfirmationError::MissingRounds);
        }
        let mut round_ids = HashSet::new();
        let mut round_end = 0;
        for verified in round {
            let e = verified.evidence();
            let probe = verified.assigned_probe();
            let expected = scope
                .required
                .iter()
                .find(|r| r.probe_id == e.result.probe_id)
                .ok_or(ConfirmationError::InvalidEvidence)?;
            if !round_ids.insert(e.result.probe_id.as_str())
                || !evidence_ids.insert(e.evidence_id.as_str())
                || e.deployment.as_ref() != Some(scope.deployment)
                || &e.device != scope.device
                || &e.revision != scope.revision
                || e.plan_id != scope.plan_id
                || e.plan_epoch != scope.plan_epoch
                || e.graph_version != scope.graph_version
                || e.probe_digest != expected.probe_digest
                || probe_digest(probe).ok().as_ref() != Some(&expected.probe_digest)
                || probe.source != expected.source
                || e.witness != expected.source.witness_id
                || verified.outcome_at(now_ms) != Outcome::Success
                || e.result.started_at_ms < scope.provisional_started_at_ms
                || e.result.finished_at_ms >= scope.deadline_ms
                || previous_round_end.is_some_and(|end| e.result.started_at_ms <= end)
                || (expected.lan_management_path
                    && (!matches!(probe.primitive, Primitive::TcpConnect | Primitive::Http)
                        || probe.endpoint.is_none()))
            {
                return Err(ConfirmationError::InvalidEvidence);
            }
            round_end = round_end.max(e.result.finished_at_ms);
            // Freshness is inclusive at its last millisecond; expose an
            // exclusive expiry suitable for the journal's current-time check.
            expires_at_ms = expires_at_ms.min(
                e.result
                    .finished_at_ms
                    .saturating_add(verified.interval_ms().saturating_mul(3))
                    .saturating_add(verified.timeout_ms())
                    .saturating_add(1),
            );
        }
        previous_round_end = Some(round_end);
    }
    let proofs: Vec<Vec<_>> = rounds
        .iter()
        .map(|round| {
            round
                .iter()
                .map(|e| SignedVerificationProof {
                    assignment: e.assignment_envelope().clone(),
                    evidence: e.envelope().clone(),
                })
                .collect()
        })
        .collect();
    let proof_digest = intent_protocol::payload_digest(
        &serde_json::to_vec(&proofs).map_err(|_| ConfirmationError::InvalidEvidence)?,
    );
    Ok(VerifiedDeploymentConfirmation {
        deployment: scope.deployment.clone(),
        device: scope.device.clone(),
        revision: scope.revision.clone(),
        plan_id: scope.plan_id.into(),
        plan_epoch: scope.plan_epoch,
        graph_version: scope.graph_version,
        confirmed_at_ms: now_ms,
        provisional_started_at_ms: scope.provisional_started_at_ms,
        deadline_ms: scope.deadline_ms,
        expires_at_ms,
        proof_digest,
        proofs,
    })
}
