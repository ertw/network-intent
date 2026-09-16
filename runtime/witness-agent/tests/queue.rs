use intent_identity::{
    AgentIdentity, DsseEnvelope, SigningKey, TrustedKey,
    evidence::{AssignmentScope, verify_assignment},
};
use intent_protocol::{
    DeviceBinding, MessageEnvelope, PROTOCOL_VERSION, RevisionRef,
    assurance::{
        Endpoint, Expectation, IpFamily, Primitive, ProbeSource, ProbeSpec, ResourceLimits,
    },
    evidence::{
        STATEMENT_TYPE, Statement, WITNESS_ASSIGNMENT_TYPE, WitnessAssignment, revision_subject,
    },
    payload_digest,
};
use intent_witness_agent::{
    execution::WitnessExecutor,
    local::LocalObservationContext,
    queue::{QueueError, Receive, WitnessQueue},
};
use std::net::IpAddr;

struct F {
    c: AgentIdentity,
    w: AgentIdentity,
    ck: SigningKey,
    wpk: Vec<u8>,
    keys: Vec<TrustedKey>,
    d: DeviceBinding,
    r: RevisionRef,
    p: ProbeSpec,
    ips: Vec<IpAddr>,
    a: WitnessAssignment,
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn trust(i: &AgentIdentity, k: &SigningKey) -> TrustedKey {
    TrustedKey {
        identity: i.clone(),
        public_key: k.public_key(),
        not_before_ms: 0,
        not_after_ms: u64::MAX,
        revoked: false,
    }
}
impl F {
    fn new() -> Self {
        let c = AgentIdentity::parse("spiffe://lab.local/agent/controller/router-a/c").unwrap();
        let w = AgentIdentity::parse("spiffe://lab.local/agent/witness/router-a/w").unwrap();
        let (ck, _) = SigningKey::generate().unwrap();
        let (_wk, wpk) = SigningKey::generate().unwrap();
        let d = DeviceBinding {
            device_id: "router-a".into(),
            profile_digest: "p".into(),
            fencing_generation: 4,
        };
        let r = RevisionRef {
            id: "r".into(),
            source_digest: "s".into(),
        };
        let ips = vec!["192.0.2.2".parse().unwrap()];
        let p = ProbeSpec {
            id: "p".into(),
            primitive: Primitive::TcpConnect,
            device: d.clone(),
            source: ProbeSource {
                witness_id: w.to_string(),
                location: "lab".into(),
                bind_address: ips[0],
                interface: Some("wan0".into()),
            },
            endpoint: Some(Endpoint {
                address: "198.51.100.1".parse().unwrap(),
                port: Some(443),
                family: IpFamily::V4,
            }),
            expectation: Expectation::TcpConnected,
            limits: ResourceLimits {
                timeout_ms: 1000,
                max_response_bytes: 1024,
                max_attempts: 1,
            },
            interval_ms: 10000,
            depends_on: vec![],
            claims: vec![],
        };
        let n = now();
        let a = WitnessAssignment {
            deployment: None,
            version: 1,
            assignment_id: "a".into(),
            issuer: c.to_string(),
            recipient: w.to_string(),
            issued_at_ms: n - 10,
            expires_at_ms: n + 60_000,
            plan_id: "plan".into(),
            plan_epoch: 1,
            graph_version: 1,
            revision: r.clone(),
            device: d.clone(),
            probes: vec![p.clone()],
        };
        let keys = vec![trust(&c, &ck)];
        Self {
            c: c.clone(),
            w: w.clone(),
            ck,
            wpk,
            keys,
            d,
            r,
            p,
            ips,
            a,
        }
    }
    fn scope(&self) -> AssignmentScope<'_> {
        AssignmentScope {
            controller: &self.c,
            witness: &self.w,
            device: &self.d,
            revision: &self.r,
            plan_id: "plan",
            plan_epoch: self.a.plan_epoch,
            graph_version: self.a.graph_version,
            location: "lab",
            bind_addresses: &self.ips,
        }
    }
    fn dsse(&self) -> DsseEnvelope {
        self.ck
            .sign(
                &serde_json::to_vec(&Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&self.r),
                    predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                    predicate: self.a.clone(),
                })
                .unwrap(),
            )
            .unwrap()
    }
    fn verified(&self) -> intent_identity::evidence::VerifiedAssignment {
        verify_assignment(&self.dsse(), &self.keys, &self.scope(), now()).unwrap()
    }
    fn msg(&self, id: &str) -> MessageEnvelope {
        let payload = serde_json::to_vec(&self.dsse()).unwrap();
        MessageEnvelope {
            version: PROTOCOL_VERSION,
            message_id: id.into(),
            sender: self.c.to_string(),
            recipient: self.w.to_string(),
            device: self.d.clone(),
            plan_epoch: self.a.plan_epoch,
            created_at_ms: now() - 1,
            expires_at_ms: now() + 50_000,
            payload_digest: payload_digest(&payload),
            payload,
        }
    }
    fn executor(&self) -> WitnessExecutor {
        WitnessExecutor::new(
            self.w.clone(),
            SigningKey::from_pkcs8(&self.wpk).unwrap(),
            LocalObservationContext {
                device: self.d.clone(),
                source: self.p.source.clone(),
                rpcd_session: "s".into(),
            },
        )
        .unwrap()
    }
}
#[tokio::test]
async fn durable_assignment_executes_once_and_retries_saved_evidence_after_reopen() {
    let f = F::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q.sqlite");
    let q = WitnessQueue::open(&path, 4).unwrap();
    let m = f.msg("m");
    assert_eq!(
        q.receive(&f.verified(), &m, now()).unwrap(),
        Receive::Inserted
    );
    assert_eq!(
        q.receive(&f.verified(), &m, now()).unwrap(),
        Receive::Duplicate
    );
    let w = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
    let id = q
        .execute(&w, &f.executor(), &f.scope(), &f.keys, now())
        .await
        .unwrap();
    assert_eq!(
        q.deliveries(now(), &f.scope(), &f.keys, 4).unwrap().len(),
        1
    );
    assert!(q.retry(&id).unwrap());
    drop(q);
    let q = WitnessQueue::open(&path, 4).unwrap();
    assert_eq!(
        q.deliveries(now(), &f.scope(), &f.keys, 4).unwrap()[0].attempts,
        1
    );
    assert!(q.acknowledge(&id).unwrap());
    assert!(
        q.deliveries(now(), &f.scope(), &f.keys, 4)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn conflicts_fences_expiry_and_crashed_leases_are_fail_closed() {
    let f = F::new();
    let dir = tempfile::tempdir().unwrap();
    let q = WitnessQueue::open(dir.path().join("q"), 4).unwrap();
    let m = f.msg("m");
    q.receive(&f.verified(), &m, now()).unwrap();
    let mut conflict = m.clone();
    conflict.created_at_ms = conflict.created_at_ms.saturating_sub(1);
    assert!(matches!(
        q.receive(&f.verified(), &conflict, now()),
        Err(QueueError::Conflict)
    ));
    let w = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
    drop(w);
    assert_eq!(q.recover().unwrap(), 1);
    assert!(q.claim(now(), &f.scope(), &f.keys).unwrap().is_some());
    let mut stale = f.msg("stale");
    stale.device.fencing_generation = 3;
    assert!(matches!(
        q.receive(&f.verified(), &stale, now()),
        Err(QueueError::Invalid)
    ));
    let mut expired = f.msg("expired");
    expired.expires_at_ms = now() - 1;
    assert!(matches!(
        q.receive(&f.verified(), &expired, now()),
        Err(QueueError::Invalid)
    ));
}

#[test]
fn schema_is_versioned_and_configuration_is_bounded() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        WitnessQueue::open(dir.path().join("zero"), 0),
        Err(QueueError::Invalid)
    ));
    assert!(matches!(
        WitnessQueue::open(":memory:", 1),
        Err(QueueError::Invalid)
    ));
    let path = dir.path().join("schema");
    let q = WitnessQueue::open(&path, 1).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    c.pragma_update(None, "foreign_keys", true).unwrap();
    assert!(
        c.execute("INSERT INTO jobs VALUES('missing','p','pending',NULL)", [])
            .is_err()
    );
    drop(q);
    c.pragma_update(None, "user_version", 99).unwrap();
    assert!(matches!(
        WitnessQueue::open(&path, 1),
        Err(QueueError::Invalid)
    ));

    // A failed migration must not leave half-created tables or a bumped version.
    let path = dir.path().join("legacy");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("CREATE TABLE jobs(old TEXT)", []).unwrap();
    assert!(WitnessQueue::open(&path, 1).is_err());
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='assignments'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

