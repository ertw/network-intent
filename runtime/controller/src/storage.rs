use intent_protocol::{payload_digest, MessageEnvelope, PROTOCOL_VERSION};
use rusqlite::{backup::Backup, params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use thiserror::Error;

pub const OBSERVATION_RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1_000;
pub const AUDIT_RETENTION_MS: u64 = 365 * 24 * 60 * 60 * 1_000;
const SCHEMA_VERSION: u32 = 4;
const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("store mutex poisoned")]
    Poisoned,
    #[error("invalid message: {0}")]
    InvalidMessage(&'static str),
    #[error("message ID {0} was reused with different immutable contents")]
    ConflictingMessageId(String),
    #[error(
        "stale fencing generation for device {device_id}: received {received}, current {current}"
    )]
    StaleFence {
        device_id: String,
        received: u64,
        current: u64,
    },
    #[error("stale plan epoch for device {device_id}: received {received}, current {current}")]
    StaleEpoch {
        device_id: String,
        received: u64,
        current: u64,
    },
    #[error("delivery queue is full (limit {limit})")]
    QueueFull { limit: usize },
    #[error("database must be a local SQLite file, not {0}")]
    InvalidDatabasePath(String),
    #[error("SQLite WAL mode was not enabled (reported {0})")]
    WalUnavailable(String),
    #[error(
        "database schema version {found} is newer than this controller supports ({supported})"
    )]
    FutureSchema { found: u32, supported: u32 },
    #[error("revision source digest does not match exact source bytes")]
    RevisionDigestMismatch,
    #[error("backup destination must differ from the source database")]
    BackupSamePath,
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    Enqueued,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    Observation,
    Audit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineReferenceKind {
    Deployment,
    Audit,
}

impl BaselineReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Deployment => "deployment",
            Self::Audit => "audit",
        }
    }
}

impl RetentionClass {
    fn retention_ms(self) -> u64 {
        match self {
            Self::Observation => OBSERVATION_RETENTION_MS,
            Self::Audit => AUDIT_RETENTION_MS,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Audit => "audit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxMessage {
    pub envelope: MessageEnvelope,
    pub attempts: u32,
    pub not_before_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxMessage {
    pub envelope: MessageEnvelope,
    pub received_at_ms: u64,
}

/// SQLite is local state, opened in WAL mode.  The mutex is intentional: it
/// makes every mutation a serialized, durable transaction.
pub struct Store {
    connection: Mutex<Connection>,
    max_queue: usize,
    database_path: PathBuf,
}

impl Store {
    pub fn open(path: impl AsRef<Path>, max_queue: usize) -> Result<Self> {
        let path = path.as_ref();
        validate_database_path(path)?;
        let connection = Connection::open(path)?;
        let metadata = fs::metadata(path)
            .map_err(|_| StorageError::InvalidDatabasePath(path.display().to_string()))?;
        if !metadata.is_file() {
            return Err(StorageError::InvalidDatabasePath(
                path.display().to_string(),
            ));
        }
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let journal_mode: String =
            connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(StorageError::WalUnavailable(journal_mode));
        }
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        migrate(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            max_queue,
            database_path: fs::canonicalize(path)
                .map_err(|_| StorageError::InvalidDatabasePath(path.display().to_string()))?,
        })
    }

    /// Durably records an inbound envelope before its transport acknowledgement.
    /// It verifies the advertised digest against the original payload bytes only.
    pub fn receive(&self, envelope: &MessageEnvelope, now_ms: u64) -> Result<ReceiveOutcome> {
        let mut connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<StoredEnvelope> = tx
            .query_row(
                "SELECT envelope FROM inbox WHERE message_id = ?1",
                [envelope.message_id.as_str()],
                |row| {
                    serde_json::from_str::<StoredEnvelope>(&row.get::<_, String>(0)?)
                        .map_err(json_sql_error)
                },
            )
            .optional()?;
        if let Some(existing) = existing {
            return if existing.0 == *envelope {
                Ok(ReceiveOutcome::Duplicate)
            } else {
                Err(StorageError::ConflictingMessageId(
                    envelope.message_id.clone(),
                ))
            };
        }
        validate(envelope, now_ms)?;
        advance_fence(&tx, envelope)?;
        retire_undeliverable(&tx, now_ms)?;
        let count: usize = tx.query_row(
            "SELECT count(*) FROM inbox WHERE processed_at_ms IS NULL AND discarded_at_ms IS NULL",
            [],
            |r| r.get(0),
        )?;
        if count >= self.max_queue {
            return Err(StorageError::QueueFull {
                limit: self.max_queue,
            });
        }
        tx.execute("INSERT INTO inbox(message_id, envelope, payload, received_at_ms, processed_at_ms) VALUES (?1, ?2, ?3, ?4, NULL)",
            params![envelope.message_id, serde_json::to_string(&StoredEnvelope(envelope.clone()))?, envelope.payload, now_ms])?;
        tx.commit()?;
        Ok(ReceiveOutcome::Inserted)
    }

    /// Adds a sender-side record.  It does not claim delivery or physical execution.
    pub fn enqueue_outbox(
        &self,
        envelope: &MessageEnvelope,
        now_ms: u64,
    ) -> Result<EnqueueOutcome> {
        validate(envelope, now_ms)?;
        let mut connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<StoredEnvelope> = tx
            .query_row(
                "SELECT envelope FROM outbox WHERE message_id = ?1",
                [envelope.message_id.as_str()],
                |row| {
                    serde_json::from_str::<StoredEnvelope>(&row.get::<_, String>(0)?)
                        .map_err(json_sql_error)
                },
            )
            .optional()?;
        if let Some(existing) = existing {
            return if existing.0 == *envelope {
                Ok(EnqueueOutcome::Duplicate)
            } else {
                Err(StorageError::ConflictingMessageId(
                    envelope.message_id.clone(),
                ))
            };
        }
        advance_fence(&tx, envelope)?;
        retire_undeliverable(&tx, now_ms)?;
        let count: usize = tx.query_row(
            "SELECT count(*) FROM outbox WHERE acknowledged_at_ms IS NULL AND discarded_at_ms IS NULL",
            [],
            |r| r.get(0),
        )?;
        if count >= self.max_queue {
            return Err(StorageError::QueueFull {
                limit: self.max_queue,
            });
        }
        tx.execute("INSERT INTO outbox(message_id, envelope, attempts, not_before_ms, acknowledged_at_ms) VALUES (?1, ?2, 0, ?3, NULL)",
            params![envelope.message_id, serde_json::to_string(&StoredEnvelope(envelope.clone()))?, now_ms])?;
        tx.commit()?;
        Ok(EnqueueOutcome::Enqueued)
    }

    pub fn acknowledge_outbox(&self, message_id: &str, now_ms: u64) -> Result<bool> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        Ok(connection.execute("UPDATE outbox SET acknowledged_at_ms = ?2 WHERE message_id = ?1 AND acknowledged_at_ms IS NULL", params![message_id, now_ms])? != 0)
    }

