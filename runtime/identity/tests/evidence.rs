use base64::{engine::general_purpose::STANDARD, Engine};
use intent_identity::{
    evidence::{probe_digest, verify_assignment, verify_evidence, AssignmentScope},
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
    state::Completeness,
    DeviceBinding, RevisionRef, PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr};

const NOW: u64 = 5_000;

struct Fixture {
    controller: AgentIdentity,
    witness: AgentIdentity,
    apply: AgentIdentity,
    controller_key: SigningKey,
    witness_key: SigningKey,
    apply_key: SigningKey,
    controller_trusted: TrustedKey,
    witness_trusted: TrustedKey,
    apply_trusted: TrustedKey,
    device: DeviceBinding,
    revision: RevisionRef,
    probe: ProbeSpec,
    bind_addresses: Vec<IpAddr>,
}

impl Fixture {
    fn new() -> Self {
        let controller =
            AgentIdentity::parse("spiffe://lab.local/agent/controller/router-a/controller-1")
                .unwrap();
        let witness =
            AgentIdentity::parse("spiffe://lab.local/agent/witness/router-a/witness-1").unwrap();
        let apply =
            AgentIdentity::parse("spiffe://lab.local/agent/apply/router-a/apply-1").unwrap();
        let (controller_key, controller_pkcs8) = SigningKey::generate().unwrap();
        let (witness_key, witness_pkcs8) = SigningKey::generate().unwrap();
        let (apply_key, apply_pkcs8) = SigningKey::generate().unwrap();
        let controller_trusted = trusted(&controller, &controller_pkcs8);
        let witness_trusted = trusted(&witness, &witness_pkcs8);
        let apply_trusted = trusted(&apply, &apply_pkcs8);
        let device = DeviceBinding {
            device_id: "router-a".into(),
            profile_digest: "profile-v1".into(),
            fencing_generation: 7,
        };
        let revision = RevisionRef {
            id: "rev-42".into(),
            source_digest: "source-42".into(),
        };
        let probe = ProbeSpec {
            id: "wan-tcp".into(),
            primitive: Primitive::TcpConnect,
            device: device.clone(),
            source: ProbeSource {
                witness_id: witness.to_string(),
                location: "lab-west".into(),
                bind_address: ip("192.0.2.10"),
                interface: Some("wan0".into()),
            },
            endpoint: Some(Endpoint {
                address: ip("198.51.100.1"),
                port: Some(443),
                family: IpFamily::V4,
            }),
            expectation: Expectation::TcpConnected,
            limits: ResourceLimits {
                timeout_ms: 1_000,
                max_response_bytes: 1_024,
                max_attempts: 2,
            },
            interval_ms: 10_000,
            depends_on: vec![],
            claims: vec!["claim-wan".into()],
        };
        Self {
            controller,
            witness,
            apply,
            controller_key,
            witness_key,
            apply_key,
            controller_trusted,
            witness_trusted,
            apply_trusted,
            device,
            revision,
            probe,
            bind_addresses: vec![ip("192.0.2.10")],
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
            bind_addresses: &self.bind_addresses,
        }
    }

    fn assignment(&self) -> WitnessAssignment {
        WitnessAssignment {
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
        }
    }

    fn assignment_envelope(&self, assignment: &WitnessAssignment) -> DsseEnvelope {
        signed(
            &self.controller_key,
            &Statement {
                statement_type: STATEMENT_TYPE.into(),
                subject: revision_subject(&self.revision),
                predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                predicate: assignment.clone(),
            },
        )
    }

    fn verified_assignment(&self) -> intent_identity::evidence::VerifiedAssignment {
        verify_assignment(
            &self.assignment_envelope(&self.assignment()),
            &[self.controller_trusted.clone()],
            &self.scope(),
            NOW,
        )
        .unwrap()
    }

    fn evidence(&self, assignment: &WitnessAssignment) -> WitnessEvidence {
        WitnessEvidence {
            version: PROTOCOL_VERSION,
            evidence_id: "evidence-7".into(),
            assignment_id: assignment.assignment_id.clone(),
            plan_id: assignment.plan_id.clone(),
            plan_epoch: assignment.plan_epoch,
            graph_version: assignment.graph_version,
            revision: assignment.revision.clone(),
            device: assignment.device.clone(),
            witness: assignment.recipient.clone(),
            probe_digest: probe_digest(&assignment.probes[0]).unwrap(),
            completeness: Completeness::Complete,
            result: ProbeResult {
                probe_id: "wan-tcp".into(),
                outcome: Outcome::Success,
                started_at_ms: 2_000,
                finished_at_ms: 3_000,
                detail: "connected".into(),
            },
        }
    }

