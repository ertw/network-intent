//! Controller evidence ingress.  This is deliberately a controller boundary:
//! it combines the authenticated transport peer, controller-owned inventory and
//! authorization, durable receipt, and monitor ingestion.  It is not a shared
//! agent runtime.

use crate::{
    monitor::MonitorStore,
    storage::{ReceiveOutcome, Store},
};
use intent_authorization::{Action, AuthorizationEngine, Decision};
use intent_identity::{
    evidence::{
        verify_assignment, verify_evidence, AssignmentScope, EvidenceError, VerifiedEvidence,
    },
    AgentIdentity, DsseEnvelope, TrustedKey,
};
use intent_protocol::MessageEnvelope;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The exact signed objects carried in an evidence message.  The outer
/// `MessageEnvelope` owns transport routing and receipt idempotency; the two
/// DSSE envelopes retain their original signatures for evidence audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSubmission {
    pub assignment: DsseEnvelope,
    pub evidence: DsseEnvelope,
}

#[derive(Debug, Error)]
pub enum IngressError {
    #[error("transport peer is not the installed witness for this assignment scope")]
    UnexpectedPeer,
    #[error("message sender is not the authenticated witness")]
    SenderMismatch,
    #[error("message recipient is not this installed controller")]
    RecipientMismatch,
    #[error("message scope does not match the current installed assignment scope")]
    MessageScopeMismatch,
    #[error("Cedar denied evidence submission")]
    Unauthorized,
    #[error("authorization evaluation failed: {0}")]
    Authorization(#[from] intent_authorization::AuthorizationError),
    #[error("invalid signed evidence submission: {0}")]
    Evidence(#[from] EvidenceError),
    #[error("invalid evidence submission encoding: {0}")]
    Json(#[from] serde_json::Error),
    #[error("inbox storage failed: {0}")]
    Storage(#[from] crate::storage::StorageError),
    #[error("monitor ingestion failed: {0}")]
    Monitor(#[from] crate::monitor::MonitorError),
    #[error("processing hook interrupted after monitor ingestion")]
    InterruptedAfterIngest,
}

pub type Result<T> = std::result::Result<T, IngressError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptOutcome {
    Inserted,
    Duplicate,
}

/// Controller-owned dependencies for one installed witness/device assignment
/// route. All borrowed values must have been loaded from authenticated local
/// controller state; no constructor accepts them from a submitted message.
pub struct EvidenceIngress<'a> {
    inbox: &'a Store,
    monitor: &'a MonitorStore,
    authorization: &'a AuthorizationEngine,
    trusted_keys: &'a [TrustedKey],
    scope: AssignmentScope<'a>,
}

impl<'a> EvidenceIngress<'a> {
    pub fn new(
        inbox: &'a Store,
        monitor: &'a MonitorStore,
        authorization: &'a AuthorizationEngine,
        trusted_keys: &'a [TrustedKey],
        scope: AssignmentScope<'a>,
    ) -> Self {
        Self {
            inbox,
            monitor,
            authorization,
            trusted_keys,
            scope,
        }
    }

    /// Validates the authenticated peer, signed statements, Cedar grant and
    /// exact current scope *before* creating the durable inbox receipt.  A
    /// caller may acknowledge its transport only after this returns `Ok`.
    pub fn receive(
        &self,
        authenticated_peer: &AgentIdentity,
        envelope: &MessageEnvelope,
        now_ms: u64,
    ) -> Result<ReceiptOutcome> {
        // Bound malformed/expired/oversized work before JSON or signature
        // parsing. This is the same storage validation repeated by `receive`
        // immediately before the durable transaction.
        crate::storage::validate(envelope, now_ms)?;
        self.verify_message(authenticated_peer, envelope, now_ms)?;
        Ok(match self.inbox.receive(envelope, now_ms)? {
            ReceiveOutcome::Inserted => ReceiptOutcome::Inserted,
            ReceiveOutcome::Duplicate => ReceiptOutcome::Duplicate,
        })
    }

    /// Replays durable receipts. Verification and authorization are repeated
    /// here because the controller's current inventory, enrollment and Cedar
    /// grants may have changed after receipt. Monitor idempotency makes a
    /// crash after `ingest` and before this receipt is marked safe to replay.
    pub fn process_pending(&self, now_ms: u64, limit: usize) -> Result<usize> {
        self.process_pending_with_hook(now_ms, limit, &mut NoopHook)
    }

    /// Same as [`Self::process_pending`], with an explicit crash seam used by
    /// fault-injection tests and shutdown coordinators. Returning an error
    /// from the hook leaves the inbox receipt pending after durable monitor
    /// ingestion; the next replay is a monitor duplicate and then marks it.
    pub fn process_pending_with_hook(
        &self,
        now_ms: u64,
        limit: usize,
        hook: &mut impl ProcessingHook,
    ) -> Result<usize> {
        let pending = self.inbox.pending_inbox_route(
            now_ms,
            limit,
            &self.scope.witness.to_string(),
            &self.scope.controller.to_string(),
            self.scope.device,
        )?;
        let mut completed = 0;
        for message in pending {
            let verified = self.verify_message(self.scope.witness, &message.envelope, now_ms)?;
            let _outcome = self.monitor.ingest(&verified, now_ms)?;
            hook.after_monitor_ingest(&message.envelope.message_id)?;
            if self
                .inbox
                .mark_inbox_processed(&message.envelope.message_id, now_ms)?
            {
                completed += 1;
            }
        }
        Ok(completed)
    }

    fn verify_message(
        &self,
        authenticated_peer: &AgentIdentity,
        envelope: &MessageEnvelope,
        now_ms: u64,
    ) -> Result<VerifiedEvidence> {
        if authenticated_peer != self.scope.witness {
            return Err(IngressError::UnexpectedPeer);
        }
        if envelope.sender != authenticated_peer.to_string() {
            return Err(IngressError::SenderMismatch);
        }
        if envelope.recipient != self.scope.controller.to_string() {
            return Err(IngressError::RecipientMismatch);
        }
        if envelope.device != *self.scope.device || envelope.plan_epoch != self.scope.plan_epoch {
            return Err(IngressError::MessageScopeMismatch);
        }
        if self.authorization.authorize(
            authenticated_peer,
            Action::SubmitEvidence,
            &envelope.device,
        )? != Decision::Allow
        {
            return Err(IngressError::Unauthorized);
        }
        // On receipt the Store verifies this payload's exact advertised digest
        // before committing it. On replay, `pending_inbox` returns those
        // original bytes; malformed legacy rows still fail closed here.
        let submission: EvidenceSubmission = serde_json::from_slice(&envelope.payload)?;
        let assignment = verify_assignment(
            &submission.assignment,
            self.trusted_keys,
            &self.scope,
            now_ms,
        )?;
        let verified =
            verify_evidence(&submission.evidence, self.trusted_keys, &assignment, now_ms)?;
        // `verify_evidence` checks the DSSE signer against assignment recipient;
        // retain this explicit identity equality as a guard for future verifier
        // changes and to keep the transport/signature binding local.
        if verified.evidence().witness != authenticated_peer.to_string() {
            return Err(IngressError::SenderMismatch);
        }
        Ok(verified)
    }
}

pub trait ProcessingHook {
    fn after_monitor_ingest(&mut self, message_id: &str) -> Result<()>;
}

struct NoopHook;
impl ProcessingHook for NoopHook {
    fn after_monitor_ingest(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
}
