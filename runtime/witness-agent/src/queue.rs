//! Witness-local durable work, fenced independently of controller storage.
use crate::execution::{ExecutionError, WitnessExecutor};
use intent_identity::{
    DsseEnvelope, TrustedKey,
    evidence::{AssignmentScope, EvidenceError, VerifiedAssignment, verify_assignment},
};
use intent_protocol::{MessageEnvelope, PROTOCOL_VERSION, payload_digest};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use std::{path::Path, sync::Mutex, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("verification: {0}")]
    Evidence(#[from] EvidenceError),
    #[error("execution: {0}")]
    Execution(#[from] ExecutionError),
    #[error("invalid queue configuration, assignment or work lease")]
    Invalid,
    #[error("message ID reused with different immutable contents")]
    Conflict,
    #[error("queue full")]
    Full,
    #[error("stale device fence or plan epoch")]
    Stale,
    #[error("assignment envelope does not exactly match message payload")]
    Payload,
    #[error("mutex poisoned")]
    Poisoned,
}
pub type Result<T> = std::result::Result<T, QueueError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Receive {
    Inserted,
    Duplicate,
}
#[derive(Debug, Clone)]
pub struct Work {
    pub message_id: String,
    pub probe_id: String,
    pub lease: String,
}
#[derive(Debug, Clone)]
pub struct Delivery {
    pub evidence_id: String,
    pub envelope: DsseEnvelope,
    pub attempts: u32,
}
pub struct WitnessQueue {
    c: Mutex<Connection>,
    max: usize,
}

// All immutable route fields are included. Local address authorization is also
// verified before claiming: a scope may authorize only a subset of addresses.
fn scope_key(scope: &AssignmentScope<'_>) -> Result<String> {
    Ok(payload_digest(&serde_json::to_vec(&(
        scope.controller.to_string(),
        scope.witness.to_string(),
        scope.device,
        scope.revision,
        scope.plan_id,
        scope.plan_epoch,
        scope.graph_version,
        scope.location,
    ))?))
}
fn assigned_key(a: &VerifiedAssignment) -> Result<String> {
    let a = a.assignment();
    Ok(payload_digest(&serde_json::to_vec(&(
        &a.issuer,
        &a.recipient,
        &a.device,
        &a.revision,
        &a.plan_id,
        a.plan_epoch,
        a.graph_version,
        &a.probes[0].source.location,
    ))?))
}
fn retire(tx: &Transaction<'_>, now: u64) -> Result<()> {
    // Keep immutable assignments and evidence forever as deduplication/audit
    // tombstones, but retire work that can no longer be executed or delivered.
    tx.execute("UPDATE jobs SET status='retired',lease=NULL WHERE status IN ('pending','running','executing') AND message_id IN (SELECT a.message_id FROM assignments a JOIN fences f ON f.device=a.device WHERE a.expires<=?1 OR a.fence!=f.fence OR a.epoch!=f.epoch)",[now])?;
    tx.execute("UPDATE outbox SET retired=1 WHERE retired=0 AND message_id IN (SELECT a.message_id FROM assignments a JOIN fences f ON f.device=a.device WHERE a.expires<=?1 OR a.fence!=f.fence OR a.epoch!=f.epoch)",[now])?;
    Ok(())
}
fn live(
    tx: &Transaction<'_>,
    work: &Work,
    scope: &AssignmentScope<'_>,
    now: u64,
    status: &str,
) -> Result<String> {
    tx.query_row("SELECT a.assignment FROM jobs j JOIN assignments a USING(message_id) JOIN fences f ON f.device=a.device WHERE j.message_id=?1 AND j.probe_id=?2 AND j.lease=?3 AND j.status=?4 AND a.route=?5 AND a.expires>?6 AND a.fence=f.fence AND a.epoch=f.epoch",params![work.message_id,work.probe_id,work.lease,status,scope_key(scope)?,now],|r|r.get(0)).optional()?.ok_or(QueueError::Stale)
}
impl WitnessQueue {
    pub fn open(path: impl AsRef<Path>, max: usize) -> Result<Self> {
        if max == 0 || max > i64::MAX as usize {
            return Err(QueueError::Invalid);
        }
        let mut c = Connection::open(path)?;
        c.busy_timeout(Duration::from_secs(5))?;
        c.pragma_update(None, "foreign_keys", true)?;
        let wal: String = c.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if !wal.eq_ignore_ascii_case("wal") {
            return Err(QueueError::Invalid);
        }
        c.pragma_update(None, "synchronous", "FULL")?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match version {
            0 => tx.execute_batch("CREATE TABLE assignments(message_id TEXT PRIMARY KEY,envelope TEXT NOT NULL,assignment TEXT NOT NULL,device TEXT NOT NULL,fence INTEGER NOT NULL,epoch INTEGER NOT NULL,expires INTEGER NOT NULL,received INTEGER NOT NULL,route TEXT NOT NULL);
                CREATE TABLE jobs(message_id TEXT NOT NULL REFERENCES assignments(message_id),probe_id TEXT NOT NULL,status TEXT NOT NULL CHECK(status IN ('pending','running','executing','done','retired')),lease TEXT,PRIMARY KEY(message_id,probe_id));
                CREATE TABLE outbox(evidence_id TEXT PRIMARY KEY,message_id TEXT NOT NULL,probe_id TEXT NOT NULL,envelope TEXT NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,acked INTEGER,retired INTEGER NOT NULL DEFAULT 0,FOREIGN KEY(message_id,probe_id) REFERENCES jobs(message_id,probe_id));
                CREATE TABLE fences(device TEXT PRIMARY KEY,fence INTEGER NOT NULL,epoch INTEGER NOT NULL);
                CREATE TABLE generations(id INTEGER PRIMARY KEY CHECK(id=1),value INTEGER NOT NULL);
                INSERT INTO generations VALUES(1,0);
                CREATE INDEX pending_route ON assignments(route,received);
                PRAGMA user_version=1;")?,
            1 => (),
            _ => return Err(QueueError::Invalid),
        }
        tx.commit()?;
        Ok(Self {
            c: Mutex::new(c),
            max,
        })
    }
    pub fn receive(
        &self,
        sealed: &VerifiedAssignment,
        envelope: &MessageEnvelope,
        now: u64,
    ) -> Result<Receive> {
        let a = sealed.assignment();
        if envelope.version != PROTOCOL_VERSION
            || envelope.message_id.is_empty()
            || envelope.message_id.len() > 256
            || envelope.message_id.chars().any(char::is_control)
            || envelope.created_at_ms > now
            || now >= envelope.expires_at_ms
            || a.issued_at_ms > now
            || now >= a.expires_at_ms
            || payload_digest(&envelope.payload) != envelope.payload_digest
            || envelope.sender != a.issuer
            || envelope.recipient != a.recipient
            || envelope.device != a.device
            || envelope.plan_epoch != a.plan_epoch
        {
            return Err(QueueError::Invalid);
        }
        let dsse: DsseEnvelope = serde_json::from_slice(&envelope.payload)?;
        if dsse != *sealed.envelope() {
            return Err(QueueError::Payload);
        }
        let mut c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let encoded = serde_json::to_string(envelope)?;
        let old: Option<String> = tx
            .query_row(
                "SELECT envelope FROM assignments WHERE message_id=?1",
                [&envelope.message_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            return if old == encoded {
                Ok(Receive::Duplicate)
            } else {
                Err(QueueError::Conflict)
            };
        }
        let fence: Option<(u64, u64)> = tx
            .query_row(
                "SELECT fence,epoch FROM fences WHERE device=?1",
                [&a.device.device_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if fence.is_some_and(|(f, e)| (a.device.fencing_generation, a.plan_epoch) < (f, e)) {
            return Err(QueueError::Stale);
        }
        tx.execute("INSERT INTO fences VALUES(?1,?2,?3) ON CONFLICT(device) DO UPDATE SET fence=excluded.fence,epoch=excluded.epoch",params![a.device.device_id,a.device.fencing_generation,a.plan_epoch])?;
        retire(&tx, now)?;
        let count:i64=tx.query_row("SELECT (SELECT count(*) FROM jobs WHERE status IN ('pending','running','executing'))+(SELECT count(*) FROM outbox WHERE acked IS NULL AND retired=0)",[],|r|r.get(0))?;
        if count as usize > self.max.saturating_sub(a.probes.len()) || a.probes.len() > self.max {
            return Err(QueueError::Full);
        }
        tx.execute(
            "INSERT INTO assignments VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                envelope.message_id,
                encoded,
                serde_json::to_string(&dsse)?,
                a.device.device_id,
                a.device.fencing_generation,
                a.plan_epoch,
                a.expires_at_ms.min(envelope.expires_at_ms),
                now,
                assigned_key(sealed)?
            ],
        )?;
        for p in &a.probes {
            tx.execute(
                "INSERT INTO jobs VALUES(?1,?2,'pending',NULL)",
                params![envelope.message_id, p.id],
            )?;
        }
        tx.commit()?;
        Ok(Receive::Inserted)
    }
    /// Recovery clears every prior token; the persisted generation never resets.
    pub fn recover(&self) -> Result<usize> {
        let c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        Ok(c.execute(
            "UPDATE jobs SET status='pending',lease=NULL WHERE status IN ('running','executing')",
            [],
        )?)
    }
    pub fn claim(
        &self,
        now: u64,
        scope: &AssignmentScope<'_>,
        keys: &[TrustedKey],
    ) -> Result<Option<Work>> {
        let mut c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        retire(&tx, now)?;
        let rows = {
            let mut s=tx.prepare("SELECT j.message_id,j.probe_id,a.assignment FROM jobs j JOIN assignments a USING(message_id) JOIN fences f ON f.device=a.device WHERE j.status='pending' AND a.route=?1 AND a.expires>?2 AND a.fence=f.fence AND a.epoch=f.epoch ORDER BY a.received,j.message_id,j.probe_id")?;
            let rows = s
                .query_map(params![scope_key(scope)?, now], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        for (id, p, raw) in rows {
            let dsse: DsseEnvelope = serde_json::from_str(&raw)?;
            if verify_assignment(&dsse, keys, scope, now).is_err() {
                continue;
            }
            let generation:i64=tx.query_row("UPDATE generations SET value=value+1 WHERE id=1 AND value<9223372036854775807 RETURNING value",[],|r|r.get(0))?;
            let lease = payload_digest(&serde_json::to_vec(&(
                "witness-claim-v1",
                &id,
                &p,
                generation,
            ))?);
            tx.execute(
                "UPDATE jobs SET status='running',lease=?3 WHERE message_id=?1 AND probe_id=?2",
                params![id, p, lease],
            )?;
            tx.commit()?;
            return Ok(Some(Work {
                message_id: id,
                probe_id: p,
                lease,
            }));
        }
        tx.commit()?;
        Ok(None)
    }
    pub async fn execute(
        &self,
        work: &Work,
        executor: &WitnessExecutor,
        scope: &AssignmentScope<'_>,
        keys: &[TrustedKey],
        now: u64,
    ) -> Result<String> {
        self.execute_using(work, scope, keys, now, |assigned, id| async move {
            executor.run_probe(&assigned, &work.probe_id, &id).await
        })
        .await
    }
    // The observation callback is private so tests can deterministically exercise
    // invalidation during an observation without performing network operations.
    async fn execute_using<F, Fut>(
        &self,
        work: &Work,
        scope: &AssignmentScope<'_>,
        keys: &[TrustedKey],
        now: u64,
        observe: F,
    ) -> Result<String>
    where
        F: FnOnce(VerifiedAssignment, String) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<DsseEnvelope, ExecutionError>>,
    {
        // Consume the live lease BEFORE calling the executor, including for a
        // second concurrent invocation with an otherwise legitimate Work.
        let assigned = {
            let mut c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let now = now.max(crate::now_ms());
            let raw = live(&tx, work, scope, now, "running")?;
            let dsse: DsseEnvelope = serde_json::from_str(&raw)?;
            let assigned = verify_assignment(&dsse, keys, scope, now)?;
            tx.execute(
                "UPDATE jobs SET status='executing' WHERE message_id=?1 AND probe_id=?2",
                params![work.message_id, work.probe_id],
            )?;
            tx.commit()?;
            assigned
        };
        let id = payload_digest(&serde_json::to_vec(&("witness-evidence-v1", &work.lease))?);
        let out = observe(assigned, id.clone()).await?;
        self.complete(work, scope, now.max(crate::now_ms()), &id, &out)?;
        Ok(id)
    }
    fn complete(
        &self,
        work: &Work,
        scope: &AssignmentScope<'_>,
        now: u64,
        id: &str,
        out: &DsseEnvelope,
    ) -> Result<()> {
        let mut c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        live(&tx, work, scope, now, "executing")?;
        tx.execute(
            "UPDATE jobs SET status='done',lease=NULL WHERE message_id=?1 AND probe_id=?2",
            params![work.message_id, work.probe_id],
        )?;
        tx.execute(
            "INSERT INTO outbox(evidence_id,message_id,probe_id,envelope) VALUES(?1,?2,?3,?4)",
            params![
                id,
                work.message_id,
                work.probe_id,
                serde_json::to_string(out)?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn deliveries(
        &self,
        now: u64,
        scope: &AssignmentScope<'_>,
        keys: &[TrustedKey],
        limit: usize,
    ) -> Result<Vec<Delivery>> {
        let mut c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        retire(&tx, now)?;
        let rows = {
            let mut s=tx.prepare("SELECT o.evidence_id,o.envelope,o.attempts,a.assignment FROM outbox o JOIN assignments a USING(message_id) JOIN fences f ON f.device=a.device WHERE o.acked IS NULL AND o.retired=0 AND a.route=?1 AND a.expires>?2 AND a.fence=f.fence AND a.epoch=f.epoch ORDER BY o.evidence_id")?;
            let rows = s
                .query_map(params![scope_key(scope)?, now], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, u32>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        let mut deliveries = Vec::new();
        for (evidence_id, raw, attempts, assignment) in rows {
            if deliveries.len() >= limit {
                break;
            }
            let dsse: DsseEnvelope = serde_json::from_str(&assignment)?;
            if verify_assignment(&dsse, keys, scope, now).is_err() {
                continue;
            }
            deliveries.push(Delivery {
                evidence_id,
                envelope: serde_json::from_str(&raw)?,
                attempts,
            });
        }
        tx.commit()?;
        Ok(deliveries)
    }
    pub fn retry(&self, id: &str) -> Result<bool> {
        let c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        Ok(c.execute("UPDATE outbox SET attempts=attempts+1 WHERE evidence_id=?1 AND acked IS NULL AND retired=0 AND attempts<4294967295",[id])?==1)
    }
    pub fn acknowledge(&self, id: &str) -> Result<bool> {
        let c = self.c.lock().map_err(|_| QueueError::Poisoned)?;
        Ok(c.execute(
            "UPDATE outbox SET acked=1 WHERE evidence_id=?1 AND acked IS NULL",
            [id],
        )? == 1)
    }
}

#[cfg(test)]
mod tests {
    // Share the real signed fixtures and public API regression tests with the
    // integration target; this module additionally exercises private transitions.
    use crate as intent_witness_agent;
    include!("../tests/queue.rs");

    #[test]
    fn every_connection_enforces_foreign_keys_and_busy_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let q = WitnessQueue::open(dir.path().join("q"), 1).unwrap();
        let c = q.c.lock().unwrap();
        assert_eq!(
            c.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5000
        );
        assert_eq!(
            c.query_row("PRAGMA synchronous", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn invalid_claims_never_invoke_observation() {
        let mut f = F::new();
        let dir = tempfile::tempdir().unwrap();
        let q = WitnessQueue::open(dir.path().join("q"), 2).unwrap();
        q.receive(&f.verified(), &f.msg("old"), now()).unwrap();
        let old = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
        let calls = std::cell::Cell::new(0);
        let mut forged = old.clone();
        forged.lease = "forged".into();
        let result = q
            .execute_using(&forged, &f.scope(), &f.keys, now(), |_, _| async {
                calls.set(calls.get() + 1);
                Err(crate::execution::ExecutionError::Scope)
            })
            .await;
        assert!(matches!(result, Err(QueueError::Stale)));
        q.recover().unwrap();
        let result = q
            .execute_using(&old, &f.scope(), &f.keys, now(), |_, _| async {
                calls.set(calls.get() + 1);
                Err(crate::execution::ExecutionError::Scope)
            })
            .await;
        assert!(matches!(result, Err(QueueError::Stale)));
        let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
        f.advance(5, 1);
        q.receive(&f.verified(), &f.msg("new"), now()).unwrap();
        f.advance(4, 1);
        let result = q
            .execute_using(&work, &f.scope(), &f.keys, now(), |_, _| async {
                calls.set(calls.get() + 1);
                Err(crate::execution::ExecutionError::Scope)
            })
            .await;
        assert!(matches!(result, Err(QueueError::Stale)));
        assert_eq!(calls.get(), 0);
    }

    #[tokio::test]
    async fn recovery_or_fence_change_during_observation_prevents_completion() {
        for recover in [true, false] {
            let f = F::new();
            let dir = tempfile::tempdir().unwrap();
            let q = WitnessQueue::open(dir.path().join("q"), 1).unwrap();
            q.receive(&f.verified(), &f.msg("old"), now()).unwrap();
            let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
            let calls = std::cell::Cell::new(0);
            let (q, f, work, calls) = (&q, &f, &work, &calls);
            let result = q
                .execute_using(
                    &work,
                    &f.scope(),
                    &f.keys,
                    now(),
                    |assigned, id| async move {
                        calls.set(calls.get() + 1);
                        // Reusing the same live token concurrently is rejected before
                        // invoking a second observation, even before recovery/fencing.
                        let duplicate = q
                            .execute_using(&work, &f.scope(), &f.keys, now(), |_, _| async {
                                panic!("duplicate observation must not run")
                            })
                            .await;
                        assert!(matches!(duplicate, Err(QueueError::Stale)));
                        let out = f
                            .executor()
                            .run_probe(&assigned, &work.probe_id, &id)
                            .await?;
                        if recover {
                            q.recover().unwrap();
                        } else {
                            let mut newer = F::new();
                            newer.advance(5, 1);
                            q.receive(&newer.verified(), &newer.msg("newer"), now())
                                .unwrap();
                        }
                        Ok(out)
                    },
                )
                .await;
            assert!(matches!(result, Err(QueueError::Stale)));
            assert_eq!(calls.get(), 1);
            assert!(
                q.deliveries(now(), &f.scope(), &f.keys, 10)
                    .unwrap()
                    .is_empty()
            );
            let c = q.c.lock().unwrap();
            assert_eq!(
                c.query_row("SELECT count(*) FROM outbox", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

    #[tokio::test]
    async fn failed_outbox_insert_rolls_back_job_completion() {
        let f = F::new();
        let dir = tempfile::tempdir().unwrap();
        let q = WitnessQueue::open(dir.path().join("q"), 1).unwrap();
        q.receive(&f.verified(), &f.msg("m"), now()).unwrap();
        let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
        q.c.lock().unwrap().execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON outbox BEGIN SELECT RAISE(ABORT,'test disk/storage failure'); END;").unwrap();
        assert!(matches!(
            q.execute(&work, &f.executor(), &f.scope(), &f.keys, now())
                .await,
            Err(QueueError::Sql(_))
        ));
        let c = q.c.lock().unwrap();
        assert_eq!(
            c.query_row("SELECT status FROM jobs", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "executing"
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM outbox", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
