use intent_authorization::{AuthorizationEngine, ControllerGrant, ControllerScope, TrustedBinding};
use intent_controller::{
    health::HealthState,
    ingress::{
        EvidenceIngress, EvidenceSubmission, IngressError, ProcessingHook, Result as IngressResult,
    },
    monitor::MonitorStore,
    storage::Store,
};
use intent_identity::{
    evidence::{probe_digest, AssignmentScope},
    AgentIdentity, DsseEnvelope, SigningKey, TrustedKey,
};
use intent_protocol::{
    assurance::{
        Endpoint, Expectation, IpFamily, Outcome, Primitive, ProbeResult, ProbeSource, ProbeSpec,
        ResourceLimits,
    },
    evidence::{
        revision_subject, Statement, WitnessAssignment, WitnessEvidence, STATEMENT_TYPE,
        WITNESS_ASSIGNMENT_TYPE, WITNESS_EVIDENCE_TYPE,
    },
    payload_digest,
    state::Completeness,
    DeviceBinding, MessageEnvelope, RevisionRef, PROTOCOL_VERSION,
};
use rusqlite::params;
use std::net::IpAddr;
use tempfile::TempDir;

const NOW: u64 = 5_000;

struct Fixture {
    _dir: TempDir,
    inbox: Store,
    monitor: MonitorStore,
    controller: AgentIdentity,
    witness: AgentIdentity,
    controller_key: SigningKey,
    witness_key: SigningKey,
    keys: Vec<TrustedKey>,
    device: DeviceBinding,
    revision: RevisionRef,
    probe: ProbeSpec,
    addresses: Vec<IpAddr>,
    authorization: AuthorizationEngine,
}

impl Fixture {
    fn new(authorize: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let inbox = Store::open(dir.path().join("inbox.sqlite"), 8).unwrap();
        let monitor_path = dir.path().join("monitor.sqlite");
        let monitor = MonitorStore::open(&monitor_path).unwrap();
        let controller =
            AgentIdentity::parse("spiffe://lab.local/agent/controller/router-a/controller-1")
                .unwrap();
        let witness =
            AgentIdentity::parse("spiffe://lab.local/agent/witness/router-a/witness-1").unwrap();
        let (controller_key, controller_pkcs8) = SigningKey::generate().unwrap();
        let (witness_key, witness_pkcs8) = SigningKey::generate().unwrap();
        let device = DeviceBinding {
            device_id: "router-a".into(),
            profile_digest: "profile-v1".into(),
            fencing_generation: 7,
        };
        let revision = RevisionRef {
            id: "rev-42".into(),
            source_digest: "source-42".into(),
        };
        let addresses = vec!["192.0.2.10".parse().unwrap()];
        let probe = ProbeSpec {
            id: "wan-tcp".into(),
            primitive: Primitive::TcpConnect,
            device: device.clone(),
            source: ProbeSource {
                witness_id: witness.to_string(),
                location: "lab-west".into(),
                bind_address: addresses[0],
                interface: Some("wan0".into()),
            },
            endpoint: Some(Endpoint {
                address: "198.51.100.1".parse().unwrap(),
                port: Some(443),
                family: IpFamily::V4,
            }),
            expectation: Expectation::TcpConnected,
            limits: ResourceLimits {
                timeout_ms: 1_000,
                max_response_bytes: 1_024,
                max_attempts: 1,
            },
            interval_ms: 10_000,
            depends_on: vec![],
            claims: vec![],
        };
        seed_monitor(&monitor_path, &probe, &revision, NOW);
        let grants = if authorize {
            vec![ControllerGrant {
                identity: witness.clone(),
                device_id: device.device_id.clone(),
                profile_digest: device.profile_digest.clone(),
                fencing_generation: device.fencing_generation,
            }]
        } else {
            vec![]
        };
        let authorization = AuthorizationEngine::new(
            ControllerScope::new(grants, vec![TrustedBinding(device.clone())]).unwrap(),
        )
        .unwrap();
        Self {
            _dir: dir,
            inbox,
            monitor,
            controller: controller.clone(),
            witness: witness.clone(),
            controller_key,
            witness_key,
            keys: vec![
                trusted(&controller, &controller_pkcs8),
                trusted(&witness, &witness_pkcs8),
            ],
            device,
            revision,
            probe,
            addresses,
            authorization,
        }
    }
    fn scope(&self) -> AssignmentScope<'_> {
        AssignmentScope {
            controller: &self.controller,
            witness: &self.witness,
            device: &self.device,
            revision: &self.revision,
            plan_id: "plan-7",
            plan_epoch: 9,
            graph_version: 3,
            location: "lab-west",
            bind_addresses: &self.addresses,
        }
    }
    fn ingress(&self) -> EvidenceIngress<'_> {
        EvidenceIngress::new(
            &self.inbox,
            &self.monitor,
            &self.authorization,
            &self.keys,
            self.scope(),
        )
    }
    fn envelope(&self, message_id: &str) -> MessageEnvelope {
        let assignment = WitnessAssignment {
            version: PROTOCOL_VERSION,
            assignment_id: "assignment-7".into(),
            issuer: self.controller.to_string(),
            recipient: self.witness.to_string(),
            issued_at_ms: 1_000,
            expires_at_ms: 10_000,
            plan_id: "plan-7".into(),
            plan_epoch: 9,
            graph_version: 3,
            revision: self.revision.clone(),
            device: self.device.clone(),
            probes: vec![self.probe.clone()],
        };
        let evidence = WitnessEvidence {
            version: PROTOCOL_VERSION,
            evidence_id: "evidence-7".into(),
            assignment_id: assignment.assignment_id.clone(),
            plan_id: assignment.plan_id.clone(),
            plan_epoch: assignment.plan_epoch,
            graph_version: assignment.graph_version,
            revision: assignment.revision.clone(),
            device: assignment.device.clone(),
            witness: self.witness.to_string(),
            probe_digest: probe_digest(&self.probe).unwrap(),
            completeness: Completeness::Complete,
            result: ProbeResult {
                probe_id: self.probe.id.clone(),
                outcome: Outcome::Success,
                started_at_ms: 2_000,
                finished_at_ms: 3_000,
                detail: "connected".into(),
            },
        };
        let payload = serde_json::to_vec(&EvidenceSubmission {
            assignment: sign(
                &self.controller_key,
                Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&self.revision),
                    predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                    predicate: assignment,
                },
            ),
            evidence: sign(
                &self.witness_key,
                Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&self.revision),
                    predicate_type: WITNESS_EVIDENCE_TYPE.into(),
                    predicate: evidence,
                },
            ),
        })
        .unwrap();
        MessageEnvelope {
            version: PROTOCOL_VERSION,
            message_id: message_id.into(),
            sender: self.witness.to_string(),
            recipient: self.controller.to_string(),
            device: self.device.clone(),
            plan_epoch: 9,
            created_at_ms: 1_500,
            expires_at_ms: 9_000,
            payload_digest: payload_digest(&payload),
            payload,
        }
    }
}

