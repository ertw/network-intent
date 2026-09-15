use intent_identity::{
    evidence::{verify_assignment, verify_evidence, AssignmentScope},
    AgentIdentity, DsseEnvelope, SigningKey, TrustedKey,
};
use intent_protocol::{
    assurance::{
        Endpoint, Expectation, IpFamily, Primitive, ProbeSource, ProbeSpec, ResourceLimits,
    },
    evidence::{
        revision_subject, Statement, WitnessAssignment, STATEMENT_TYPE, WITNESS_ASSIGNMENT_TYPE,
    },
    state::Completeness,
    DeviceBinding, RevisionRef, PROTOCOL_VERSION,
};
use intent_witness_agent::{
    execution::{ExecutionError, WitnessExecutor},
    local::LocalObservationContext,
};
use std::net::{IpAddr, Ipv4Addr};

struct Fixture {
    controller: AgentIdentity,
    witness: AgentIdentity,
    controller_key: SigningKey,
    witness_pkcs8: Vec<u8>,
    controller_trusted: TrustedKey,
    witness_trusted: TrustedKey,
    device: DeviceBinding,
    revision: RevisionRef,
    assignment: WitnessAssignment,
    source: ProbeSource,
    bind_addresses: Vec<IpAddr>,
}
fn ip(value: &str) -> IpAddr {
    value.parse::<Ipv4Addr>().unwrap().into()
}
fn trusted(identity: &AgentIdentity, pkcs8: &[u8]) -> TrustedKey {
    TrustedKey {
        identity: identity.clone(),
        public_key: SigningKey::from_pkcs8(pkcs8).unwrap().public_key(),
        not_before_ms: 0,
        not_after_ms: u64::MAX,
        revoked: false,
    }
}
fn signed<T: serde::Serialize>(key: &SigningKey, value: &T) -> DsseEnvelope {
    key.sign(&serde_json::to_vec(value).unwrap()).unwrap()
}
impl Fixture {
    fn new(expired: bool) -> Self {
        let controller =
            AgentIdentity::parse("spiffe://lab.local/agent/controller/router-a/controller-1")
                .unwrap();
        let witness =
            AgentIdentity::parse("spiffe://lab.local/agent/witness/router-a/witness-1").unwrap();
        let (controller_key, controller_pkcs8) = SigningKey::generate().unwrap();
        let (_witness_key, witness_pkcs8) = SigningKey::generate().unwrap();
        let device = DeviceBinding {
            device_id: "router-a".into(),
            profile_digest: "profile-a".into(),
            fencing_generation: 4,
        };
        let revision = RevisionRef {
            id: "rev-1".into(),
            source_digest: "source-1".into(),
        };
        let source = ProbeSource {
            witness_id: witness.to_string(),
            location: "lab".into(),
            bind_address: ip("192.0.2.2"),
            interface: Some("wan0".into()),
        };
        let probe = ProbeSpec {
            id: "probe-unsupported".into(),
            primitive: Primitive::TcpConnect,
            device: device.clone(),
            source: source.clone(),
            endpoint: Some(Endpoint {
                address: ip("198.51.100.1"),
                port: Some(443),
                family: IpFamily::V4,
            }),
            expectation: Expectation::TcpConnected,
            limits: ResourceLimits {
                timeout_ms: 1000,
                max_response_bytes: 1024,
                max_attempts: 1,
            },
            interval_ms: 30_000,
            depends_on: vec![],
            claims: vec!["claim".into()],
        };
        let now = now_ms();
        let assignment = WitnessAssignment {
            version: PROTOCOL_VERSION,
            assignment_id: "assignment-1".into(),
            issuer: controller.to_string(),
            recipient: witness.to_string(),
            issued_at_ms: if expired { 1 } else { now.saturating_sub(1000) },
            expires_at_ms: if expired {
                2
            } else {
                now.saturating_add(120_000)
            },
            plan_id: "plan-1".into(),
            plan_epoch: 1,
            graph_version: 1,
            revision: revision.clone(),
            device: device.clone(),
            probes: vec![probe],
        };
        Self {
            controller: controller.clone(),
            witness: witness.clone(),
            controller_key,
            controller_trusted: trusted(&controller, &controller_pkcs8),
            witness_trusted: trusted(&witness, &witness_pkcs8),
            witness_pkcs8,
            device,
            revision,
            assignment,
            source,
            bind_addresses: vec![ip("192.0.2.2")],
        }
    }
    fn scope(&self) -> AssignmentScope<'_> {
        AssignmentScope {
            controller: &self.controller,
            witness: &self.witness,
            device: &self.device,
            revision: &self.revision,
            plan_id: "plan-1",
            plan_epoch: 1,
            graph_version: 1,
            location: "lab",
            bind_addresses: &self.bind_addresses,
        }
    }
    fn verified(&self, now: u64) -> intent_identity::evidence::VerifiedAssignment {
        let statement = Statement {
            statement_type: STATEMENT_TYPE.into(),
            subject: revision_subject(&self.revision),
            predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
            predicate: self.assignment.clone(),
        };
        verify_assignment(
            &signed(&self.controller_key, &statement),
            &[self.controller_trusted.clone()],
            &self.scope(),
            now,
        )
        .unwrap()
    }
    fn executor(&self) -> WitnessExecutor {
        WitnessExecutor::new(
            self.witness.clone(),
            SigningKey::from_pkcs8(&self.witness_pkcs8).unwrap(),
            LocalObservationContext {
                device: self.device.clone(),
                source: self.source.clone(),
                rpcd_session: "session-a".into(),
            },
        )
        .unwrap()
    }
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[tokio::test]
async fn signed_assignment_executes_safe_unsupported_probe_and_verifies_evidence() {
    let f = Fixture::new(false);
    let verified = f.verified(now_ms());
    let envelope = f
        .executor()
        .run_probe(&verified, "probe-unsupported", "evidence-1")
        .await
        .unwrap();
    let evidence =
        verify_evidence(&envelope, &[f.witness_trusted.clone()], &verified, now_ms()).unwrap();
    assert_eq!(
        evidence.evidence().result.outcome,
        intent_protocol::assurance::Outcome::Unsupported
    );
    assert!(matches!(
        evidence.evidence().completeness,
        Completeness::Unavailable { .. }
    ));
    assert_eq!(
        evidence.outcome_at(now_ms()),
        intent_protocol::assurance::Outcome::Unavailable
    );
}

