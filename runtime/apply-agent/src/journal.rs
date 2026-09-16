//! Durable guarded-apply state machine.  This module deliberately has no UCI
//! transport: the only effectful boundary is [`ApplyBackend`].
use intent_identity::confirmation::VerifiedDeploymentConfirmation;
use intent_protocol::{DeviceBinding, RevisionRef, evidence::DeploymentBinding, payload_digest};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};
use thiserror::Error;

pub const DEFAULT_CONFIRMATION_WINDOW_MS: u64 = 180_000;
pub const REQUIRED_VERIFICATION_ROUNDS: u32 = 2;
const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedBaselineField {
    pub package: String,
    pub section: String,
    pub field: String,
    /// Digest of a non-secret baseline value; raw values never enter this journal.
    pub value_digest: Option<String>,
    /// Stable secret-store reference, never a secret value.
    pub secret_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRequest {
    pub deployment_id: String,
    pub device: DeviceBinding,
    pub plan_epoch: u64,
    pub revision: RevisionRef,
    pub plan_id: String,
    pub plan_digest: String,
    pub graph_version: u64,
    pub owned_baseline_digest: String,
    pub owned_baseline: Vec<OwnedBaselineField>,
    pub confirmation_window_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentPhase {
    Prepared,
    Preflighted,
    Staged,
    Checkpointed,
    ProvisionalIntent,
    AwaitingVerification,
    ConfirmIntent,
    Confirmed,
    RollingBack,
    RolledBack,
    Uncertain,
}
impl DeploymentPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Preflighted => "preflighted",
            Self::Staged => "staged",
            Self::Checkpointed => "checkpointed",
            Self::ProvisionalIntent => "provisional_intent",
            Self::AwaitingVerification => "awaiting_verification",
            Self::ConfirmIntent => "confirm_intent",
            Self::Confirmed => "confirmed",
            Self::RollingBack => "rolling_back",
            Self::RolledBack => "rolled_back",
            Self::Uncertain => "uncertain",
        }
    }
    fn parse(v: &str) -> Result<Self> {
        Ok(match v {
            "prepared" => Self::Prepared,
            "preflighted" => Self::Preflighted,
            "staged" => Self::Staged,
            "checkpointed" => Self::Checkpointed,
            "provisional_intent" => Self::ProvisionalIntent,
            "awaiting_verification" => Self::AwaitingVerification,
            "confirm_intent" => Self::ConfirmIntent,
            "confirmed" => Self::Confirmed,
            "rolling_back" => Self::RollingBack,
            "rolled_back" => Self::RolledBack,
            "uncertain" => Self::Uncertain,
            _ => return Err(JournalError::CorruptState),
        })
    }
    // Only successful forward progress makes an earlier delivered step a no-op.
    fn completed_step(self, step: Self) -> bool {
        let rank = |p| match p {
            Self::Prepared => 0,
            Self::Preflighted => 1,
            Self::Staged => 2,
            Self::Checkpointed => 3,
            Self::ProvisionalIntent => 4,
            Self::AwaitingVerification => 5,
            Self::ConfirmIntent => 6,
            Self::Confirmed => 7,
            _ => -1,
        };
        rank(self) >= rank(step)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentStatus {
    pub request: DeploymentRequest,
    pub checkpoint_digest: String,
    pub phase: DeploymentPhase,
    pub expires_at_ms: u64,
    pub verification_rounds: u32,
    pub intent_marker: bool,
    pub provisional_started_at_ms: Option<u64>,
}

/// The interval committed before a provisional apply. A native backend must
/// arm its timed apply with the remaining time before its first live effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyWindow {
    pub started_at_ms: u64,
    pub deadline_ms: u64,
}