    fn evidence_envelope(&self, evidence: &WitnessEvidence) -> DsseEnvelope {
        signed(
            &self.witness_key,
            &Statement {
                statement_type: STATEMENT_TYPE.into(),
                subject: revision_subject(&self.revision),
                predicate_type: WITNESS_EVIDENCE_TYPE.into(),
                predicate: evidence.clone(),
            },
        )
    }
}

fn ip(s: &str) -> IpAddr {
    s.parse::<Ipv4Addr>().unwrap().into()
}

fn trusted(identity: &AgentIdentity, pkcs8: &[u8]) -> TrustedKey {
    let key = SigningKey::from_pkcs8(pkcs8).unwrap();
    TrustedKey {
        identity: identity.clone(),
        public_key: key.public_key(),
        not_before_ms: 0,
        not_after_ms: 20_000,
        revoked: false,
    }
}

fn signed<T: serde::Serialize>(key: &SigningKey, statement: &Statement<T>) -> DsseEnvelope {
    key.sign(&serde_json::to_vec(statement).unwrap()).unwrap()
}

fn mutate_json<T: serde::Serialize>(
    statement: &Statement<T>,
    path: &[&str],
    value: Value,
) -> Value {
    let mut json = serde_json::to_value(statement).unwrap();
    let mut target = &mut json;
    for part in &path[..path.len() - 1] {
        if let Some((name, index)) = part.split_once('[') {
            let index = index.trim_end_matches(']').parse::<usize>().unwrap();
            target = target.get_mut(name).unwrap().get_mut(index).unwrap();
        } else {
            target = target.get_mut(*part).unwrap();
        }
    }
    target[path[path.len() - 1]] = value;
    json
}

fn signed_json(key: &SigningKey, value: &Value) -> DsseEnvelope {
    key.sign(&serde_json::to_vec(value).unwrap()).unwrap()
}

#[test]
fn independently_signed_assignment_and_evidence_are_accepted() {
    let f = Fixture::new();
    let assignment = f.verified_assignment();
    let evidence = f.evidence(assignment.assignment());
    let verified = verify_evidence(
        &f.evidence_envelope(&evidence),
        &[f.witness_trusted.clone()],
        &assignment,
        NOW,
    )
    .unwrap();
    assert_eq!(verified.outcome_at(NOW), Outcome::Success);
}

#[test]
fn assignment_scope_rejects_wrong_role_and_every_binding_field() {
    let f = Fixture::new();
    let base = f.assignment();
    let cases = [
        ("predicate", "predicate", json!({"version": 99})),
        ("device", "predicate.device.device_id", json!("other")),
        ("revision", "predicate.revision.id", json!("other-revision")),
        ("epoch", "predicate.plan_epoch", json!(10)),
        (
            "recipient",
            "predicate.recipient",
            json!(f.apply.to_string()),
        ),
        (
            "location",
            "predicate.probes[0].source.location",
            json!("other-location"),
        ),
        (
            "source",
            "predicate.probes[0].source.bind_address",
            json!("192.0.2.11"),
        ),
    ];
    for (label, path, value) in cases {
        let path: Vec<_> = path.split('.').collect();
        let value = if label == "predicate" {
            let mut statement = serde_json::to_value(&Statement {
                statement_type: STATEMENT_TYPE.into(),
                subject: revision_subject(&f.revision),
                predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                predicate: base.clone(),
            })
            .unwrap();
            statement["predicate"] = json!({"version": 99});
            statement
        } else {
            mutate_json(
                &Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&f.revision),
                    predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                    predicate: base.clone(),
                },
                &path,
                value,
            )
        };
        assert!(
            verify_assignment(
                &signed_json(&f.controller_key, &value),
                &[f.controller_trusted.clone()],
                &f.scope(),
                NOW
            )
            .is_err(),
            "{label}"
        );
    }
    let assignment_signed_by_witness = f
        .witness_key
        .sign(
            &serde_json::to_vec(&Statement {
                statement_type: STATEMENT_TYPE.into(),
                subject: revision_subject(&f.revision),
                predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                predicate: base,
            })
            .unwrap(),
        )
        .unwrap();
    assert!(verify_assignment(
        &assignment_signed_by_witness,
        &[f.witness_trusted.clone()],
        &f.scope(),
        NOW
    )
    .is_err());
}

#[test]
fn assignment_validity_rejects_expired_and_future_statements() {
    let f = Fixture::new();
    for (issued, expires) in [(NOW + 1, 10_000), (1_000, NOW)] {
        let mut a = f.assignment();
        a.issued_at_ms = issued;
        a.expires_at_ms = expires;
        assert!(verify_assignment(
            &f.assignment_envelope(&a),
            &[f.controller_trusted.clone()],
            &f.scope(),
            NOW
        )
        .is_err());
    }
}