#[tokio::test]
async fn executor_rejects_scope_and_unknown_ids_before_execution() {
    let f = Fixture::new(false);
    let verified = f.verified(now_ms());
    assert!(matches!(
        f.executor()
            .run_probe(&verified, "unknown", "evidence-1")
            .await,
        Err(ExecutionError::Scope)
    ));
    assert!(matches!(
        f.executor()
            .run_probe(&verified, "probe-unsupported", "")
            .await,
        Err(ExecutionError::Scope)
    ));
    let apply = AgentIdentity::parse("spiffe://lab.local/agent/apply/router-a/apply-1").unwrap();
    let (key, _) = SigningKey::generate().unwrap();
    assert!(matches!(
        WitnessExecutor::new(
            apply,
            key,
            LocalObservationContext {
                device: f.device.clone(),
                source: f.source.clone(),
                rpcd_session: "session-a".into()
            }
        ),
        Err(ExecutionError::Scope)
    ));
    let mut wrong_source = f.source.clone();
    wrong_source.witness_id = "spiffe://lab.local/agent/witness/router-a/other".into();
    let (key, _) = SigningKey::generate().unwrap();
    assert!(matches!(
        WitnessExecutor::new(
            f.witness.clone(),
            key,
            LocalObservationContext {
                device: f.device.clone(),
                source: wrong_source,
                rpcd_session: "session-a".into()
            }
        ),
        Err(ExecutionError::Scope)
    ));
}

#[tokio::test]
async fn assignment_lifetime_is_checked_again_at_execution() {
    let f = Fixture::new(true);
    let verified = f.verified(1);
    assert!(matches!(
        f.executor()
            .run_probe(&verified, "probe-unsupported", "evidence-1")
            .await,
        Err(ExecutionError::Expired)
    ));
}