impl F {
    fn advance(&mut self, fence: u64, epoch: u64) {
        self.d.fencing_generation = fence;
        self.p.device = self.d.clone();
        self.a.device = self.d.clone();
        self.a.probes = vec![self.p.clone()];
        self.a.plan_epoch = epoch;
    }
}

#[test]
fn pending_capacity_retires_expired_and_superseded_work() {
    let mut f = F::new();
    let dir = tempfile::tempdir().unwrap();
    let q = WitnessQueue::open(dir.path().join("q"), 1).unwrap();
    let n = now() + 10_000;
    let mut expiring = f.msg("expire");
    expiring.expires_at_ms = n + 10;
    q.receive(&f.verified(), &expiring, n).unwrap();
    let replacement = f.msg("replacement");
    q.receive(&f.verified(), &replacement, n + 10).unwrap();
    f.advance(4, 2);
    let newest = f.msg("newest");
    q.receive(&f.verified(), &newest, n + 11).unwrap();
    let work = q.claim(n + 11, &f.scope(), &f.keys).unwrap().unwrap();
    assert_eq!(work.message_id, "newest");
    f.advance(4, 1);
    assert!(q.claim(n + 11, &f.scope(), &f.keys).unwrap().is_none());
    assert!(matches!(
        q.receive(&f.verified(), &f.msg("older"), n + 11),
        Err(QueueError::Stale)
    ));
    // A higher fence starts a new owner even when its epoch is lower.
    f.advance(5, 1);
    q.receive(&f.verified(), &f.msg("new-owner"), n + 11)
        .unwrap();
    assert_eq!(
        q.claim(n + 11, &f.scope(), &f.keys)
            .unwrap()
            .unwrap()
            .message_id,
        "new-owner"
    );
}