/// All calls run under the journal's cross-process operation lock. Backends
/// must implement preflight, stage, checkpoint, discard, and rollback idempotently by
/// deployment ID (their replies can be lost). The restore artifact must remain
/// available until a terminal outcome is durably known. Never reenter journal
/// mutations from a backend callback. Native UCI integration is separate.
pub trait ApplyBackend {
    type Error: std::error::Error + Send + Sync + 'static;
    /// Read only: never reapply. Confirmed must be an authoritative backend
    /// observation bound to the exact restore receipt, not a guess from config.
    fn reconcile(
        &mut self,
        deployment_id: &str,
    ) -> std::result::Result<BackendDisposition, Self::Error>;
    fn preflight(&mut self, deployment_id: &str) -> std::result::Result<(), Self::Error>;
    fn stage(&mut self, deployment_id: &str) -> std::result::Result<(), Self::Error>;
    /// Fsync the restore artifact before returning a stable receipt.
    fn durable_checkpoint(
        &mut self,
        deployment_id: &str,
    ) -> std::result::Result<CheckpointReceipt, Self::Error>;
    /// Discard session staging only; this must not modify live configuration.
    fn discard_staging(&mut self, deployment_id: &str) -> std::result::Result<(), Self::Error>;
    /// Arm a timed apply for the remaining durable window before the first
    /// live effect, and refuse when the backend's current time reaches it.
    fn provisional_apply(
        &mut self,
        deployment_id: &str,
        plan_digest: &str,
        window: ApplyWindow,
    ) -> std::result::Result<(), Self::Error>;
    /// Confirm before this exclusive authorization expiry. The backend must
    /// refuse if expiry is reached while it waits or before its live effect.
    fn confirm(
        &mut self,
        deployment_id: &str,
        checkpoint_digest: &str,
        authorization_expires_at_ms: u64,
    ) -> std::result::Result<(), Self::Error>;
    fn rollback(
        &mut self,
        deployment_id: &str,
        checkpoint_digest: &str,
    ) -> std::result::Result<(), Self::Error>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendDisposition {
    AppliedOrUnknown,
    /// Definitively provisional, with no backend confirmation.
    Provisional,
    AlreadyRolledBack,
    Confirmed {
        checkpoint_digest: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointReceipt {
    pub restore_artifact_digest: String,
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("filesystem: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal mutex poisoned")]
    Poisoned,
    #[error("invalid deployment request: {0}")]
    Invalid(&'static str),
    #[error("deployment ID reused with conflicting immutable contents")]
    ConflictingDeployment,
    #[error("another deployment is unfinished for this device")]
    ActiveDeployment,
    #[error("deployment is not in the required phase (found {0:?})")]
    WrongPhase(DeploymentPhase),
    #[error("device fence, profile, or plan epoch is stale")]
    StaleBinding,
    #[error("journal contains invalid or inconsistent state")]
    CorruptState,
    #[error("database schema is newer than this agent")]
    FutureSchema,
    #[error("backend operation failed: {0}")]
    Backend(String),
    #[error("backend confirmation outcome is unknown; reconciliation must resolve it first")]
    UncertainConfirmation,
}
pub type Result<T> = std::result::Result<T, JournalError>;

/// A trusted local receipt, written only after accepting an unforgeable identity
/// token. This is not a wire authorization and has no public import API. Its
/// digest binds the verified rounds; the remaining fields permit fail-closed
/// restart validation against the local immutable request and apply interval.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmationProof {
    request_digest: String,
    checkpoint_digest: String,
    proof_digest: String,
    signed_proofs_json: String,
    confirmed_at_ms: u64,
    accepted_at_ms: u64,
    expires_at_ms: u64,
    provisional_started_at_ms: u64,
    deadline_ms: u64,
}

/// Main-journal FULL-synchronous commits can complete while an IMMEDIATE
/// transaction in the dedicated operation-lock database remains held. Thus an
/// intent survives a crash *before* its physical effect, without exposing an
/// unlocked gap to another handle/process between the intent and effect.
/// Both databases must reside in the same private local directory; do not
/// unlink/replace them while an agent is running (SQLite's normal file rule).
pub struct DeploymentJournal {
    connection: Mutex<Connection>,
    operation_lock: Mutex<Connection>,
}
impl DeploymentJournal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut c = Connection::open(path.as_ref())?;
        c.busy_timeout(Duration::from_secs(5))?;
        let canonical = std::fs::canonicalize(path.as_ref())?;
        let mut lock_path = canonical.into_os_string();
        lock_path.push(".operations.sqlite");
        let mut operation_lock = Connection::open(std::path::PathBuf::from(lock_path))?;
        operation_lock.busy_timeout(Duration::from_secs(5))?;
        let guard = operation_lock.transaction_with_behavior(TransactionBehavior::Immediate)?;
        c.pragma_update(None, "journal_mode", "WAL")?;
        let mode: String = c.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(JournalError::Invalid("SQLite WAL unavailable"));
        }
        c.pragma_update(None, "foreign_keys", "ON")?;
        c.pragma_update(None, "synchronous", "FULL")?;
        migrate(&mut c)?;
        guard.commit()?;
        Ok(Self {
            connection: Mutex::new(c),
            operation_lock: Mutex::new(operation_lock),
        })
    }

    fn exclusive<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut lock = self
            .operation_lock
            .lock()
            .map_err(|_| JournalError::Poisoned)?;
        let guard = lock.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut c = self.connection.lock().map_err(|_| JournalError::Poisoned)?;
        let result = f(&mut c);
        // No data is stored in the lock database. Dropping the transaction on
        // error/crash releases the OS-backed SQLite lock, with no stale lease.
        guard.commit()?;
        result
    }

    pub fn prepare(&self, request: &DeploymentRequest, now_ms: u64) -> Result<DeploymentStatus> {
        validate_request(request)?;
        let immutable = request_digest(request)?;
        self.exclusive(|c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let existing: Option<String> = tx.query_row("SELECT immutable_digest FROM deployments WHERE deployment_id=?1", [&request.deployment_id], |r| r.get(0)).optional()?;
            if let Some(existing) = existing {
                if existing != immutable { return Err(JournalError::ConflictingDeployment); }
                let s = bound_status(&tx, &request.deployment_id)?;
                tx.commit()?;
                return Ok(s);
            }
            let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM deployments WHERE device_id=?1 AND phase NOT IN ('confirmed','rolled_back'))", [&request.device.device_id], |r| r.get(0))?;
            if active { return Err(JournalError::ActiveDeployment); }
            let fence: Option<(u64, u64, String)> = tx.query_row("SELECT fence,epoch,profile_digest FROM device_fences WHERE device_id=?1", [&request.device.device_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?;
            if let Some((f, e, p)) = fence {
                if request.device.fencing_generation < f || request.plan_epoch < e
                    || (request.device.fencing_generation == f && request.device.profile_digest != p)
                { return Err(JournalError::StaleBinding); }
            }
            tx.execute("INSERT INTO device_fences(device_id,fence,epoch,profile_digest) VALUES(?1,?2,?3,?4) ON CONFLICT(device_id) DO UPDATE SET fence=excluded.fence,epoch=excluded.epoch,profile_digest=excluded.profile_digest", params![request.device.device_id,request.device.fencing_generation,request.plan_epoch,request.device.profile_digest])?;
            tx.execute("INSERT INTO deployments(deployment_id,device_id,immutable_digest,checkpoint_digest,request,phase,expires_at_ms,rounds,intent_marker,created_at_ms) VALUES(?1,?2,?3,'',?4,'prepared',?5,0,0,?6)", params![request.deployment_id,request.device.device_id,immutable,serde_json::to_string(request)?,i64::MAX,now_ms])?;
            let s = bound_status(&tx, &request.deployment_id)?;
            tx.commit()?;
            Ok(s)
        })
    }

    pub fn preflight<B: ApplyBackend>(&self, id: &str, backend: &mut B) -> Result<()> {
        self.exclusive(|c| {
            let s = bound_status(c, id)?;
            if s.phase.completed_step(DeploymentPhase::Preflighted) {
                return Ok(());
            }
            require_phase(&s, DeploymentPhase::Prepared)?;
            backend.preflight(id).map_err(backend_error)?;
            advance(c, id, s.phase, DeploymentPhase::Preflighted)
        })
    }
    pub fn stage<B: ApplyBackend>(&self, id: &str, backend: &mut B) -> Result<()> {
        self.exclusive(|c| {
            let s = bound_status(c, id)?;
            if s.phase.completed_step(DeploymentPhase::Staged) {
                return Ok(());
            }
            require_phase(&s, DeploymentPhase::Preflighted)?;
            backend.stage(id).map_err(backend_error)?;
            advance(c, id, s.phase, DeploymentPhase::Staged)
        })
    }
    pub fn durable_checkpoint<B: ApplyBackend>(&self, id: &str, backend: &mut B) -> Result<()> {
        self.exclusive(|c| {
            let s = bound_status(c, id)?;
            if s.phase.completed_step(DeploymentPhase::Checkpointed) {
                return Ok(());
            }
            require_phase(&s, DeploymentPhase::Staged)?;
            let receipt = backend.durable_checkpoint(id).map_err(backend_error)?;
            if receipt.restore_artifact_digest.is_empty() {
                return Err(JournalError::Invalid("empty durable checkpoint receipt"));
            }
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            bound_status(&tx, id)?;
            cas(&tx, id, s.phase, DeploymentPhase::Checkpointed)?;
            tx.execute(
                "UPDATE deployments SET checkpoint_digest=?2 WHERE deployment_id=?1",
                params![id, receipt.restore_artifact_digest],
            )?;
            tx.commit()?;
            Ok(())
        })
    }
    pub fn provisional_apply<B: ApplyBackend>(
        &self,
        id: &str,
        now_ms: u64,
        backend: &mut B,
    ) -> Result<()> {
        let entered_at = Instant::now();
        self.exclusive(|c| {
            let now_ms = elapsed_now_ms(now_ms, entered_at)?;
            let s = bound_status(c, id)?;
            if matches!(
                s.phase,
                DeploymentPhase::AwaitingVerification
                    | DeploymentPhase::ConfirmIntent
                    | DeploymentPhase::Confirmed
            ) {
                return Ok(());
            }
            require_phase(&s, DeploymentPhase::Checkpointed)?;
            let window = begin_apply(c, &s, now_ms)?;
            bound_status(c, id)?;
            match backend.provisional_apply(id, &s.request.plan_digest, window) {
                Ok(()) => advance(
                    c,
                    id,
                    DeploymentPhase::ProvisionalIntent,
                    DeploymentPhase::AwaitingVerification,
                ),
                Err(e) => {
                    advance(
                        c,
                        id,
                        DeploymentPhase::ProvisionalIntent,
                        DeploymentPhase::Uncertain,
                    )?;
                    Err(backend_error(e))
                }
            }
        })
    }

    /// Persist the sealed proof and confirm intent before calling the backend.
    /// A lost confirm reply leaves ConfirmIntent, never an invented success.
    pub fn record_independent_confirmation<B: ApplyBackend>(
        &self,
        id: &str,
        evidence: &VerifiedDeploymentConfirmation,
        now_ms: u64,
        backend: &mut B,
    ) -> Result<DeploymentPhase> {
        let entered_at = Instant::now();
        self.exclusive(|c| {
            let caller_now_ms = now_ms;
            let now_ms = elapsed_now_ms(caller_now_ms, entered_at)?;
            let s = bound_status(c, id)?;
            if s.phase == DeploymentPhase::Confirmed {
                validate_persisted_proof(c, &s)?;
                return Ok(s.phase);
            }
            if s.phase == DeploymentPhase::ConfirmIntent {
                // The authorization was already durably sealed before the
                // backend confirm call.  A retry must only reconcile that
                // call; it must not require a still-fresh token or issue a
                // second physical confirmation.
                return reconcile_locked(c, &s, backend);
            }
            // Validate before a new confirmation intent: neither a stale token
            // nor a token scoped to another apply window authorizes an effect.
            let proof = confirmation_proof(&s, evidence, now_ms)?;
            require_phase(&s, DeploymentPhase::AwaitingVerification)?;
            begin_confirmation(c, &s, &proof)?;
            bound_status(c, id)?;
            let authorization_expires_at_ms = proof.expires_at_ms.min(s.expires_at_ms);
            if elapsed_now_ms(caller_now_ms, entered_at)? >= authorization_expires_at_ms {
                // The sealed intent is retained so a lost/late call can only
                // be reconciled; no backend confirmation is issued here.
                return Err(JournalError::Invalid("confirmation authorization expired"));
            }
            backend
                .confirm(id, &s.checkpoint_digest, authorization_expires_at_ms)
                .map_err(backend_error)?;
            finish_confirmation(c, id)?;
            Ok(DeploymentPhase::Confirmed)
        })
    }

    /// Startup recovery never repeats apply or confirm. Unknown confirmation
    /// stays fenced until the backend reports an authoritative terminal state
    /// or reports definitively provisional, when safe rollback can proceed.
    pub fn recover<B: ApplyBackend>(&self, _now_ms: u64, backend: &mut B) -> Result<Vec<String>> {
        self.exclusive(|c| {
            let ids: Vec<String> = {
                let mut q = c.prepare("SELECT deployment_id FROM deployments WHERE phase IN ('prepared','preflighted','staged','checkpointed','provisional_intent','awaiting_verification','confirm_intent','uncertain','rolling_back') ORDER BY deployment_id")?;
                let rows = q.query_map([], |r| r.get(0))?.collect::<std::result::Result<_,_>>()?;
                rows
            };
            let mut done = Vec::new();
            let mut first_error = None;
            for id in ids {
                let result = bound_status(c, &id).and_then(|s| if s.intent_marker {
                    reconcile_locked(c, &s, backend)
                } else {
                    rollback_locked(c, &s, backend).map(|()| DeploymentPhase::RolledBack)
                });
                match result {
                    Ok(_) => done.push(id),
                    Err(e) => { if first_error.is_none() { first_error = Some(e); } }
                }
            }
            // A broken device does not prevent recovery of unrelated devices.
            if let Some(e) = first_error { return Err(e); }
            Ok(done)
        })
    }
    pub fn rollback<B: ApplyBackend>(&self, id: &str, backend: &mut B) -> Result<()> {
        self.exclusive(|c| {
            let s = bound_status(c, id)?;
            if s.phase == DeploymentPhase::RolledBack {
                return Ok(());
            }
            if s.phase == DeploymentPhase::Confirmed {
                return Err(JournalError::WrongPhase(s.phase));
            }
            if s.phase == DeploymentPhase::ConfirmIntent {
                let phase = reconcile_locked(c, &s, backend)?;
                if phase == DeploymentPhase::Confirmed {
                    return Err(JournalError::WrongPhase(phase));
                }
                return Ok(());
            }
            rollback_locked(c, &s, backend)
        })
    }
    pub fn status(&self, id: &str) -> Result<DeploymentStatus> {
        let c = self.connection.lock().map_err(|_| JournalError::Poisoned)?;
        read_status(&c, id)
    }
}

