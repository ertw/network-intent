//! Durable, controller-local assurance scheduling and evidence history.
//!
//! Signature verification happens before this boundary.  This store still
//! checks the *current* admitted plan, epoch, graph, probe and device binding
//! because a previously valid assignment can be stale after plan activation.
use crate::{
    admission::AdmittedRevision,
    health::{HealthError, HealthState, HealthTransition},
};
use intent_identity::{
    evidence::{probe_digest, VerifiedEvidence},
    DsseEnvelope,
};
use intent_protocol::{assurance::ProbeSpec, payload_digest};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};
use thiserror::Error;

pub const EVIDENCE_RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1_000;
pub const INCIDENT_RETENTION_MS: u64 = 365 * 24 * 60 * 60 * 1_000;
const SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("monitor mutex poisoned")]
    Poisoned,
    #[error("invalid monitor input")]
    Invalid,
    #[error("evidence ID was reused with different immutable contents")]
    ConflictingEvidenceId,
    #[error("evidence is not bound to the current enabled plan, graph, probe or device")]
    StaleScope,
    #[error("health state error: {0}")]
    Health(#[from] HealthError),
    #[error("corrupt monitor history")]
    CorruptHistory,
}
pub type Result<T> = std::result::Result<T, MonitorError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledProbe {
    pub plan_id: String,
    pub plan_epoch: u64,
    pub graph_version: u64,
    pub probe: ProbeSpec,
    pub due_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedProbe {
    pub scheduled: ScheduledProbe,
    pub lease_token: String,
    pub lease_until_ms: u64,
    pub lease_generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentEvent {
    pub evidence_id: String,
    pub transition: HealthTransition,
    pub at_ms: u64,
    pub state: HealthState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAuditRecord {
    pub evidence_id: String,
    pub evidence_envelope: DsseEnvelope,
    pub assignment_envelope: DsseEnvelope,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestOutcome {
    Inserted(HealthTransition),
    Duplicate,
}

/// Uses one SQLite connection behind a mutex: all mutations are serialized and
/// `BEGIN IMMEDIATE` makes schedule claims and evidence/health updates atomic.
pub struct MonitorStore {
    connection: Mutex<Connection>,
}
impl MonitorStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let mode: String = connection.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(MonitorError::Invalid);
        }
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Activates only a compiler-admitted revision; callers cannot supply an
    /// arbitrary success flag or unverified plan.
    pub fn activate(&self, admitted: &AdmittedRevision, now_ms: u64) -> Result<()> {
        self.activate_parts(admitted.plan(), admitted.revision(), now_ms)
    }
    fn activate_parts(
        &self,
        plan: &intent_protocol::assurance::AssurancePlan,
        revision: &intent_protocol::RevisionRef,
        now_ms: u64,
    ) -> Result<()> {
        if plan.id.is_empty() || plan.probes.is_empty() {
            return Err(MonitorError::Invalid);
        }
        let plan_digest = payload_digest(&serde_json::to_vec(plan)?);
        let mut c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let same_epoch: Option<String> = tx
            .query_row(
                "SELECT plan_digest FROM monitor_plans WHERE plan_id=?1 AND epoch=?2",
                params![plan.id, plan.epoch],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = same_epoch {
            return if existing == plan_digest {
                Ok(())
            } else {
                Err(MonitorError::Invalid)
            };
        }
        let previous: Option<(u64, u64)> = tx
            .query_row(
                "SELECT epoch, graph_version FROM monitor_plans WHERE plan_id=?1 ORDER BY epoch DESC LIMIT 1",
                [plan.id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if previous.is_some_and(|(epoch, graph)| plan.epoch <= epoch || plan.graph_version <= graph)
        {
            return Err(MonitorError::Invalid);
        }
        tx.execute(
            "UPDATE monitor_plans SET enabled = 0 WHERE plan_id = ?1",
            [plan.id.as_str()],
        )?;
        tx.execute("INSERT INTO monitor_plans(plan_id, epoch, graph_version, revision_id, revision_digest, plan_digest, enabled, activated_at_ms) VALUES(?1,?2,?3,?4,?5,?6,1,?7)", params![plan.id, plan.epoch, plan.graph_version, revision.id, revision.source_digest, plan_digest, now_ms])?;
        for probe in &plan.probes {
            if probe.id.is_empty() || probe.interval_ms == 0 || probe.limits.timeout_ms == 0 {
                return Err(MonitorError::Invalid);
            }
            tx.execute("INSERT INTO monitor_probes(plan_id, epoch, probe_id, graph_version, probe_json, probe_digest, due_at_ms, lease_token, lease_until_ms, attempts, lease_generation) VALUES(?1,?2,?3,?4,?5,?6,?7,NULL,NULL,0,0)",
                params![plan.id, plan.epoch, probe.id, plan.graph_version, serde_json::to_string(probe)?, probe_digest(probe)?, now_ms])?;
            let state = HealthState::new(
                plan.id.clone(),
                plan.epoch,
                plan.graph_version,
                probe.id.clone(),
            );
            tx.execute("INSERT INTO monitor_health(plan_id, epoch, probe_id, state_json) VALUES(?1,?2,?3,?4)", params![plan.id, plan.epoch, probe.id, serde_json::to_string(&state)?])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn due(&self, now_ms: u64, limit: usize) -> Result<Vec<ScheduledProbe>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        query_due(&c, now_ms, limit)
    }
    pub fn claim_due(
        &self,
        now_ms: u64,
        limit: usize,
        lease_ms: u64,
        worker: &str,
    ) -> Result<Vec<ClaimedProbe>> {
        if limit == 0 || lease_ms == 0 || worker.is_empty() || worker.len() > 256 {
            return Err(MonitorError::Invalid);
        }
        let mut c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let due = query_due_tx(&tx, now_ms, limit)?;
        let until = now_ms.saturating_add(lease_ms);
        let mut claimed = Vec::new();
        for scheduled in due {
            let changed = tx.execute("UPDATE monitor_probes SET lease_until_ms=?5, attempts=attempts+1, lease_generation=lease_generation+1 WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3 AND (lease_until_ms IS NULL OR lease_until_ms <= ?4)", params![scheduled.plan_id, scheduled.plan_epoch, scheduled.probe.id, now_ms, until])?;
            if changed == 1 {
                let generation: u64 = tx.query_row("SELECT lease_generation FROM monitor_probes WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3", params![scheduled.plan_id, scheduled.plan_epoch, scheduled.probe.id], |r| r.get(0))?;
                let token = format!("{worker}:{generation}");
                tx.execute("UPDATE monitor_probes SET lease_token=?4 WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3", params![scheduled.plan_id, scheduled.plan_epoch, scheduled.probe.id, token])?;
                claimed.push(ClaimedProbe {
                    scheduled,
                    lease_token: token,
                    lease_until_ms: until,
                    lease_generation: generation,
                });
            }
        }
        tx.commit()?;
        Ok(claimed)
    }
    pub fn finish(&self, claim: &ClaimedProbe, now_ms: u64) -> Result<bool> {
        self.release(claim, now_ms, 0)
    }
    pub fn retry(&self, claim: &ClaimedProbe, now_ms: u64, delay_ms: u64) -> Result<bool> {
        self.release(claim, now_ms, delay_ms)
    }
    fn release(&self, claim: &ClaimedProbe, now_ms: u64, delay_ms: u64) -> Result<bool> {
        let c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let due = now_ms.saturating_add(delay_ms.max(claim.scheduled.probe.interval_ms));
        Ok(c.execute("UPDATE monitor_probes SET due_at_ms=?7, lease_token=NULL, lease_until_ms=NULL WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3 AND lease_token=?4 AND lease_until_ms=?5 AND lease_generation=?6 AND lease_until_ms > ?8", params![claim.scheduled.plan_id, claim.scheduled.plan_epoch, claim.scheduled.probe.id, claim.lease_token, claim.lease_until_ms, claim.lease_generation, due, now_ms])? == 1)
    }

    /// Stores the immutable evidence and applies the health transition in the
    /// same transaction. A duplicate never advances a health streak twice.
    pub fn ingest(&self, verified: &VerifiedEvidence, now_ms: u64) -> Result<IngestOutcome> {
        let e = verified.evidence();
        let payload = serde_json::to_vec(e)?;
        // The idempotency identity uses the exact two signed payload strings,
        // rather than a reserialized typed witness record. Valid retransmits
        // with another signature therefore deduplicate, while changed signed
        // evidence or assignment bytes conflict.
        let digest = signed_payload_digest(verified);
        let evidence_envelope = serde_json::to_string(verified.envelope())?;
        let assignment_envelope = serde_json::to_string(verified.assignment_envelope())?;
        let mut c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT digest FROM monitor_evidence WHERE evidence_id=?1",
                [e.evidence_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return if existing == digest {
                Ok(IngestOutcome::Duplicate)
            } else {
                Err(MonitorError::ConflictingEvidenceId)
            };
        }
        let (graph, revision, stored_digest, probe_json): (u64, String, String, String) = tx.query_row(
            "SELECT p.graph_version, p.revision_id, p.revision_digest, q.probe_json FROM monitor_plans p JOIN monitor_probes q ON p.plan_id=q.plan_id AND p.epoch=q.epoch WHERE p.plan_id=?1 AND p.epoch=?2 AND p.enabled=1 AND q.probe_id=?3",
            params![e.plan_id, e.plan_epoch, e.result.probe_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .optional()?.ok_or(MonitorError::StaleScope)?;
        let probe: ProbeSpec = serde_json::from_str(&probe_json)?;
        if graph != e.graph_version
            || revision != e.revision.id
            || stored_digest != e.revision.source_digest
            || probe.device != e.device
            || probe_digest(&probe)? != e.probe_digest
        {
            return Err(MonitorError::StaleScope);
        }
        let state_json: String = tx.query_row(
            "SELECT state_json FROM monitor_health WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3",
            params![e.plan_id, e.plan_epoch, e.result.probe_id],
            |r| r.get(0),
        )?;
        let mut state: HealthState = serde_json::from_str(&state_json)?;
        let transition = state.observe(verified, now_ms)?;
        tx.execute("INSERT INTO monitor_evidence(evidence_id,digest,plan_id,epoch,graph_version,probe_id,device_id,profile_digest,fencing_generation,finished_at_ms,payload_json,evidence_envelope_json,assignment_envelope_json,received_at_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)", params![e.evidence_id,digest,e.plan_id,e.plan_epoch,e.graph_version,e.result.probe_id,e.device.device_id,e.device.profile_digest,e.device.fencing_generation,e.result.finished_at_ms,String::from_utf8(payload).map_err(|_| MonitorError::Invalid)?,evidence_envelope,assignment_envelope,now_ms])?;
        tx.execute(
            "UPDATE monitor_health SET state_json=?4 WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3",
            params![
                e.plan_id,
                e.plan_epoch,
                e.result.probe_id,
                serde_json::to_string(&state)?
            ],
        )?;
        if matches!(
            transition,
            HealthTransition::Opened | HealthTransition::Recovered
        ) {
            tx.execute("INSERT INTO monitor_incidents(evidence_id,transition,at_ms,state_json) VALUES(?1,?2,?3,?4)", params![e.evidence_id, format!("{transition:?}"), now_ms, serde_json::to_string(&state)?])?;
        }
        tx.commit()?;
        Ok(IngestOutcome::Inserted(transition))
    }
    pub fn health(&self, plan_id: &str, epoch: u64, probe_id: &str) -> Result<Option<HealthState>> {
        let c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let state: Option<String> = c.query_row(
            "SELECT state_json FROM monitor_health WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3",
            params![plan_id, epoch, probe_id],
            |r| r.get(0),
        )
        .optional()?;
        state
            .map(|json| serde_json::from_str(&json).map_err(MonitorError::from))
            .transpose()
    }
    /// Returns retained original signed envelopes for cryptographic audit; it
    /// never reconstructs an envelope from the typed payload cache.
    pub fn audit_evidence(&self, evidence_id: &str) -> Result<Option<EvidenceAuditRecord>> {
        let c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let row: Option<(String, String)> = c.query_row("SELECT evidence_envelope_json, assignment_envelope_json FROM monitor_evidence WHERE evidence_id=?1", [evidence_id], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
        row.map(|(evidence, assignment)| {
            if evidence.is_empty() || assignment.is_empty() {
                return Err(MonitorError::CorruptHistory);
            }
            Ok(EvidenceAuditRecord {
                evidence_id: evidence_id.into(),
                evidence_envelope: serde_json::from_str(&evidence)?,
                assignment_envelope: serde_json::from_str(&assignment)?,
            })
        })
        .transpose()
    }
    pub fn timeline(&self, limit: usize) -> Result<Vec<IncidentEvent>> {
        let c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let mut s = c.prepare("SELECT evidence_id,transition,at_ms,state_json FROM monitor_incidents ORDER BY id DESC LIMIT ?1")?;
        let rows = s
            .query_map([limit], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(evidence_id, transition, at_ms, state)| {
                let transition = match transition.as_str() {
                    "Opened" => HealthTransition::Opened,
                    "Recovered" => HealthTransition::Recovered,
                    _ => return Err(MonitorError::CorruptHistory),
                };
                Ok(IncidentEvent {
                    evidence_id,
                    transition,
                    at_ms,
                    state: serde_json::from_str(&state)?,
                })
            })
            .collect()
    }
    /// After an outage, expiry makes stale health unknown without closing an incident.
    pub fn reconcile_expirations(&self, now_ms: u64) -> Result<usize> {
        let mut c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut q = tx.prepare("SELECT plan_id,epoch,probe_id,state_json FROM monitor_health")?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, u64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(q);
        let mut changed = 0;
        for (plan, epoch, probe, json) in rows {
            let mut state: HealthState = serde_json::from_str(&json)?;
            let was = state.status;
            state.expire(now_ms);
            if state.status != was {
                tx.execute("UPDATE monitor_health SET state_json=?4 WHERE plan_id=?1 AND epoch=?2 AND probe_id=?3",params![plan,epoch,probe,serde_json::to_string(&state)?])?;
                changed += 1;
            }
        }
        tx.commit()?;
        Ok(changed)
    }
    pub fn purge_expired(&self, now_ms: u64) -> Result<()> {
        let c = self.connection.lock().map_err(|_| MonitorError::Poisoned)?;
        // Incident rows deliberately retain their evidence reference; do not
        // delete evidence that an incident/audit record still references.
        c.execute(
            "DELETE FROM monitor_incidents WHERE at_ms < ?1",
            [now_ms.saturating_sub(INCIDENT_RETENTION_MS)],
        )?;
        c.execute("DELETE FROM monitor_evidence WHERE received_at_ms < ?1 AND evidence_id NOT IN (SELECT evidence_id FROM monitor_incidents)", [now_ms.saturating_sub(EVIDENCE_RETENTION_MS)])?;
        Ok(())
    }
}

fn query_due(c: &Connection, now: u64, limit: usize) -> Result<Vec<ScheduledProbe>> {
    let mut s=c.prepare("SELECT q.plan_id,q.epoch,q.graph_version,q.probe_json,q.due_at_ms FROM monitor_probes q JOIN monitor_plans p ON p.plan_id=q.plan_id AND p.epoch=q.epoch WHERE p.enabled=1 AND q.due_at_ms<=?1 AND (q.lease_until_ms IS NULL OR q.lease_until_ms<=?1) ORDER BY q.due_at_ms,q.plan_id,q.probe_id LIMIT ?2")?;
    decode_due(&mut s, now, limit)
}
fn query_due_tx(
    c: &rusqlite::Transaction<'_>,
    now: u64,
    limit: usize,
) -> Result<Vec<ScheduledProbe>> {
    let mut s=c.prepare("SELECT q.plan_id,q.epoch,q.graph_version,q.probe_json,q.due_at_ms FROM monitor_probes q JOIN monitor_plans p ON p.plan_id=q.plan_id AND p.epoch=q.epoch WHERE p.enabled=1 AND q.due_at_ms<=?1 AND (q.lease_until_ms IS NULL OR q.lease_until_ms<=?1) ORDER BY q.due_at_ms,q.plan_id,q.probe_id LIMIT ?2")?;
    decode_due(&mut s, now, limit)
}
fn decode_due(
    s: &mut rusqlite::Statement<'_>,
    now: u64,
    limit: usize,
) -> Result<Vec<ScheduledProbe>> {
    s.query_map(params![now, limit], |r| {
        Ok(ScheduledProbe {
            plan_id: r.get(0)?,
            plan_epoch: r.get(1)?,
            graph_version: r.get(2)?,
            probe: serde_json::from_str(&r.get::<_, String>(3)?).map_err(json_sql)?,
            due_at_ms: r.get(4)?,
        })
    })?
    .collect::<std::result::Result<Vec<_>, _>>()
    .map_err(Into::into)
}
fn json_sql(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}
fn signed_payload_digest(verified: &VerifiedEvidence) -> String {
    let evidence = verified.envelope().payload.as_bytes();
    let assignment = verified.assignment_envelope().payload.as_bytes();
    let mut bytes = Vec::with_capacity(evidence.len() + assignment.len() + 17);
    bytes.extend_from_slice(&(evidence.len() as u64).to_be_bytes());
    bytes.extend_from_slice(evidence);
    bytes.extend_from_slice(&(assignment.len() as u64).to_be_bytes());
    bytes.extend_from_slice(assignment);
    payload_digest(&bytes)
}
fn migrate(connection: &mut Connection) -> Result<()> {
    // Lock before reading the version; concurrent starts and failed migrations
    // must not expose a partly upgraded schema.
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let c = &tx;
    let version: u32 = c.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(MonitorError::Invalid);
    };
    if version == 0 {
        c.execute_batch("CREATE TABLE monitor_plans(plan_id TEXT NOT NULL,epoch INTEGER NOT NULL,graph_version INTEGER NOT NULL,revision_id TEXT NOT NULL,revision_digest TEXT NOT NULL,plan_digest TEXT NOT NULL,enabled INTEGER NOT NULL,activated_at_ms INTEGER NOT NULL,PRIMARY KEY(plan_id,epoch)); CREATE TABLE monitor_probes(plan_id TEXT NOT NULL,epoch INTEGER NOT NULL,probe_id TEXT NOT NULL,graph_version INTEGER NOT NULL,probe_json TEXT NOT NULL,probe_digest TEXT NOT NULL,due_at_ms INTEGER NOT NULL,lease_token TEXT,lease_until_ms INTEGER,attempts INTEGER NOT NULL,lease_generation INTEGER NOT NULL,PRIMARY KEY(plan_id,epoch,probe_id),FOREIGN KEY(plan_id,epoch) REFERENCES monitor_plans(plan_id,epoch)); CREATE TABLE monitor_health(plan_id TEXT NOT NULL,epoch INTEGER NOT NULL,probe_id TEXT NOT NULL,state_json TEXT NOT NULL,PRIMARY KEY(plan_id,epoch,probe_id),FOREIGN KEY(plan_id,epoch,probe_id) REFERENCES monitor_probes(plan_id,epoch,probe_id)); CREATE TABLE monitor_evidence(evidence_id TEXT PRIMARY KEY,digest TEXT NOT NULL,plan_id TEXT NOT NULL,epoch INTEGER NOT NULL,graph_version INTEGER NOT NULL,probe_id TEXT NOT NULL,device_id TEXT NOT NULL,profile_digest TEXT NOT NULL,fencing_generation INTEGER NOT NULL,finished_at_ms INTEGER NOT NULL,payload_json TEXT NOT NULL,evidence_envelope_json TEXT NOT NULL,assignment_envelope_json TEXT NOT NULL,received_at_ms INTEGER NOT NULL); CREATE TABLE monitor_incidents(id INTEGER PRIMARY KEY,evidence_id TEXT NOT NULL,transition TEXT NOT NULL,at_ms INTEGER NOT NULL,state_json TEXT NOT NULL,FOREIGN KEY(evidence_id) REFERENCES monitor_evidence(evidence_id)); CREATE INDEX monitor_due ON monitor_probes(due_at_ms,lease_until_ms); CREATE INDEX monitor_evidence_retention ON monitor_evidence(received_at_ms); CREATE INDEX monitor_incident_retention ON monitor_incidents(at_ms); PRAGMA user_version=3;")?;
    } else if version == 1 {
        c.execute_batch("ALTER TABLE monitor_plans ADD COLUMN plan_digest TEXT NOT NULL DEFAULT ''; ALTER TABLE monitor_probes ADD COLUMN lease_generation INTEGER NOT NULL DEFAULT 0; PRAGMA user_version=2;")?;
        c.execute_batch("ALTER TABLE monitor_evidence ADD COLUMN evidence_envelope_json TEXT NOT NULL DEFAULT ''; ALTER TABLE monitor_evidence ADD COLUMN assignment_envelope_json TEXT NOT NULL DEFAULT ''; PRAGMA user_version=3;")?;
    } else if version == 2 {
        c.execute_batch("ALTER TABLE monitor_evidence ADD COLUMN evidence_envelope_json TEXT NOT NULL DEFAULT ''; ALTER TABLE monitor_evidence ADD COLUMN assignment_envelope_json TEXT NOT NULL DEFAULT ''; PRAGMA user_version=3;")?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthStatus;

    #[test]
    fn failed_migration_rolls_back_all_schema_changes() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE monitor_plans(plan_id TEXT); PRAGMA user_version=1;").unwrap();
        // The second ALTER fails because the deliberately damaged fixture has
        // no monitor_probes table. The first ALTER must be rolled back too.
        assert!(migrate(&mut connection).is_err());
        assert_eq!(connection.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0)).unwrap(), 1);
        assert!(connection.prepare("SELECT plan_digest FROM monitor_plans").is_err());
    }
    use intent_identity::{
        evidence::{verify_assignment, verify_evidence, AssignmentScope},
        AgentIdentity, DsseEnvelope, SigningKey, TrustedKey,
    };
    use intent_protocol::{
        assurance::{Expectation, Outcome, Primitive, ProbeResult, ProbeSource, ResourceLimits},
        evidence::{
            revision_subject, Statement, WitnessAssignment, WitnessEvidence, STATEMENT_TYPE,
            WITNESS_ASSIGNMENT_TYPE, WITNESS_EVIDENCE_TYPE,
        },
        state::Completeness,
        DeviceBinding, RevisionRef, PROTOCOL_VERSION,
    };
    use tempfile::tempdir;

    fn probe() -> ProbeSpec {
        ProbeSpec {
            id: "p".into(),
            primitive: Primitive::NetifdState,
            device: DeviceBinding {
                device_id: "d".into(),
                profile_digest: "profile".into(),
                fencing_generation: 7,
            },
            source: ProbeSource {
                witness_id: "spiffe://lab.local/agent/witness/d/w".into(),
                location: "lab".into(),
                bind_address: "192.0.2.1".parse().unwrap(),
                interface: None,
            },
            endpoint: None,
            expectation: Expectation::NetifdInterface {
                interface: "lan".into(),
                up: true,
                addresses: vec![],
            },
            limits: ResourceLimits {
                timeout_ms: 1_000,
                max_response_bytes: 1_024,
                max_attempts: 1,
            },
            interval_ms: 15_000,
            depends_on: vec![],
            claims: vec![],
        }
    }
    fn trusted(identity: &AgentIdentity, bytes: &[u8]) -> TrustedKey {
        TrustedKey {
            identity: identity.clone(),
            public_key: SigningKey::from_pkcs8(bytes).unwrap().public_key(),
            not_before_ms: 0,
            not_after_ms: 20_000,
            revoked: false,
        }
    }
    fn signed<T: serde::Serialize>(key: &SigningKey, statement: &Statement<T>) -> DsseEnvelope {
        key.sign(&serde_json::to_vec(statement).unwrap()).unwrap()
    }
    fn verified(n: u64, outcome: Outcome) -> intent_identity::evidence::VerifiedEvidence {
        let controller = AgentIdentity::parse("spiffe://lab.local/agent/controller/d/c").unwrap();
        let witness = AgentIdentity::parse("spiffe://lab.local/agent/witness/d/w").unwrap();
        let (controller_key, controller_bytes) = SigningKey::generate().unwrap();
        let (witness_key, witness_bytes) = SigningKey::generate().unwrap();
        let device = DeviceBinding {
            device_id: "d".into(),
            profile_digest: "profile".into(),
            fencing_generation: 7,
        };
        let revision = RevisionRef {
            id: "rev".into(),
            source_digest: "digest".into(),
        };
        let probe = probe();
        let assignment = WitnessAssignment {
            version: PROTOCOL_VERSION,
            assignment_id: "a".into(),
            issuer: controller.to_string(),
            recipient: witness.to_string(),
            issued_at_ms: 1_000,
            expires_at_ms: 10_000,
            plan_id: "plan".into(),
            plan_epoch: 1,
            graph_version: 2,
            revision: revision.clone(),
            device: device.clone(),
            probes: vec![probe.clone()],
        };
        let addresses = ["192.0.2.1".parse().unwrap()];
        let scope = AssignmentScope {
            controller: &controller,
            witness: &witness,
            device: &device,
            revision: &revision,
            plan_id: "plan",
            plan_epoch: 1,
            graph_version: 2,
            location: "lab",
            bind_addresses: &addresses,
        };
        let assignment = verify_assignment(
            &signed(
                &controller_key,
                &Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&revision),
                    predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                    predicate: assignment,
                },
            ),
            &[trusted(&controller, &controller_bytes)],
            &scope,
            5_000,
        )
        .unwrap();
        let evidence = WitnessEvidence {
            version: PROTOCOL_VERSION,
            evidence_id: format!("e{n}"),
            assignment_id: "a".into(),
            plan_id: "plan".into(),
            plan_epoch: 1,
            graph_version: 2,
            revision,
            device,
            witness: witness.to_string(),
            probe_digest: probe_digest(&probe).unwrap(),
            completeness: Completeness::Complete,
            result: ProbeResult {
                probe_id: "p".into(),
                outcome,
                started_at_ms: 1_500 + n * 100,
                finished_at_ms: 1_550 + n * 100,
                detail: String::new(),
            },
        };
        verify_evidence(
            &signed(
                &witness_key,
                &Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&evidence.revision),
                    predicate_type: WITNESS_EVIDENCE_TYPE.into(),
                    predicate: evidence,
                },
            ),
            &[trusted(&witness, &witness_bytes)],
            &assignment,
            5_000,
        )
        .unwrap()
    }
    // Test-only SQL fixture: production can only create schedules through an
    // `AdmittedRevision`, whose constructor is private to compiler admission.
    fn seed(store: &MonitorStore, now: u64) {
        let p = probe();
        let c = store.connection.lock().unwrap();
        c.execute(
            "INSERT INTO monitor_plans VALUES('plan',1,2,'rev','digest','fixture',1,?1)",
            [now],
        )
        .unwrap();
        c.execute(
            "INSERT INTO monitor_probes VALUES('plan',1,'p',2,?1,?2,?3,NULL,NULL,0,0)",
            params![
                serde_json::to_string(&p).unwrap(),
                probe_digest(&p).unwrap(),
                now
            ],
        )
        .unwrap();
        c.execute(
            "INSERT INTO monitor_health VALUES('plan',1,'p',?1)",
            [serde_json::to_string(&HealthState::new("plan".into(), 1, 2, "p".into())).unwrap()],
        )
        .unwrap();
    }
    #[test]
    fn schedules_and_expired_leases_survive_reopen_without_double_claiming() {
        let d = tempdir().unwrap();
        let path = d.path().join("monitor.sqlite");
        let store = MonitorStore::open(&path).unwrap();
        seed(&store, 100);
        let claim = store.claim_due(100, 2, 20, "worker-a").unwrap();
        assert_eq!(claim.len(), 1);
        assert!(store.claim_due(101, 2, 20, "worker-b").unwrap().is_empty());
        drop(store);
        let store = MonitorStore::open(&path).unwrap();
        let replacement = store.claim_due(120, 1, 20, "worker-b").unwrap();
        assert_eq!(replacement.len(), 1);
        assert!(!store.finish(&claim[0], 120).unwrap());
        assert!(store.finish(&replacement[0], 121).unwrap());
    }
    #[test]
    fn reconcile_marks_health_unknown_after_controller_outage() {
        let d = tempdir().unwrap();
        let store = MonitorStore::open(d.path().join("monitor.sqlite")).unwrap();
        seed(&store, 0);
        {
            let c = store.connection.lock().unwrap();
            let mut state = HealthState::new("plan".into(), 1, 2, "p".into());
            state.status = HealthStatus::Healthy;
            state.freshness_until_ms = Some(10);
            c.execute(
                "UPDATE monitor_health SET state_json=?1",
                [serde_json::to_string(&state).unwrap()],
            )
            .unwrap();
        }
        assert_eq!(store.reconcile_expirations(11).unwrap(), 1);
        assert_eq!(
            store.health("plan", 1, "p").unwrap().unwrap().status,
            HealthStatus::Unknown
        );
    }
    #[test]
    fn failed_foreign_key_write_rolls_back_transaction() {
        let d = tempdir().unwrap();
        let store = MonitorStore::open(d.path().join("monitor.sqlite")).unwrap();
        let mut c = store.connection.lock().unwrap();
        let tx = c.transaction().unwrap();
        assert!(tx
            .execute(
                "INSERT INTO monitor_health VALUES('missing',1,'p','{}')",
                []
            )
            .is_err());
        drop(tx);
        let n: i64 = c
            .query_row("SELECT count(*) FROM monitor_health", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
    #[test]
    fn exact_evidence_replay_deduplicates_conflict_and_rollback_do_not_advance_health() {
        let d = tempdir().unwrap();
        let store = MonitorStore::open(d.path().join("monitor.sqlite")).unwrap();
        seed(&store, 0);
        let first = verified(1, Outcome::Violation);
        assert_eq!(
            store.ingest(&first, 5_000).unwrap(),
            IngestOutcome::Inserted(HealthTransition::Updated)
        );
        let before = store.health("plan", 1, "p").unwrap().unwrap();
        assert_eq!(
            store.ingest(&first, 5_000).unwrap(),
            IngestOutcome::Duplicate
        );
        assert_eq!(store.health("plan", 1, "p").unwrap().unwrap(), before);
        assert!(matches!(
            store.ingest(&verified(1, Outcome::Success), 5_000),
            Err(MonitorError::ConflictingEvidenceId)
        ));
        {
            let c = store.connection.lock().unwrap();
            c.execute_batch("CREATE TRIGGER reject_health BEFORE UPDATE ON monitor_health BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
        }
        assert!(matches!(
            store.ingest(&verified(2, Outcome::Violation), 5_000),
            Err(MonitorError::Sql(_))
        ));
        let c = store.connection.lock().unwrap();
        let evidence: i64 = c
            .query_row("SELECT count(*) FROM monitor_evidence", [], |r| r.get(0))
            .unwrap();
        assert_eq!(evidence, 1);
        drop(c);
        assert_eq!(store.health("plan", 1, "p").unwrap().unwrap(), before);
    }
    #[test]
    fn activation_is_immutable_idempotent_and_monotonic() {
        let d = tempdir().unwrap();
        let store = MonitorStore::open(d.path().join("monitor.sqlite")).unwrap();
        let revision = RevisionRef {
            id: "rev".into(),
            source_digest: "digest".into(),
        };
        let mut plan = intent_protocol::assurance::AssurancePlan {
            version: 1,
            id: "activate".into(),
            epoch: 2,
            revision: revision.clone(),
            graph_version: 3,
            profiles: vec![],
            claims: vec![],
            probes: vec![probe()],
        };
        store.activate_parts(&plan, &revision, 0).unwrap();
        {
            let c = store.connection.lock().unwrap();
            let mut h = HealthState::new("activate".into(), 2, 3, "p".into());
            h.status = HealthStatus::Degraded;
            c.execute(
                "UPDATE monitor_health SET state_json=?1",
                [serde_json::to_string(&h).unwrap()],
            )
            .unwrap();
        }
        store.activate_parts(&plan, &revision, 1).unwrap();
        assert_eq!(
            store.health("activate", 2, "p").unwrap().unwrap().status,
            HealthStatus::Degraded
        );
        plan.graph_version = 4;
        assert!(matches!(
            store.activate_parts(&plan, &revision, 2),
            Err(MonitorError::Invalid)
        ));
        let mut older = plan.clone();
        older.epoch = 1;
        older.graph_version = 4;
        assert!(matches!(
            store.activate_parts(&older, &revision, 2),
            Err(MonitorError::Invalid)
        ));
        let mut next = plan;
        next.epoch = 3;
        next.graph_version = 4;
        store.activate_parts(&next, &revision, 3).unwrap();
        assert!(store.health("activate", 3, "p").unwrap().is_some());
    }
    #[test]
    fn corrupt_timeline_transition_is_rejected() {
        let d = tempdir().unwrap();
        let store = MonitorStore::open(d.path().join("monitor.sqlite")).unwrap();
        seed(&store, 0);
        let v = verified(1, Outcome::Violation);
        store.ingest(&v, 5_000).unwrap();
        {
            let c = store.connection.lock().unwrap();
            c.execute("INSERT INTO monitor_incidents(evidence_id,transition,at_ms,state_json) VALUES('e1','nonsense',1,'{}')",[]).unwrap();
        }
        assert!(matches!(
            store.timeline(10),
            Err(MonitorError::CorruptHistory)
        ));
    }
    #[test]
    fn original_signed_envelopes_survive_reopen_for_audit() {
        let d = tempdir().unwrap();
        let path = d.path().join("monitor.sqlite");
        let store = MonitorStore::open(&path).unwrap();
        seed(&store, 0);
        let verified = verified(1, Outcome::Violation);
        let evidence = verified.envelope().clone();
        let assignment = verified.assignment_envelope().clone();
        store.ingest(&verified, 5_000).unwrap();
        drop(store);
        let store = MonitorStore::open(&path).unwrap();
        let audit = store.audit_evidence("e1").unwrap().unwrap();
        assert_eq!(audit.evidence_envelope, evidence);
        assert_eq!(audit.assignment_envelope, assignment);
    }
}
