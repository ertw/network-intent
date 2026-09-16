//! Observation execution after assignment signature/scope verification. The
//! witness service must persist the returned signed envelope before delivery.
use crate::{execute_with_local, local::LocalObservationContext, now_ms};
use intent_identity::{evidence::{probe_digest, VerifiedAssignment}, AgentIdentity, DsseEnvelope, SigningKey};
use intent_protocol::{assurance::Outcome, evidence::*, state::Completeness, AgentRole, PROTOCOL_VERSION};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("assignment does not match this witness's installed execution scope")]
    Scope,
    #[error("assignment has insufficient remaining validity for the bounded observation")]
    Expired,
    #[error("evidence encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("evidence signing failed: {0}")]
    Signing(#[from] intent_identity::IdentityError),
}

pub struct WitnessExecutor {
    identity: AgentIdentity,
    signer: SigningKey,
    local: LocalObservationContext,
}
impl WitnessExecutor {
    pub fn new(identity: AgentIdentity, signer: SigningKey, local: LocalObservationContext) -> Result<Self, ExecutionError> {
        if identity.role() != AgentRole::Witness || local.source.witness_id != identity.to_string() {
            return Err(ExecutionError::Scope);
        }
        Ok(Self { identity, signer, local })
    }

    /// `evidence_id` is allocated by the durable witness work queue. A delivery
    /// retry resends the saved envelope; it must not reexecute under that ID.
    pub async fn run_probe(&self, verified: &VerifiedAssignment, probe_id: &str, evidence_id: &str) -> Result<DsseEnvelope, ExecutionError> {
        let assignment = verified.assignment();
        let probe = assignment.probes.iter().find(|p| p.id == probe_id).ok_or(ExecutionError::Scope)?;
        if assignment.recipient != self.identity.to_string() || probe.source != self.local.source
            || evidence_id.is_empty() || evidence_id.len() > 256 || evidence_id.chars().any(char::is_control) {
            return Err(ExecutionError::Scope);
        }
        let now = now_ms();
        if now < assignment.issued_at_ms || now.saturating_add(probe.limits.timeout_ms) >= assignment.expires_at_ms {
            return Err(ExecutionError::Expired);
        }
        let result = execute_with_local(probe, &self.local).await;
        if result.finished_at_ms >= assignment.expires_at_ms { return Err(ExecutionError::Expired); }
        let completeness = match result.outcome {
            Outcome::Unsupported | Outcome::Unavailable | Outcome::MalformedResponse => Completeness::Unavailable { reason: "observation could not produce complete evidence".into() },
            _ => Completeness::Complete,
        };
        let evidence = WitnessEvidence {
            deployment: assignment.deployment.clone(),
            version: PROTOCOL_VERSION, evidence_id: evidence_id.into(), assignment_id: assignment.assignment_id.clone(),
            plan_id: assignment.plan_id.clone(), plan_epoch: assignment.plan_epoch, graph_version: assignment.graph_version,
            revision: assignment.revision.clone(), device: assignment.device.clone(), witness: self.identity.to_string(),
            probe_digest: probe_digest(probe)?, completeness, result,
        };
        let statement = Statement { statement_type: STATEMENT_TYPE.into(), subject: revision_subject(&assignment.revision),
            predicate_type: WITNESS_EVIDENCE_TYPE.into(), predicate: evidence };
        Ok(self.signer.sign(&serde_json::to_vec(&statement)?)?)
    }
}