fn trusted(identity: &AgentIdentity, pkcs8: &[u8]) -> TrustedKey {
    TrustedKey {
        identity: identity.clone(),
        public_key: SigningKey::from_pkcs8(pkcs8).unwrap().public_key(),
        not_before_ms: 0,
        not_after_ms: 20_000,
        revoked: false,
    }
}
fn sign<T: serde::Serialize>(key: &SigningKey, statement: Statement<T>) -> DsseEnvelope {
    key.sign(&serde_json::to_vec(&statement).unwrap()).unwrap()
}

// The production activation boundary accepts only a compiler-admitted revision.
// This temporary database fixture seeds the already-migrated monitor schema so
// ingress can exercise its real durable ingestion path without weakening that boundary.
fn seed_monitor(path: &std::path::Path, probe: &ProbeSpec, revision: &RevisionRef, now: u64) {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection
        .execute(
            "INSERT INTO monitor_plans VALUES('plan-7',9,3,?1,?2,'fixture',1,?3)",
            params![revision.id, revision.source_digest, now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO monitor_probes VALUES('plan-7',9,?1,3,?2,?3,?4,NULL,NULL,0,0)",
            params![
                probe.id,
                serde_json::to_string(probe).unwrap(),
                probe_digest(probe).unwrap(),
                now
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO monitor_health VALUES('plan-7',9,?1,?2)",
            params![
                probe.id,
                serde_json::to_string(&HealthState::new("plan-7".into(), 9, 3, probe.id.clone()))
                    .unwrap()
            ],
        )
        .unwrap();
}

struct CrashOnce;
impl ProcessingHook for CrashOnce {
    fn after_monitor_ingest(&mut self, _: &str) -> IngressResult<()> {
        Err(IngressError::InterruptedAfterIngest)
    }
}

#[test]
fn signed_evidence_is_durable_before_ack_and_replay_after_crash_is_idempotent() {
    let fixture = Fixture::new(true);
    let message = fixture.envelope("message-7");
    assert_eq!(
        fixture
            .ingress()
            .receive(&fixture.witness, &message, NOW)
            .unwrap(),
        intent_controller::ingress::ReceiptOutcome::Inserted
    );
    let mut crash = CrashOnce;
    assert!(matches!(
        fixture
            .ingress()
            .process_pending_with_hook(NOW, 1, &mut crash),
        Err(IngressError::InterruptedAfterIngest)
    ));
    assert_eq!(fixture.inbox.pending_inbox(NOW, 2).unwrap().len(), 1);
    assert_eq!(fixture.ingress().process_pending(NOW, 2).unwrap(), 1);
    assert!(fixture.inbox.pending_inbox(NOW, 2).unwrap().is_empty());
    let audit = fixture
        .monitor
        .audit_evidence("evidence-7")
        .unwrap()
        .unwrap();
    let submitted: EvidenceSubmission = serde_json::from_slice(&message.payload).unwrap();
    assert_eq!(audit.evidence_envelope, submitted.evidence);
    assert_eq!(audit.assignment_envelope, submitted.assignment);
}

#[test]
fn receipt_rejects_unauthorized_peer_scope_and_forged_evidence_before_durability() {
    let unauthorized = Fixture::new(false);
    let m = unauthorized.envelope("denied");
    assert!(matches!(
        unauthorized
            .ingress()
            .receive(&unauthorized.witness, &m, NOW),
        Err(IngressError::Unauthorized)
    ));
    assert!(unauthorized.inbox.pending_inbox(NOW, 2).unwrap().is_empty());

    let fixture = Fixture::new(true);
    let mut wrong_sender = fixture.envelope("wrong-sender");
    wrong_sender.sender = fixture.controller.to_string();
    assert!(matches!(
        fixture
            .ingress()
            .receive(&fixture.witness, &wrong_sender, NOW),
        Err(IngressError::SenderMismatch)
    ));
    let mut stale = fixture.envelope("stale");
    stale.plan_epoch = 8;
    assert!(matches!(
        fixture.ingress().receive(&fixture.witness, &stale, NOW),
        Err(IngressError::MessageScopeMismatch)
    ));
    let mut forged = fixture.envelope("forged");
    let mut body: EvidenceSubmission = serde_json::from_slice(&forged.payload).unwrap();
    body.evidence.signatures[0].sig =
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="
            .into();
    forged.payload = serde_json::to_vec(&body).unwrap();
    forged.payload_digest = payload_digest(&forged.payload);
    assert!(matches!(
        fixture.ingress().receive(&fixture.witness, &forged, NOW),
        Err(IngressError::Evidence(_))
    ));
    assert!(fixture.inbox.pending_inbox(NOW, 4).unwrap().is_empty());
}

#[test]
fn route_filtered_replay_cannot_be_starved_by_a_different_inbox_route() {
    let fixture = Fixture::new(true);
    let mut other_route = fixture.envelope("aaa-other-route");
    other_route.sender = "spiffe://lab.local/agent/witness/router-b/witness-1".into();
    other_route.recipient = "spiffe://lab.local/agent/controller/router-b/controller-1".into();
    other_route.device = DeviceBinding {
        device_id: "router-b".into(),
        profile_digest: "profile-v1".into(),
        fencing_generation: 1,
    };
    other_route.plan_epoch = 1;
    fixture.inbox.receive(&other_route, NOW).unwrap();
    let ours = fixture.envelope("zzz-our-route");
    fixture
        .ingress()
        .receive(&fixture.witness, &ours, NOW)
        .unwrap();

    assert_eq!(fixture.ingress().process_pending(NOW, 1).unwrap(), 1);
    let pending = fixture.inbox.pending_inbox(NOW, 4).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].envelope.message_id, "aaa-other-route");
}

#[test]
fn malformed_outer_digest_and_oversized_payload_are_rejected_before_signature_parsing() {
    let fixture = Fixture::new(true);
    let mut bad_digest = fixture.envelope("bad-digest");
    bad_digest.payload_digest = "not-the-payload-digest".into();
    assert!(matches!(
        fixture
            .ingress()
            .receive(&fixture.witness, &bad_digest, NOW),
        Err(IngressError::Storage(
            intent_controller::storage::StorageError::InvalidMessage(
                "payload digest does not match exact payload bytes"
            )
        ))
    ));

    let mut oversized = fixture.envelope("oversized");
    oversized.payload = vec![0; 1_048_577];
    oversized.payload_digest = payload_digest(&oversized.payload);
    assert!(matches!(
        fixture.ingress().receive(&fixture.witness, &oversized, NOW),
        Err(IngressError::Storage(
            intent_controller::storage::StorageError::InvalidMessage(
                "payload exceeds storage limit"
            )
        ))
    ));
    assert!(fixture.inbox.pending_inbox(NOW, 4).unwrap().is_empty());
}