#[tokio::test]
async fn live_work_and_unacked_evidence_share_capacity_and_keep_tombstones() {
    let mut f = F::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q");
    let q = WitnessQueue::open(&path, 2).unwrap();
    let first = f.msg("first");
    q.receive(&f.verified(), &first, now()).unwrap();
    let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
    let id = q
        .execute(&work, &f.executor(), &f.scope(), &f.keys, now())
        .await
        .unwrap();
    q.receive(&f.verified(), &f.msg("pending"), now()).unwrap();
    assert!(matches!(
        q.receive(&f.verified(), &f.msg("full"), now()),
        Err(QueueError::Full)
    ));
    assert!(q.acknowledge(&id).unwrap());
    q.receive(&f.verified(), &f.msg("space"), now()).unwrap();
    assert_eq!(
        q.receive(&f.verified(), &first, now()).unwrap(),
        Receive::Duplicate
    );
    f.advance(5, 1);
    q.receive(&f.verified(), &f.msg("fenced"), now()).unwrap();
    assert!(
        q.deliveries(now(), &f.scope(), &f.keys, 10)
            .unwrap()
            .is_empty()
    );
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM assignments", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM outbox", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn recovery_tokens_and_evidence_ids_do_not_depend_on_time_or_identifier_lengths() {
    let mut f = F::new();
    f.p.id = "p".repeat(400);
    f.a.probes = vec![f.p.clone()];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q");
    let q = WitnessQueue::open(&path, 1).unwrap();
    q.receive(&f.verified(), &f.msg(&"m".repeat(256)), now())
        .unwrap();
    let n = now();
    let old = q.claim(n, &f.scope(), &f.keys).unwrap().unwrap();
    drop(q);
    let q = WitnessQueue::open(&path, 1).unwrap();
    assert_eq!(q.recover().unwrap(), 1);
    let new = q.claim(n, &f.scope(), &f.keys).unwrap().unwrap();
    assert_ne!(old.lease, new.lease);
    assert_eq!(new.lease.len(), 64);
    assert!(matches!(
        q.execute(&old, &f.executor(), &f.scope(), &f.keys, n).await,
        Err(QueueError::Stale)
    ));
    let mut forged = new.clone();
    forged.lease = "fake".into();
    assert!(matches!(
        q.execute(&forged, &f.executor(), &f.scope(), &f.keys, n)
            .await,
        Err(QueueError::Stale)
    ));
    let id = q
        .execute(&new, &f.executor(), &f.scope(), &f.keys, n)
        .await
        .unwrap();
    assert_eq!(id.len(), 64);
    assert!(matches!(
        q.execute(&new, &f.executor(), &f.scope(), &f.keys, n).await,
        Err(QueueError::Stale)
    ));
}

#[test]
fn a_different_scope_at_the_head_cannot_starve_current_work() {
    let f = F::new();
    let other = F::new(); // Same device, distinct signed revision scope.
    let dir = tempfile::tempdir().unwrap();
    let q = WitnessQueue::open(dir.path().join("q"), 4).unwrap();
    let mut other = other;
    other.r.id = "other-revision".into();
    other.a.revision = other.r.clone();
    q.receive(&other.verified(), &other.msg("head"), now())
        .unwrap();
    q.receive(&f.verified(), &f.msg("current"), now()).unwrap();
    assert_eq!(
        q.claim(now(), &f.scope(), &f.keys)
            .unwrap()
            .unwrap()
            .message_id,
        "current"
    );
    assert_eq!(
        q.claim(now(), &other.scope(), &other.keys)
            .unwrap()
            .unwrap()
            .message_id,
        "head"
    );
}

#[test]
fn sealed_assignment_time_is_checked_again_at_receive() {
    let f = F::new();
    let sealed = f.verified();
    let dir = tempfile::tempdir().unwrap();
    let q = WitnessQueue::open(dir.path().join("q"), 1).unwrap();
    let mut msg = f.msg("early");
    msg.created_at_ms = f.a.issued_at_ms - 2;
    assert!(matches!(
        q.receive(&sealed, &msg, f.a.issued_at_ms - 1),
        Err(QueueError::Invalid)
    ));
    msg.expires_at_ms = f.a.expires_at_ms + 1;
    assert!(matches!(
        q.receive(&sealed, &msg, f.a.expires_at_ms),
        Err(QueueError::Invalid)
    ));
}

#[tokio::test]
async fn delivery_retires_expired_or_superseded_evidence_without_deleting_it() {
    let mut f = F::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q");
    let q = WitnessQueue::open(&path, 1).unwrap();
    let msg = f.msg("first");
    q.receive(&f.verified(), &msg, now()).unwrap();
    let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
    q.execute(&work, &f.executor(), &f.scope(), &f.keys, now())
        .await
        .unwrap();
    f.advance(5, 1);
    // Retirement happens before capacity, even with a still-unacked outbox.
    q.receive(&f.verified(), &f.msg("second"), now()).unwrap();
    assert!(
        q.deliveries(now(), &f.scope(), &f.keys, 10)
            .unwrap()
            .is_empty()
    );
    let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
    q.execute(&work, &f.executor(), &f.scope(), &f.keys, now())
        .await
        .unwrap();
    assert_eq!(
        q.deliveries(now(), &f.scope(), &f.keys, 10).unwrap().len(),
        1
    );
    assert!(
        q.deliveries(f.a.expires_at_ms, &f.scope(), &f.keys, 10)
            .unwrap()
            .is_empty()
    );
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM outbox WHERE retired=1", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn claims_and_delivery_recheck_address_scope_and_current_trust() {
    let mut f = F::new();
    let dir = tempfile::tempdir().unwrap();
    let q = WitnessQueue::open(dir.path().join("q"), 2).unwrap();
    q.receive(&f.verified(), &f.msg("old-address"), now())
        .unwrap();
    f.ips = vec!["192.0.2.3".parse().unwrap()];
    f.p.source.bind_address = f.ips[0];
    f.a.probes = vec![f.p.clone()];
    q.receive(&f.verified(), &f.msg("new-address"), now())
        .unwrap();
    let work = q.claim(now(), &f.scope(), &f.keys).unwrap().unwrap();
    assert_eq!(work.message_id, "new-address");
    q.execute(&work, &f.executor(), &f.scope(), &f.keys, now())
        .await
        .unwrap();
    let deliveries = q.deliveries(now(), &f.scope(), &f.keys, 10).unwrap();
    assert_eq!(deliveries.len(), 1);
    let wk = SigningKey::from_pkcs8(&f.wpk).unwrap();
    let verified = intent_identity::evidence::verify_evidence(
        &deliveries[0].envelope,
        &[trust(&f.w, &wk)],
        &f.verified(),
        now(),
    )
    .unwrap();
    assert_eq!(
        verified.evidence().result.outcome,
        intent_protocol::assurance::Outcome::Unsupported
    );
    let mut revoked = f.keys.clone();
    revoked[0].revoked = true;
    assert!(
        q.deliveries(now(), &f.scope(), &revoked, 10)
            .unwrap()
            .is_empty()
    );
    f.ips = vec!["192.0.2.2".parse().unwrap()];
    assert!(
        q.deliveries(now(), &f.scope(), &f.keys, 10)
            .unwrap()
            .is_empty()
    );
}