    /// Retry scheduling is monotonic: attempts and its next eligible time never move backwards.
    pub fn retry_outbox(&self, message_id: &str, not_before_ms: u64) -> Result<bool> {
        let mut connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let envelope: Option<StoredEnvelope> = tx
            .query_row(
                "SELECT envelope FROM outbox WHERE message_id = ?1 AND acknowledged_at_ms IS NULL AND discarded_at_ms IS NULL",
                [message_id],
                |row| serde_json::from_str(&row.get::<_, String>(0)?).map_err(json_sql_error),
            )
            .optional()?;
        let Some(envelope) = envelope else {
            return Ok(false);
        };
        if not_before_ms >= envelope.0.expires_at_ms {
            return Err(StorageError::InvalidMessage(
                "retry would occur after envelope expiry",
            ));
        }
        assert_current_fence(&tx, &envelope.0)?;
        let changed = tx.execute("UPDATE outbox SET attempts = attempts + 1, not_before_ms = MAX(not_before_ms, ?2) WHERE message_id = ?1 AND acknowledged_at_ms IS NULL", params![message_id, not_before_ms])? != 0;
        tx.commit()?;
        Ok(changed)
    }

    pub fn ready_outbox(&self, now_ms: u64, limit: usize) -> Result<Vec<OutboxMessage>> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let mut statement = connection.prepare("SELECT envelope, attempts, not_before_ms FROM outbox WHERE acknowledged_at_ms IS NULL AND discarded_at_ms IS NULL AND not_before_ms <= ?1 ORDER BY not_before_ms, message_id")?;
        let messages = statement
            .query_map([now_ms], |r| {
                Ok(OutboxMessage {
                    envelope: serde_json::from_str::<StoredEnvelope>(&r.get::<_, String>(0)?)
                        .map_err(json_sql_error)?
                        .0,
                    attempts: r.get(1)?,
                    not_before_ms: r.get(2)?,
                })
            })?
            .filter_map(|item| match item {
                Ok(message) => match is_deliverable(&connection, &message.envelope, now_ms) {
                    Ok(true) => Some(Ok(message)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
                Err(error) => Some(Err(StorageError::from(error))),
            })
            .take(limit)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    /// Lists durable, unprocessed receipts that are still current for their device.
    /// Older fencing generations and epochs are retained as deduplication tombstones.
    pub fn pending_inbox(&self, now_ms: u64, limit: usize) -> Result<Vec<InboxMessage>> {
        self.pending_inbox_matching(now_ms, limit, |_| true)
    }

    /// Filter routing before the batch limit so one device's queued messages
    /// cannot starve a different controller-owned assignment route.
    pub fn pending_inbox_route(&self, now_ms: u64, limit: usize, sender: &str, recipient: &str, device: &intent_protocol::DeviceBinding) -> Result<Vec<InboxMessage>> {
        self.pending_inbox_matching(now_ms, limit, |e| e.sender == sender && e.recipient == recipient && &e.device == device)
    }

    fn pending_inbox_matching(&self, now_ms: u64, limit: usize, matches: impl Fn(&MessageEnvelope) -> bool) -> Result<Vec<InboxMessage>> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let mut statement = connection.prepare("SELECT envelope, received_at_ms FROM inbox WHERE processed_at_ms IS NULL AND discarded_at_ms IS NULL ORDER BY received_at_ms, message_id")?;
        let messages = statement
            .query_map([], |r| {
                Ok(InboxMessage {
                    envelope: serde_json::from_str::<StoredEnvelope>(&r.get::<_, String>(0)?)
                        .map_err(json_sql_error)?
                        .0,
                    received_at_ms: r.get(1)?,
                })
            })?
            .filter_map(|item| match item {
                Ok(message) if !matches(&message.envelope) => None,
                Ok(message) => match is_deliverable(&connection, &message.envelope, now_ms) {
                    Ok(true) => Some(Ok(message)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
                Err(error) => Some(Err(StorageError::from(error))),
            })
            .take(limit)
            .collect::<std::result::Result<Vec<_>, _>>();
        messages
    }

    /// Processing completion is durable and idempotent; it does not erase the receipt tombstone.
    pub fn mark_inbox_processed(&self, message_id: &str, processed_at_ms: u64) -> Result<bool> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        Ok(connection.execute("UPDATE inbox SET processed_at_ms = ?2 WHERE message_id = ?1 AND processed_at_ms IS NULL AND discarded_at_ms IS NULL", params![message_id, processed_at_ms])? != 0)
    }

    pub fn accept_revision(
        &self,
        id: &str,
        source_digest: &str,
        source: &[u8],
        accepted_at_ms: u64,
    ) -> Result<()> {
        validate_identifier(id, "revision ID")?;
        if payload_digest(source) != source_digest {
            return Err(StorageError::RevisionDigestMismatch);
        }
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        connection.execute("INSERT INTO revisions(id, source_digest, source, accepted_at_ms) VALUES (?1, ?2, ?3, ?4)", params![id, source_digest, source, accepted_at_ms])?;
        Ok(())
    }

    pub fn put_snapshot(
        &self,
        id: &str,
        retention: RetentionClass,
        body: &[u8],
        recorded_at_ms: u64,
        baseline_id: Option<&str>,
    ) -> Result<()> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let tx = connection.unchecked_transaction()?;
        tx.execute("INSERT INTO snapshots(id, retention_class, body, recorded_at_ms) VALUES (?1, ?2, ?3, ?4)", params![id, retention.as_str(), body, recorded_at_ms])?;
        if let Some(baseline_id) = baseline_id {
            tx.execute(
                "INSERT INTO snapshot_baselines(snapshot_id, baseline_id) VALUES (?1, ?2)",
                params![id, baseline_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Pins a baseline while a deployment record or audit record depends on it.
    pub fn reference_baseline(
        &self,
        reference_id: &str,
        kind: BaselineReferenceKind,
        baseline_id: &str,
    ) -> Result<()> {
        validate_identifier(reference_id, "baseline reference ID")?;
        validate_identifier(baseline_id, "baseline ID")?;
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        connection.execute(
            "INSERT INTO baseline_references(reference_id, kind, baseline_id) VALUES (?1, ?2, ?3)",
            params![reference_id, kind.as_str(), baseline_id],
        )?;
        Ok(())
    }

    /// Deletes only expired records with no baseline reference.  Audit evidence
    /// has the longer retention window; referenced baselines remain intact.
    pub fn prune(&self, now_ms: u64) -> Result<usize> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let observation_cutoff = now_ms.saturating_sub(RetentionClass::Observation.retention_ms());
        let audit_cutoff = now_ms.saturating_sub(RetentionClass::Audit.retention_ms());
        Ok(connection.execute("DELETE FROM snapshots WHERE id NOT IN (SELECT baseline_id FROM snapshot_baselines) AND id NOT IN (SELECT baseline_id FROM baseline_references) AND ((retention_class = 'observation' AND recorded_at_ms < ?1) OR (retention_class = 'audit' AND recorded_at_ms < ?2))", params![observation_cutoff, audit_cutoff])?)
    }

    pub fn snapshot_exists(&self, id: &str) -> Result<bool> {
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        Ok(connection
            .query_row("SELECT 1 FROM snapshots WHERE id = ?1", [id], |_| Ok(()))
            .optional()?
            .is_some())
    }

    /// Uses SQLite's online backup API while the source remains usable.
    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<()> {
        let destination_path = destination.as_ref();
        validate_database_path(destination_path)?;
        if paths_refer_to_same_file(&self.database_path, destination_path) {
            return Err(StorageError::BackupSamePath);
        }
        let connection = self.connection.lock().map_err(|_| StorageError::Poisoned)?;
        let mut destination = Connection::open(destination_path)?;
        let backup = Backup::new(&connection, &mut destination)?;
        backup.run_to_completion(64, std::time::Duration::from_millis(1), None)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredEnvelope(MessageEnvelope);

fn json_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

pub(crate) fn validate(envelope: &MessageEnvelope, now_ms: u64) -> Result<()> {
    if envelope.version != PROTOCOL_VERSION {
        return Err(StorageError::InvalidMessage("unsupported protocol version"));
    }
    validate_identifier(&envelope.message_id, "message ID")?;
    validate_identifier(&envelope.sender, "sender")?;
    validate_identifier(&envelope.recipient, "recipient")?;
    validate_identifier(&envelope.device.device_id, "device ID")?;
    validate_identifier(&envelope.device.profile_digest, "profile digest")?;
    if envelope.payload.len() > MAX_PAYLOAD_BYTES {
        return Err(StorageError::InvalidMessage(
            "payload exceeds storage limit",
        ));
    }
    if envelope.created_at_ms > now_ms {
        return Err(StorageError::InvalidMessage(
            "envelope creation time is in the future",
        ));
    }
    if now_ms >= envelope.expires_at_ms {
        return Err(StorageError::InvalidMessage("envelope has expired"));
    }
    if payload_digest(&envelope.payload) != envelope.payload_digest {
        return Err(StorageError::InvalidMessage(
            "payload digest does not match exact payload bytes",
        ));
    }
    Ok(())
}

fn validate_identifier(value: &str, name: &'static str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES || value.bytes().any(|byte| byte == 0)
    {
        return Err(StorageError::InvalidMessage(name));
    }
    Ok(())
}

fn validate_database_path(path: &Path) -> Result<()> {
    let rendered = path.to_string_lossy();
    if rendered == ":memory:" || rendered.starts_with("file:") || rendered.contains("?mode=memory")
    {
        return Err(StorageError::InvalidDatabasePath(rendered.into_owned()));
    }
    Ok(())
}

fn paths_refer_to_same_file(source: &Path, destination: &Path) -> bool {
    match fs::canonicalize(destination) {
        Ok(destination) => destination == source,
        Err(_) => destination == source,
    }
}

fn is_deliverable(
    connection: &Connection,
    envelope: &MessageEnvelope,
    now_ms: u64,
) -> Result<bool> {
    if now_ms >= envelope.expires_at_ms {
        return Ok(false);
    }
    let fence: Option<(u64, u64)> = connection
        .query_row(
            "SELECT fencing_generation, plan_epoch FROM device_fences WHERE device_id = ?1",
            [envelope.device.device_id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(matches!(fence, Some((generation, epoch))
        if generation == envelope.device.fencing_generation && epoch == envelope.plan_epoch))
}

// Retire delivery work without erasing immutable receipt/outbox tombstones.
// Apply operation journals are separate: this is not permission to retry or
// discard an uncertain physical operation.
fn retire_undeliverable(tx: &rusqlite::Transaction<'_>, now_ms: u64) -> Result<()> {
    for (table, pending) in [
        ("inbox", "processed_at_ms"),
        ("outbox", "acknowledged_at_ms"),
    ] {
        let mut statement = tx.prepare(&format!("SELECT message_id, envelope FROM {table} WHERE {pending} IS NULL AND discarded_at_ms IS NULL"))?;
        let rows = statement
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (id, bytes) in rows {
            let envelope: StoredEnvelope = serde_json::from_str(&bytes)?;
            if !is_deliverable(tx, &envelope.0, now_ms)? {
                let reason = if now_ms >= envelope.0.expires_at_ms {
                    "expired"
                } else {
                    "superseded"
                };
                tx.execute(&format!("UPDATE {table} SET discarded_at_ms = ?2, discard_reason = ?3 WHERE message_id = ?1"), params![id, now_ms, reason])?;
            }
        }
    }
    Ok(())
}

fn assert_current_fence(tx: &rusqlite::Transaction<'_>, envelope: &MessageEnvelope) -> Result<()> {
    let fence: Option<(u64, u64)> = tx
        .query_row(
            "SELECT fencing_generation, plan_epoch FROM device_fences WHERE device_id = ?1",
            [envelope.device.device_id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match fence {
        Some((generation, _)) if generation > envelope.device.fencing_generation => {
            Err(StorageError::StaleFence {
                device_id: envelope.device.device_id.clone(),
                received: envelope.device.fencing_generation,
                current: generation,
            })
        }
        Some((generation, epoch))
            if generation == envelope.device.fencing_generation && epoch > envelope.plan_epoch =>
        {
            Err(StorageError::StaleEpoch {
                device_id: envelope.device.device_id.clone(),
                received: envelope.plan_epoch,
                current: epoch,
            })
        }
        _ => Ok(()),
    }
}

fn advance_fence(tx: &rusqlite::Transaction<'_>, envelope: &MessageEnvelope) -> Result<()> {
    let existing: Option<(u64, u64)> = tx
        .query_row(
            "SELECT fencing_generation, plan_epoch FROM device_fences WHERE device_id = ?1",
            [envelope.device.device_id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((fence, epoch)) = existing {
        if envelope.device.fencing_generation < fence {
            return Err(StorageError::StaleFence {
                device_id: envelope.device.device_id.clone(),
                received: envelope.device.fencing_generation,
                current: fence,
            });
        }
        if envelope.device.fencing_generation == fence && envelope.plan_epoch < epoch {
            return Err(StorageError::StaleEpoch {
                device_id: envelope.device.device_id.clone(),
                received: envelope.plan_epoch,
                current: epoch,
            });
        }
    }
    // A plan epoch is monotonic within one fencing generation.  Acquiring a
    // newer generation establishes a new epoch namespace, so its epoch may
    // restart; messages from the prior generation can never become ready again.
    tx.execute("INSERT INTO device_fences(device_id, fencing_generation, plan_epoch) VALUES (?1, ?2, ?3) ON CONFLICT(device_id) DO UPDATE SET fencing_generation = excluded.fencing_generation, plan_epoch = excluded.plan_epoch", params![envelope.device.device_id, envelope.device.fencing_generation, envelope.plan_epoch])?;
    Ok(())
}

fn migrate(connection: &Connection) -> Result<()> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(StorageError::FutureSchema {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    let tx = connection.unchecked_transaction()?;
    let mut version = version;
    while version < SCHEMA_VERSION {
        match version {
            0 => tx.execute_batch("CREATE TABLE inbox (message_id TEXT PRIMARY KEY, envelope TEXT NOT NULL, payload BLOB NOT NULL, received_at_ms INTEGER NOT NULL);
                CREATE TABLE outbox (message_id TEXT PRIMARY KEY, envelope TEXT NOT NULL, attempts INTEGER NOT NULL, not_before_ms INTEGER NOT NULL, acknowledged_at_ms INTEGER);
                CREATE TABLE device_fences (device_id TEXT PRIMARY KEY, fencing_generation INTEGER NOT NULL, plan_epoch INTEGER NOT NULL);
                CREATE TABLE revisions (id TEXT PRIMARY KEY, source_digest TEXT NOT NULL, source BLOB NOT NULL, accepted_at_ms INTEGER NOT NULL);
                CREATE TABLE snapshots (id TEXT PRIMARY KEY, retention_class TEXT NOT NULL CHECK(retention_class IN ('observation', 'audit')), body BLOB NOT NULL, recorded_at_ms INTEGER NOT NULL);
                CREATE TABLE snapshot_baselines (snapshot_id TEXT PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE, baseline_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE RESTRICT);
                PRAGMA user_version = 1;")?,
            1 => tx.execute_batch("ALTER TABLE inbox ADD COLUMN processed_at_ms INTEGER; PRAGMA user_version = 2;")?,
            2 => tx.execute_batch("CREATE TABLE baseline_references (reference_id TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('deployment', 'audit')), baseline_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE RESTRICT); PRAGMA user_version = 3;")?,
            3 => tx.execute_batch("ALTER TABLE inbox ADD COLUMN discarded_at_ms INTEGER;
                ALTER TABLE inbox ADD COLUMN discard_reason TEXT;
                ALTER TABLE outbox ADD COLUMN discarded_at_ms INTEGER;
                ALTER TABLE outbox ADD COLUMN discard_reason TEXT;
                CREATE INDEX inbox_pending ON inbox(processed_at_ms, discarded_at_ms);
                CREATE INDEX outbox_pending ON outbox(acknowledged_at_ms, discarded_at_ms, not_before_ms);
                PRAGMA user_version = 4;")?,
            _ => unreachable!("future versions were rejected above"),
        }
        version += 1;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use intent_protocol::{DeviceBinding, MessageEnvelope};
    use tempfile::tempdir;

    fn envelope(id: &str, fence: u64, epoch: u64, payload: &[u8]) -> MessageEnvelope {
        MessageEnvelope {
            version: PROTOCOL_VERSION,
            message_id: id.into(),
            sender: "witness-a".into(),
            recipient: "controller".into(),
            device: DeviceBinding {
                device_id: "router-a".into(),
                profile_digest: "profile".into(),
                fencing_generation: fence,
            },
            plan_epoch: epoch,
            created_at_ms: 10,
            expires_at_ms: 10_000,
            payload_digest: payload_digest(payload),
            payload: payload.into(),
        }
    }
    fn store() -> (tempfile::TempDir, Store) {
        let d = tempdir().unwrap();
        let s = Store::open(d.path().join("state.sqlite"), 2).unwrap();
        (d, s)
    }

    #[test]
    fn restart_deduplicates_exact_message() {
        let (d, s) = store();
        let e = envelope("m1", 1, 1, b"raw bytes");
        assert_eq!(s.receive(&e, 20).unwrap(), ReceiveOutcome::Inserted);
        drop(s);
        let s = Store::open(d.path().join("state.sqlite"), 2).unwrap();
        assert_eq!(s.receive(&e, 20).unwrap(), ReceiveOutcome::Duplicate);
    }
    #[test]
    fn expired_or_superseded_messages_release_capacity_without_erasing_tombstones() {
        let (_d, s) = store();
        for id in ["a", "b"] {
            s.receive(&envelope(id, 1, 1, id.as_bytes()), 20).unwrap();
            s.enqueue_outbox(&envelope(id, 1, 1, id.as_bytes()), 20)
                .unwrap();
        }
        let mut fresh = envelope("c", 2, 1, b"c");
        fresh.created_at_ms = 20_000;
        fresh.expires_at_ms = 30_000;
        s.receive(&fresh, 20_001).unwrap();
        s.enqueue_outbox(&fresh, 20_001).unwrap();
        assert_eq!(s.pending_inbox(20_001, 10).unwrap().len(), 1);
        assert_eq!(s.ready_outbox(20_001, 10).unwrap().len(), 1);
        assert_eq!(
            s.receive(&envelope("a", 1, 1, b"a"), 20_001).unwrap(),
            ReceiveOutcome::Duplicate
        );
        assert!(!s.mark_inbox_processed("a", 20_001).unwrap());
    }

    #[test]
    fn conflicting_id_is_rejected() {
        let (_d, s) = store();
        assert!(matches!(
            s.receive(&envelope("m1", 1, 1, b"a"), 20),
            Ok(ReceiveOutcome::Inserted)
        ));
        assert!(matches!(
            s.receive(&envelope("m1", 1, 1, b"b"), 20),
            Err(StorageError::ConflictingMessageId(_))
        ));
    }
    #[test]
    fn fence_and_epoch_cannot_roll_back() {
        let (_d, s) = store();
        s.receive(&envelope("new", 2, 4, b"a"), 20).unwrap();
        assert!(matches!(
            s.receive(&envelope("old-fence", 1, 99, b"b"), 20),
            Err(StorageError::StaleFence { .. })
        ));
        assert!(matches!(
            s.receive(&envelope("old-epoch", 2, 3, b"c"), 20),
            Err(StorageError::StaleEpoch { .. })
        ));
    }
    #[test]
    fn outbox_is_bounded_and_progress_is_monotonic() {
        let (_d, s) = store();
        s.enqueue_outbox(&envelope("a", 1, 1, b"a"), 20).unwrap();
        s.enqueue_outbox(&envelope("b", 1, 1, b"b"), 20).unwrap();
        assert!(matches!(
            s.enqueue_outbox(&envelope("c", 1, 1, b"c"), 20),
            Err(StorageError::QueueFull { .. })
        ));
        s.retry_outbox("a", 100).unwrap();
        s.retry_outbox("a", 50).unwrap();
        let one = s
            .ready_outbox(100, 5)
            .unwrap()
            .into_iter()
            .find(|x| x.envelope.message_id == "a")
            .unwrap();
        assert_eq!((one.attempts, one.not_before_ms), (2, 100));
    }
    #[test]
    fn expired_and_bad_digest_are_rejected() {
        let (_d, s) = store();
        assert!(matches!(
            s.receive(&envelope("late", 1, 1, b"x"), 10_001),
            Err(StorageError::InvalidMessage(_))
        ));
        let mut e = envelope("digest", 1, 1, b"x");
        e.payload_digest = "bad".into();
        assert!(matches!(
            s.receive(&e, 20),
            Err(StorageError::InvalidMessage(_))
        ));
    }
    #[test]
    fn validates_required_fields_time_window_and_bounds() {
        let (_d, s) = store();
        let mut future = envelope("future", 1, 1, b"x");
        future.created_at_ms = 21;
        assert!(matches!(
            s.receive(&future, 20),
            Err(StorageError::InvalidMessage(_))
        ));
        let mut missing_sender = envelope("sender", 1, 1, b"x");
        missing_sender.sender.clear();
        assert!(matches!(
            s.receive(&missing_sender, 20),
            Err(StorageError::InvalidMessage(_))
        ));
        let mut missing_profile = envelope("profile", 1, 1, b"x");
        missing_profile.device.profile_digest.clear();
        assert!(matches!(
            s.receive(&missing_profile, 20),
            Err(StorageError::InvalidMessage(_))
        ));
        let payload = vec![0; MAX_PAYLOAD_BYTES + 1];
        assert!(matches!(
            s.receive(&envelope("large", 1, 1, &payload), 20),
            Err(StorageError::InvalidMessage(_))
        ));
    }
    #[test]
    fn inbox_is_bounded_and_processed_state_survives_restart() {
        let (d, s) = store();
        s.receive(&envelope("a", 1, 1, b"a"), 20).unwrap();
        s.receive(&envelope("b", 1, 1, b"b"), 20).unwrap();
        assert!(matches!(
            s.receive(&envelope("c", 1, 1, b"c"), 20),
            Err(StorageError::QueueFull { .. })
        ));
        assert_eq!(s.pending_inbox(20, 10).unwrap().len(), 2);
        assert!(s.mark_inbox_processed("a", 21).unwrap());
        assert!(!s.mark_inbox_processed("a", 22).unwrap());
        drop(s);
        let s = Store::open(d.path().join("state.sqlite"), 2).unwrap();
        let pending = s.pending_inbox(30, 10).unwrap();
        assert_eq!(
            pending
                .iter()
                .map(|m| m.envelope.message_id.as_str())
                .collect::<Vec<_>>(),
            ["b"]
        );
        s.receive(&envelope("c", 1, 1, b"c"), 30).unwrap();
    }
    #[test]
    fn duplicate_receipt_is_acknowledged_after_expiry_but_never_processed() {
        let (_d, s) = store();
        let mut e = envelope("once", 1, 1, b"x");
        e.expires_at_ms = 30;
        assert_eq!(s.receive(&e, 20).unwrap(), ReceiveOutcome::Inserted);
        assert_eq!(s.receive(&e, 30).unwrap(), ReceiveOutcome::Duplicate);
        assert!(s.pending_inbox(30, 1).unwrap().is_empty());
    }
    #[test]
    fn ready_messages_never_cross_expiry_or_a_new_fence() {
        let d = tempdir().unwrap();
        let s = Store::open(d.path().join("state.sqlite"), 3).unwrap();
        let mut expired = envelope("expired", 1, 1, b"x");
        expired.expires_at_ms = 30;
        s.enqueue_outbox(&expired, 20).unwrap();
        s.enqueue_outbox(&envelope("old", 1, 1, b"a"), 20).unwrap();
        s.enqueue_outbox(&envelope("current", 2, 0, b"b"), 20)
            .unwrap();
        let ready = s.ready_outbox(30, 10).unwrap();
        assert_eq!(
            ready
                .iter()
                .map(|m| m.envelope.message_id.as_str())
                .collect::<Vec<_>>(),
            ["current"]
        );
        // Superseded messages have already been retired to immutable tombstones.
        assert!(!s.retry_outbox("old", 31).unwrap());
        assert!(!s.retry_outbox("expired", 30).unwrap());
    }
    #[test]
    fn revision_digest_must_match_source_bytes() {
        let (_d, s) = store();
        assert!(matches!(
            s.accept_revision("r1", "wrong", b"source", 1),
            Err(StorageError::RevisionDigestMismatch)
        ));
        s.accept_revision("r1", &payload_digest(b"source"), b"source", 1)
            .unwrap();
    }
    #[test]
    fn rejects_memory_and_uri_databases() {
        assert!(matches!(
            Store::open(":memory:", 1),
            Err(StorageError::InvalidDatabasePath(_))
        ));
        assert!(matches!(
            Store::open("file:state.sqlite?mode=memory", 1),
            Err(StorageError::InvalidDatabasePath(_))
        ));
    }
    #[test]
    fn future_migration_is_rejected_without_changing_version() {
        let d = tempdir().unwrap();
        let path = d.path().join("future.sqlite");
        let c = Connection::open(&path).unwrap();
        c.pragma_update(None, "user_version", 99_u32).unwrap();
        drop(c);
        assert!(matches!(
            Store::open(&path, 1),
            Err(StorageError::FutureSchema { found: 99, .. })
        ));
        let c = Connection::open(&path).unwrap();
        let version: u32 = c
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 99);
    }
    #[test]
    fn versioned_migration_upgrades_v1_without_resetting_it() {
        let d = tempdir().unwrap();
        let path = d.path().join("v1.sqlite");
        let c = Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE inbox (message_id TEXT PRIMARY KEY, envelope TEXT NOT NULL, payload BLOB NOT NULL, received_at_ms INTEGER NOT NULL);
            CREATE TABLE outbox (message_id TEXT PRIMARY KEY, envelope TEXT NOT NULL, attempts INTEGER NOT NULL, not_before_ms INTEGER NOT NULL, acknowledged_at_ms INTEGER);
            CREATE TABLE device_fences (device_id TEXT PRIMARY KEY, fencing_generation INTEGER NOT NULL, plan_epoch INTEGER NOT NULL);
            CREATE TABLE revisions (id TEXT PRIMARY KEY, source_digest TEXT NOT NULL, source BLOB NOT NULL, accepted_at_ms INTEGER NOT NULL);
            CREATE TABLE snapshots (id TEXT PRIMARY KEY, retention_class TEXT NOT NULL CHECK(retention_class IN ('observation', 'audit')), body BLOB NOT NULL, recorded_at_ms INTEGER NOT NULL);
            CREATE TABLE snapshot_baselines (snapshot_id TEXT PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE, baseline_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE RESTRICT);
            PRAGMA user_version = 1;").unwrap();
        drop(c);
        let _store = Store::open(&path, 1).unwrap();
        let c = Connection::open(&path).unwrap();
        let version: u32 = c
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let columns: Vec<String> = c
            .prepare("PRAGMA table_info(inbox)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert!(columns.contains(&"processed_at_ms".to_owned()));
    }
    #[test]
    fn transaction_rolls_back_baseline_link_failure() {
        let (_d, s) = store();
        assert!(s
            .put_snapshot(
                "orphan",
                RetentionClass::Observation,
                b"x",
                1,
                Some("missing")
            )
            .is_err());
        assert!(!s.snapshot_exists("orphan").unwrap());
    }
    #[test]
    fn retention_preserves_baselines_and_backup_restores() {
        let (d, s) = store();
        let now = OBSERVATION_RETENTION_MS + 10;
        s.put_snapshot("base", RetentionClass::Observation, b"b", 0, None)
            .unwrap();
        s.put_snapshot("audit", RetentionClass::Audit, b"a", now, Some("base"))
            .unwrap();
        s.put_snapshot(
            "deployment-base",
            RetentionClass::Observation,
            b"d",
            0,
            None,
        )
        .unwrap();
        s.reference_baseline(
            "deployment-1",
            BaselineReferenceKind::Deployment,
            "deployment-base",
        )
        .unwrap();
        s.put_snapshot("old", RetentionClass::Observation, b"o", 0, None)
            .unwrap();
        assert_eq!(s.prune(now).unwrap(), 1);
        assert!(s.snapshot_exists("base").unwrap());
        assert!(s.snapshot_exists("deployment-base").unwrap());
        assert!(s.snapshot_exists("audit").unwrap());
        let backup = d.path().join("backup.sqlite");
        s.backup(&backup).unwrap();
        drop(s);
        let restored = Store::open(backup, 2).unwrap();
        assert!(restored.snapshot_exists("base").unwrap());
        assert!(restored.snapshot_exists("audit").unwrap());
    }
    #[test]
    fn backup_refuses_source_path() {
        let (d, s) = store();
        assert!(matches!(
            s.backup(d.path().join("state.sqlite")),
            Err(StorageError::BackupSamePath)
        ));
    }
}