#[test]
fn evidence_is_bound_to_immutable_assignment_and_rejects_role_fields_and_digest() {
    let f = Fixture::new();
    let assignment = f.verified_assignment();
    let e = f.evidence(assignment.assignment());
    let statement = Statement {
        statement_type: STATEMENT_TYPE.into(),
        subject: revision_subject(&f.revision),
        predicate_type: WITNESS_EVIDENCE_TYPE.into(),
        predicate: e.clone(),
    };
    let cases = [
        ("payload", None),
        (
            "predicate",
            Some(("predicate.plan_id", json!("other-plan"))),
        ),
        (
            "subject",
            Some((
                "subject",
                json!([{"name":"other-revision","digest":{"sha256":"source-42"}}]),
            )),
        ),
        (
            "device",
            Some(("predicate.device.device_id", json!("other"))),
        ),
        ("revision", Some(("predicate.revision.id", json!("other")))),
        ("epoch", Some(("predicate.plan_epoch", json!(10)))),
        (
            "unknown-probe",
            Some(("predicate.result.probe_id", json!("not-assigned"))),
        ),
        ("probe", Some(("predicate.probe_digest", json!("deadbeef")))),
        (
            "order",
            Some(("predicate.result.started_at_ms", json!(4_000))),
        ),
        (
            "future",
            Some(("predicate.result.finished_at_ms", json!(6_000))),
        ),
    ];
    for (label, change) in cases {
        let envelope = if let Some((path, value)) = change {
            signed_json(
                &f.witness_key,
                &mutate_json(&statement, &path.split('.').collect::<Vec<_>>(), value),
            )
        } else {
            let mut changed = f.evidence_envelope(&e);
            changed.payload = STANDARD.encode(b"modified");
            changed
        };
        assert!(
            verify_evidence(&envelope, &[f.witness_trusted.clone()], &assignment, NOW).is_err(),
            "{label}"
        );
    }
    let apply_signed = signed(&f.apply_key, &statement);
    assert!(verify_evidence(&apply_signed, &[f.apply_trusted.clone()], &assignment, NOW).is_err());
}

#[test]
fn evidence_freshness_and_completeness_never_turn_unavailable_into_success() {
    let f = Fixture::new();
    let assignment = f.verified_assignment();
    for completeness in [
        Completeness::Partial {
            missing: vec!["wan-tcp".into()],
        },
        Completeness::Unavailable {
            reason: "timeout".into(),
        },
    ] {
        let mut e = f.evidence(assignment.assignment());
        e.completeness = completeness;
        let verified = verify_evidence(
            &f.evidence_envelope(&e),
            &[f.witness_trusted.clone()],
            &assignment,
            NOW,
        )
        .unwrap();
        assert_eq!(verified.outcome_at(NOW), Outcome::Unavailable);
    }
    let mut long_assignment = f.assignment();
    long_assignment.expires_at_ms = 100_000;
    let mut long_controller = f.controller_trusted.clone();
    long_controller.not_after_ms = 100_000;
    let long_assignment = verify_assignment(
        &f.assignment_envelope(&long_assignment),
        &[long_controller],
        &f.scope(),
        50_000,
    )
    .unwrap();
    let mut stale = f.evidence(long_assignment.assignment());
    stale.result.finished_at_ms = 4_000;
    let mut long_witness = f.witness_trusted.clone();
    long_witness.not_after_ms = 100_000;
    let verified = verify_evidence(
        &f.evidence_envelope(&stale),
        &[long_witness],
        &long_assignment,
        50_000,
    )
    .unwrap();
    assert_eq!(verified.outcome_at(50_000), Outcome::Unavailable);
}

#[test]
fn revoked_key_and_duplicate_json_fields_are_rejected() {
    let f = Fixture::new();
    let assignment = f.assignment_envelope(&f.assignment());
    let mut revoked = f.controller_trusted.clone();
    revoked.revoked = true;
    assert!(verify_assignment(&assignment, &[revoked], &f.scope(), NOW).is_err());
    let duplicate = br#"{"_type":"https://in-toto.io/Statement/v1","_type":"duplicate","subject":[],"predicateType":"urn:network-intent:assignment:witness:v1","predicate":{}}"#;
    let env = f.controller_key.sign(duplicate).unwrap();
    assert!(verify_assignment(&env, &[f.controller_trusted.clone()], &f.scope(), NOW).is_err());
}