fn backend_error(e: impl std::fmt::Display) -> JournalError {
    JournalError::Backend(e.to_string())
}
/// `now_ms` comes from the caller, while `Instant` covers time spent waiting
/// for the cross-process operation lock. This keeps journal deadlines from
/// gaining time merely because another deployment was executing.
fn elapsed_now_ms(now_ms: u64, entered_at: Instant) -> Result<u64> {
    let elapsed_ms = u64::try_from(entered_at.elapsed().as_millis())
        .map_err(|_| JournalError::Invalid("elapsed time overflow"))?;
    now_ms
        .checked_add(elapsed_ms)
        .ok_or(JournalError::Invalid("clock deadline overflow"))
}
fn require_phase(s: &DeploymentStatus, expected: DeploymentPhase) -> Result<()> {
    if s.phase != expected {
        return Err(JournalError::WrongPhase(s.phase));
    }
    Ok(())
}
fn read_status(c: &Connection, id: &str) -> Result<DeploymentStatus> {
    c.query_row("SELECT request,checkpoint_digest,phase,expires_at_ms,rounds,intent_marker,provisional_started_at_ms FROM deployments WHERE deployment_id=?1", [id], |r| {
        let raw: String = r.get(0)?;
        Ok(DeploymentStatus {
            request: serde_json::from_str(&raw).map_err(json_sql)?,
            checkpoint_digest: r.get(1)?,
            phase: DeploymentPhase::parse(&r.get::<_,String>(2)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
            expires_at_ms: r.get(3)?, verification_rounds: r.get(4)?,
            intent_marker: r.get::<_,i64>(5)? != 0, provisional_started_at_ms: r.get(6)?,
        })
    }).map_err(Into::into)
}
/// Called before EVERY effect and transition, including retry and recovery.
fn bound_status(c: &Connection, id: &str) -> Result<DeploymentStatus> {
    let s = read_status(c, id)?;
    let current: Option<(u64, u64, String)> = c
        .query_row(
            "SELECT fence,epoch,profile_digest FROM device_fences WHERE device_id=?1",
            [&s.request.device.device_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if current
        != Some((
            s.request.device.fencing_generation,
            s.request.plan_epoch,
            s.request.device.profile_digest.clone(),
        ))
    {
        return Err(JournalError::StaleBinding);
    }
    let (stored_digest, device_id): (String, String) = c.query_row(
        "SELECT immutable_digest,device_id FROM deployments WHERE deployment_id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if s.request.deployment_id != id
        || stored_digest != request_digest(&s.request)?
        || device_id != s.request.device.device_id
    {
        return Err(JournalError::CorruptState);
    }
    Ok(s)
}
fn cas(c: &Connection, id: &str, from: DeploymentPhase, to: DeploymentPhase) -> Result<()> {
    let s = bound_status(c, id)?;
    require_phase(&s, from)?;
    if c.execute(
        "UPDATE deployments SET phase=?3 WHERE deployment_id=?1 AND phase=?2",
        params![id, from.as_str(), to.as_str()],
    )? != 1
    {
        return Err(JournalError::CorruptState);
    }
    Ok(())
}
fn advance(c: &mut Connection, id: &str, from: DeploymentPhase, to: DeploymentPhase) -> Result<()> {
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
    cas(&tx, id, from, to)?;
    tx.commit()?;
    Ok(())
}
fn begin_apply(c: &mut Connection, s: &DeploymentStatus, now_ms: u64) -> Result<ApplyWindow> {
    let deadline = now_ms
        .checked_add(
            s.request
                .confirmation_window_ms
                .unwrap_or(DEFAULT_CONFIRMATION_WINDOW_MS),
        )
        .filter(|v| *v <= i64::MAX as u64)
        .ok_or(JournalError::Invalid("confirmation deadline overflow"))?;
    if s.checkpoint_digest.is_empty() {
        return Err(JournalError::CorruptState);
    }
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
    cas(
        &tx,
        &s.request.deployment_id,
        DeploymentPhase::Checkpointed,
        DeploymentPhase::ProvisionalIntent,
    )?;
    tx.execute("UPDATE deployments SET expires_at_ms=?2,provisional_started_at_ms=?3,intent_marker=1 WHERE deployment_id=?1", params![s.request.deployment_id,deadline,now_ms])?;
    tx.commit()?;
    Ok(ApplyWindow {
        started_at_ms: now_ms,
        deadline_ms: deadline,
    })
}
fn confirmation_proof(
    s: &DeploymentStatus,
    evidence: &VerifiedDeploymentConfirmation,
    now_ms: u64,
) -> Result<ConfirmationProof> {
    let binding = DeploymentBinding {
        deployment_id: s.request.deployment_id.clone(),
        checkpoint_digest: s.checkpoint_digest.clone(),
    };
    if evidence.deployment() != &binding
        || evidence.device() != &s.request.device
        || evidence.revision() != &s.request.revision
        || evidence.plan_id() != s.request.plan_id
        || evidence.plan_epoch() != s.request.plan_epoch
        || evidence.graph_version() != s.request.graph_version
        || now_ms >= s.expires_at_ms
        || now_ms >= evidence.expires_at_ms()
        || now_ms < evidence.confirmed_at_ms()
        || evidence.confirmed_at_ms() >= s.expires_at_ms
        || Some(evidence.provisional_started_at_ms()) != s.provisional_started_at_ms
        || evidence.deadline_ms() != s.expires_at_ms
        || evidence.proof_digest().is_empty()
    {
        return Err(JournalError::Invalid(
            "confirmation does not match local checkpoint, apply interval, or current freshness",
        ));
    }
    Ok(ConfirmationProof {
        request_digest: request_digest(&s.request)?,
        checkpoint_digest: s.checkpoint_digest.clone(),
        proof_digest: evidence.proof_digest().into(),
        signed_proofs_json: serde_json::to_string(evidence.proofs())?,
        confirmed_at_ms: evidence.confirmed_at_ms(),
        accepted_at_ms: now_ms,
        expires_at_ms: evidence.expires_at_ms(),
        provisional_started_at_ms: evidence.provisional_started_at_ms(),
        deadline_ms: evidence.deadline_ms(),
    })
}
fn begin_confirmation(
    c: &mut Connection,
    s: &DeploymentStatus,
    proof: &ConfirmationProof,
) -> Result<()> {
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
    cas(
        &tx,
        &s.request.deployment_id,
        DeploymentPhase::AwaitingVerification,
        DeploymentPhase::ConfirmIntent,
    )?;
    tx.execute(
        "UPDATE deployments SET confirmation_proof=?2 WHERE deployment_id=?1",
        params![s.request.deployment_id, serde_json::to_string(proof)?],
    )?;
    tx.commit()?;
    Ok(())
}
fn validate_persisted_proof(c: &Connection, s: &DeploymentStatus) -> Result<()> {
    let raw: Option<String> = c.query_row(
        "SELECT confirmation_proof FROM deployments WHERE deployment_id=?1",
        [&s.request.deployment_id],
        |r| r.get(0),
    )?;
    let proof: ConfirmationProof =
        serde_json::from_str(raw.as_deref().ok_or(JournalError::CorruptState)?)?;
    if proof.request_digest != request_digest(&s.request)?
        || proof.checkpoint_digest != s.checkpoint_digest
        || proof.proof_digest != payload_digest(proof.signed_proofs_json.as_bytes())
        || proof.accepted_at_ms < proof.confirmed_at_ms
        || proof.confirmed_at_ms < proof.provisional_started_at_ms
        || proof.accepted_at_ms >= proof.expires_at_ms
        || proof.accepted_at_ms >= s.expires_at_ms
        || proof.deadline_ms != s.expires_at_ms
        || Some(proof.provisional_started_at_ms) != s.provisional_started_at_ms
        || !s.intent_marker
    {
        return Err(JournalError::CorruptState);
    }
    Ok(())
}
fn finish_confirmation(c: &mut Connection, id: &str) -> Result<()> {
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let s = bound_status(&tx, id)?;
    validate_persisted_proof(&tx, &s)?;
    cas(
        &tx,
        id,
        DeploymentPhase::ConfirmIntent,
        DeploymentPhase::Confirmed,
    )?;
    tx.execute(
        "UPDATE deployments SET rounds=?2 WHERE deployment_id=?1",
        params![id, REQUIRED_VERIFICATION_ROUNDS],
    )?;
    tx.commit()?;
    Ok(())
}
fn reconcile_locked<B: ApplyBackend>(
    c: &mut Connection,
    s: &DeploymentStatus,
    backend: &mut B,
) -> Result<DeploymentPhase> {
    let id = &s.request.deployment_id;
    bound_status(c, id)?;
    // Validate before the reconciliation call as well as before confirmation.
    if s.phase == DeploymentPhase::ConfirmIntent {
        validate_persisted_proof(c, s)?;
    }
    match backend.reconcile(id).map_err(backend_error)? {
        BackendDisposition::Confirmed { checkpoint_digest } => {
            if s.phase != DeploymentPhase::ConfirmIntent || checkpoint_digest != s.checkpoint_digest
            {
                return Err(JournalError::CorruptState);
            }
            finish_confirmation(c, id)?;
            Ok(DeploymentPhase::Confirmed)
        }
        BackendDisposition::AlreadyRolledBack => {
            advance(c, id, s.phase, DeploymentPhase::RolledBack)?;
            Ok(DeploymentPhase::RolledBack)
        }
        BackendDisposition::AppliedOrUnknown if s.phase == DeploymentPhase::ConfirmIntent => {
            Err(JournalError::UncertainConfirmation)
        }
        BackendDisposition::AppliedOrUnknown | BackendDisposition::Provisional => {
            rollback_locked(c, s, backend)?;
            Ok(DeploymentPhase::RolledBack)
        }
    }
}
fn rollback_locked<B: ApplyBackend>(
    c: &mut Connection,
    s: &DeploymentStatus,
    backend: &mut B,
) -> Result<()> {
    let id = &s.request.deployment_id;
    if !s.intent_marker {
        // Persist cleanup intent before an idempotent, no-live-write discard.
        // A lost reply remains retryable in RollingBack after reboot.
        if s.phase != DeploymentPhase::RollingBack {
            advance(c, id, s.phase, DeploymentPhase::RollingBack)?;
        }
        bound_status(c, id)?;
        backend.discard_staging(id).map_err(backend_error)?;
        return advance(
            c,
            id,
            DeploymentPhase::RollingBack,
            DeploymentPhase::RolledBack,
        );
    }
    if s.checkpoint_digest.is_empty() {
        return Err(JournalError::CorruptState);
    }
    if s.phase != DeploymentPhase::RollingBack {
        advance(c, id, s.phase, DeploymentPhase::RollingBack)?;
    }
    bound_status(c, id)?;
    backend
        .rollback(id, &s.checkpoint_digest)
        .map_err(backend_error)?;
    advance(
        c,
        id,
        DeploymentPhase::RollingBack,
        DeploymentPhase::RolledBack,
    )
}
fn validate_request(r: &DeploymentRequest) -> Result<()> {
    if r.deployment_id.is_empty()
        || r.plan_id.is_empty()
        || r.plan_digest.is_empty()
        || r.owned_baseline_digest.is_empty()
        || r.revision.id.is_empty()
        || r.revision.source_digest.is_empty()
        || r.device.device_id.is_empty()
        || r.device.profile_digest.is_empty()
        || r.owned_baseline.is_empty()
    {
        return Err(JournalError::Invalid(
            "missing immutable deployment binding",
        ));
    }
    if r.confirmation_window_ms == Some(0) {
        return Err(JournalError::Invalid("zero confirmation window"));
    }
    Ok(())
}
fn request_digest(r: &DeploymentRequest) -> Result<String> {
    Ok(payload_digest(&serde_json::to_vec(r)?))
}
fn json_sql(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}
fn migrate(c: &mut Connection) -> Result<()> {
    // Lock BEFORE reading the version, so concurrent initializers cannot both
    // decide to create/alter the same tables. DDL and version commit atomically.
    let tx = c.transaction_with_behavior(TransactionBehavior::Exclusive)?;
    let v: u32 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if v > SCHEMA_VERSION {
        return Err(JournalError::FutureSchema);
    }
    if v == 0 {
        tx.execute_batch("CREATE TABLE deployments(deployment_id TEXT PRIMARY KEY, device_id TEXT NOT NULL, immutable_digest TEXT NOT NULL, checkpoint_digest TEXT NOT NULL, request TEXT NOT NULL, phase TEXT NOT NULL, expires_at_ms INTEGER NOT NULL, rounds INTEGER NOT NULL, intent_marker INTEGER NOT NULL, created_at_ms INTEGER NOT NULL, provisional_started_at_ms INTEGER, confirmation_proof TEXT); CREATE TABLE device_fences(device_id TEXT PRIMARY KEY, fence INTEGER NOT NULL, epoch INTEGER NOT NULL, profile_digest TEXT NOT NULL);")?;
    } else if v == 1 {
        // Existing ambiguous active operations cannot be made safe by choosing
        // a winner. Refuse migration if the old journal already overlapped them.
        let overlaps: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM deployments WHERE phase NOT IN ('confirmed','rolled_back') GROUP BY json_extract(request,'$.device.device_id') HAVING count(*)>1)", [], |r| r.get(0))?;
        if overlaps {
            return Err(JournalError::ActiveDeployment);
        }
        tx.execute_batch("ALTER TABLE deployments ADD COLUMN device_id TEXT; ALTER TABLE deployments ADD COLUMN provisional_started_at_ms INTEGER; ALTER TABLE deployments ADD COLUMN confirmation_proof TEXT; ALTER TABLE device_fences ADD COLUMN profile_digest TEXT NOT NULL DEFAULT ''; UPDATE deployments SET device_id=json_extract(request,'$.device.device_id'); UPDATE device_fences SET profile_digest=COALESCE((SELECT json_extract(d.request,'$.device.profile_digest') FROM deployments d WHERE d.device_id=device_fences.device_id AND json_extract(d.request,'$.device.fencing_generation')=device_fences.fence AND json_extract(d.request,'$.plan_epoch')=device_fences.epoch ORDER BY created_at_ms DESC LIMIT 1),'');")?;
    }
    tx.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS one_active_deployment_per_device ON deployments(device_id) WHERE phase NOT IN ('confirmed','rolled_back');")?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests;
