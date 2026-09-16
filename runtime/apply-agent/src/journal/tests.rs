use super::*;
use intent_identity::confirmation::{ConfirmationScope, RequiredVerification, verify_confirmation};
use intent_identity::evidence::{
    AssignmentScope, probe_digest, verify_assignment, verify_evidence,
};
use intent_identity::{AgentIdentity, SigningKey, TrustedKey};
use intent_protocol::{PROTOCOL_VERSION, assurance::*, evidence::*, state::Completeness};
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};
use tempfile::tempdir;

#[derive(Default)]
struct Fake {
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    disposition: Option<BackendDisposition>,
    checkpoint_seen: Option<String>,
    apply_window: Option<ApplyWindow>,
    confirm_expires_at_ms: Option<u64>,
}
impl Fake {
    fn call(&mut self, op: &'static str) -> std::io::Result<()> {
        self.calls.push(op);
        if self.fail == Some(op) {
            Err(std::io::Error::other("lost response"))
        } else {
            Ok(())
        }
    }
}
impl ApplyBackend for Fake {
    type Error = std::io::Error;
    fn reconcile(&mut self, _: &str) -> std::io::Result<BackendDisposition> {
        self.call("reconcile")?;
        Ok(self
            .disposition
            .clone()
            .unwrap_or(BackendDisposition::AppliedOrUnknown))
    }
    fn preflight(&mut self, _: &str) -> std::io::Result<()> {
        self.call("preflight")
    }
    fn stage(&mut self, _: &str) -> std::io::Result<()> {
        self.call("stage")
    }
    fn durable_checkpoint(&mut self, _: &str) -> std::io::Result<CheckpointReceipt> {
        self.call("checkpoint")?;
        Ok(CheckpointReceipt {
            restore_artifact_digest: "a".repeat(64),
        })
    }
    fn discard_staging(&mut self, _: &str) -> std::io::Result<()> {
        self.call("discard")
    }
    fn provisional_apply(&mut self, _: &str, _: &str, window: ApplyWindow) -> std::io::Result<()> {
        self.apply_window = Some(window);
        self.call("apply")
    }
    fn confirm(&mut self, _: &str, digest: &str, expires_at_ms: u64) -> std::io::Result<()> {
        self.checkpoint_seen = Some(digest.into());
        self.confirm_expires_at_ms = Some(expires_at_ms);
        self.call("confirm")
    }
    fn rollback(&mut self, _: &str, digest: &str) -> std::io::Result<()> {
        self.checkpoint_seen = Some(digest.into());
        self.call("rollback")
    }
}
fn req(id: &str) -> DeploymentRequest {
    DeploymentRequest {
        deployment_id: id.into(),
        device: DeviceBinding {
            device_id: "r1".into(),
            profile_digest: "profile".into(),
            fencing_generation: 4,
        },
        plan_epoch: 9,
        revision: RevisionRef {
            id: "rev".into(),
            source_digest: "src".into(),
        },
        plan_id: "plan-id".into(),
        plan_digest: "plan".into(),
        graph_version: 3,
        owned_baseline_digest: "base".into(),
        owned_baseline: vec![OwnedBaselineField {
            package: "network".into(),
            section: "lan".into(),
            field: "proto".into(),
            value_digest: Some("d".into()),
            secret_reference: None,
        }],
        confirmation_window_ms: None,
    }
}
fn ready(j: &DeploymentJournal, b: &mut Fake) {
    j.prepare(&req("a"), 0).unwrap();
    j.preflight("a", b).unwrap();
    j.stage("a", b).unwrap();
    j.durable_checkpoint("a", b).unwrap();
}
fn applied(j: &DeploymentJournal, b: &mut Fake) {
    ready(j, b);
    j.provisional_apply("a", 1_500, b).unwrap();
}
// Real controller and independent witness signatures produce the sealed token.
fn token(s: &DeploymentStatus) -> VerifiedDeploymentConfirmation {
    let controller = AgentIdentity::parse("spiffe://lab.local/agent/controller/r1/c").unwrap();
    let witness = AgentIdentity::parse("spiffe://lab.local/agent/witness/r1/w").unwrap();
    let (ck, _) = SigningKey::generate().unwrap();
    let (wk, _) = SigningKey::generate().unwrap();
    let trusted = |identity: AgentIdentity, key: &SigningKey| TrustedKey {
        identity,
        public_key: key.public_key(),
        not_before_ms: 0,
        not_after_ms: 1_000_000,
        revoked: false,
    };
    let ct = trusted(controller.clone(), &ck);
    let wt = trusted(witness.clone(), &wk);
    let addresses = ["192.0.2.10".parse().unwrap()];
    let probe = ProbeSpec {
        id: "lan".into(),
        primitive: Primitive::TcpConnect,
        device: s.request.device.clone(),
        source: ProbeSource {
            witness_id: witness.to_string(),
            location: "lan".into(),
            bind_address: addresses[0],
            interface: Some("lan0".into()),
        },
        endpoint: Some(Endpoint {
            address: "192.0.2.1".parse().unwrap(),
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
        claims: vec!["management".into()],
    };
    let binding = DeploymentBinding {
        deployment_id: s.request.deployment_id.clone(),
        checkpoint_digest: s.checkpoint_digest.clone(),
    };
    let assignment = WitnessAssignment {
        version: PROTOCOL_VERSION,
        assignment_id: "assignment".into(),
        issuer: controller.to_string(),
        recipient: witness.to_string(),
        issued_at_ms: 1_000,
        expires_at_ms: 900_000,
        deployment: Some(binding.clone()),
        plan_id: s.request.plan_id.clone(),
        plan_epoch: s.request.plan_epoch,
        graph_version: s.request.graph_version,
        revision: s.request.revision.clone(),
        device: s.request.device.clone(),
        probes: vec![probe.clone()],
    };
    let envelope = ck
        .sign(
            &serde_json::to_vec(&Statement {
                statement_type: STATEMENT_TYPE.into(),
                subject: revision_subject(&s.request.revision),
                predicate_type: WITNESS_ASSIGNMENT_TYPE.into(),
                predicate: assignment.clone(),
            })
            .unwrap(),
        )
        .unwrap();
    let scope = AssignmentScope {
        controller: &controller,
        witness: &witness,
        device: &s.request.device,
        revision: &s.request.revision,
        plan_id: &s.request.plan_id,
        plan_epoch: s.request.plan_epoch,
        graph_version: s.request.graph_version,
        location: "lan",
        bind_addresses: &addresses,
    };
    let va = verify_assignment(&envelope, &[ct], &scope, 5_000).unwrap();
    let seal = |id: &str, start, finish| {
        let e = WitnessEvidence {
            version: PROTOCOL_VERSION,
            evidence_id: id.into(),
            assignment_id: assignment.assignment_id.clone(),
            plan_id: assignment.plan_id.clone(),
            plan_epoch: assignment.plan_epoch,
            graph_version: assignment.graph_version,
            revision: assignment.revision.clone(),
            device: assignment.device.clone(),
            witness: witness.to_string(),
            deployment: Some(binding.clone()),
            probe_digest: probe_digest(&probe).unwrap(),
            completeness: Completeness::Complete,
            result: ProbeResult {
                probe_id: probe.id.clone(),
                outcome: Outcome::Success,
                started_at_ms: start,
                finished_at_ms: finish,
                detail: "connected".into(),
            },
        };
        let env = wk
            .sign(
                &serde_json::to_vec(&Statement {
                    statement_type: STATEMENT_TYPE.into(),
                    subject: revision_subject(&s.request.revision),
                    predicate_type: WITNESS_EVIDENCE_TYPE.into(),
                    predicate: e,
                })
                .unwrap(),
            )
            .unwrap();
        verify_evidence(&env, std::slice::from_ref(&wt), &va, 5_000).unwrap()
    };
    let first = seal("first", 2_000, 2_100);
    let second = seal("second", 4_000, 4_100);
    let required = [RequiredVerification {
        probe_id: probe.id.clone(),
        probe_digest: probe_digest(&probe).unwrap(),
        source: probe.source.clone(),
        lan_management_path: true,
    }];
    verify_confirmation(
        &[vec![&first], vec![&second]],
        &ConfirmationScope {
            deployment: &binding,
            device: &s.request.device,
            revision: &s.request.revision,
            plan_id: &s.request.plan_id,
            plan_epoch: s.request.plan_epoch,
            graph_version: s.request.graph_version,
            provisional_started_at_ms: s.provisional_started_at_ms.unwrap(),
            deadline_ms: s.expires_at_ms,
            required: &required,
        },
        5_000,
    )
    .unwrap()
}
fn persist_confirm_intent(j: &DeploymentJournal) {
    let s = j.status("a").unwrap();
    let t = token(&s);
    let p = confirmation_proof(&s, &t, 5_000).unwrap();
    j.exclusive(|c| begin_confirmation(c, &s, &p)).unwrap();
}

#[test]
fn successful_delivery_retries_do_not_repeat_effects() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    j.prepare(&req("a"), 10).unwrap();
    j.preflight("a", &mut b).unwrap();
    j.stage("a", &mut b).unwrap();
    j.durable_checkpoint("a", &mut b).unwrap();
    j.provisional_apply("a", 2_000, &mut b).unwrap();
    assert_eq!(b.calls, vec!["preflight", "stage", "checkpoint", "apply"]);
}
#[test]
fn one_unfinished_deployment_per_device_and_conflicting_id_rejected() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    j.prepare(&req("a"), 0).unwrap();
    let mut conflict = req("a");
    conflict.plan_digest = "other".into();
    assert!(matches!(
        j.prepare(&conflict, 0),
        Err(JournalError::ConflictingDeployment)
    ));
    let mut next = req("b");
    next.device.fencing_generation += 1;
    assert!(matches!(
        j.prepare(&next, 0),
        Err(JournalError::ActiveDeployment)
    ));
    j.rollback("a", &mut Fake::default()).unwrap();
    j.prepare(&next, 0).unwrap();
    assert!(matches!(
        j.preflight("a", &mut Fake::default()),
        Err(JournalError::StaleBinding)
    ));
}
#[test]
fn binding_monotonicity_requires_fence_for_profile_and_no_epoch_regression() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    j.prepare(&req("a"), 0).unwrap();
    j.rollback("a", &mut Fake::default()).unwrap();
    for change in 0..3 {
        let mut r = req("b");
        match change {
            0 => r.device.fencing_generation -= 1,
            1 => r.device.profile_digest = "other".into(),
            _ => {
                r.device.fencing_generation += 1;
                r.plan_epoch -= 1;
            }
        }
        assert!(matches!(j.prepare(&r, 0), Err(JournalError::StaleBinding)));
    }
}
#[test]
fn every_effect_and_transition_rejects_stale_fence_epoch_or_profile() {
    for binding in ["fence=fence+1", "epoch=epoch+1", "profile_digest='changed'"] {
        for step in 0..7 {
            let d = tempdir().unwrap();
            let j = DeploymentJournal::open(d.path().join("j")).unwrap();
            let mut b = Fake::default();
            j.prepare(&req("a"), 0).unwrap();
            if step >= 1 {
                j.preflight("a", &mut b).unwrap();
            }
            if step >= 2 {
                j.stage("a", &mut b).unwrap();
            }
            if step >= 3 {
                j.durable_checkpoint("a", &mut b).unwrap();
            }
            if step >= 4 {
                j.provisional_apply("a", 1_500, &mut b).unwrap();
            }
            let t = if step == 4 {
                Some(token(&j.status("a").unwrap()))
            } else {
                None
            };
            j.connection
                .lock()
                .unwrap()
                .execute(&format!("UPDATE device_fences SET {binding}"), [])
                .unwrap();
            b.calls.clear();
            let result = match step {
                0 => j.preflight("a", &mut b),
                1 => j.stage("a", &mut b),
                2 => j.durable_checkpoint("a", &mut b),
                3 => j.provisional_apply("a", 1_500, &mut b),
                4 => j
                    .record_independent_confirmation("a", t.as_ref().unwrap(), 5_000, &mut b)
                    .map(|_| ()),
                5 => j.rollback("a", &mut b),
                _ => j.recover(5_000, &mut b).map(|_| ()),
            };
            assert!(
                matches!(result, Err(JournalError::StaleBinding)),
                "{binding}, step {step}"
            );
            assert!(b.calls.is_empty());
        }
    }
}
#[test]
fn failures_before_apply_are_safe_to_retry() {
    for effect in ["preflight", "stage", "checkpoint"] {
        let d = tempdir().unwrap();
        let j = DeploymentJournal::open(d.path().join("j")).unwrap();
        let mut b = Fake {
            fail: Some(effect),
            ..Default::default()
        };
        j.prepare(&req("a"), 0).unwrap();
        let run = |j: &DeploymentJournal, b: &mut Fake| {
            j.preflight("a", b)?;
            j.stage("a", b)?;
            j.durable_checkpoint("a", b)
        };
        assert!(run(&j, &mut b).is_err());
        b.fail = None;
        run(&j, &mut b).unwrap();
        j.provisional_apply("a", 1_500, &mut b).unwrap();
        assert_eq!(b.calls.iter().filter(|c| **c == "apply").count(), 1);
    }
}
#[test]
fn uncertain_apply_is_never_retried_and_uses_durable_restore_receipt() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    let mut b = Fake::default();
    ready(&j, &mut b);
    b.fail = Some("apply");
    assert!(j.provisional_apply("a", 1_500, &mut b).is_err());
    assert!(j.provisional_apply("a", 1_500, &mut b).is_err());
    drop(j);
    let j = DeploymentJournal::open(path).unwrap();
    b.fail = None;
    j.recover(2_000, &mut b).unwrap();
    j.recover(2_000, &mut b).unwrap();
    assert_eq!(b.calls.iter().filter(|c| **c == "apply").count(), 1);
    assert_eq!(b.checkpoint_seen, Some("a".repeat(64)));
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::RolledBack);
}
#[test]
fn crash_at_every_preconfirm_boundary_is_recoverable_without_an_apply_retry() {
    for step in 0..6 {
        let d = tempdir().unwrap();
        let path = d.path().join("j");
        let j = DeploymentJournal::open(&path).unwrap();
        let mut b = Fake::default();
        j.prepare(&req("a"), 0).unwrap();
        if step >= 1 {
            j.preflight("a", &mut b).unwrap();
        }
        if step >= 2 {
            j.stage("a", &mut b).unwrap();
        }
        if step >= 3 {
            j.durable_checkpoint("a", &mut b).unwrap();
        }
        if step == 4 {
            let s = j.status("a").unwrap();
            j.exclusive(|c| begin_apply(c, &s, 1_500)).unwrap();
        }
        if step == 5 {
            j.provisional_apply("a", 1_500, &mut b).unwrap();
        }
        drop(j);
        let j = DeploymentJournal::open(path).unwrap();
        let applies = b.calls.iter().filter(|c| **c == "apply").count();
        j.recover(2_000, &mut b).unwrap();
        assert_eq!(applies, b.calls.iter().filter(|c| **c == "apply").count());
        assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::RolledBack);
        if step < 4 {
            assert!(b.calls.contains(&"discard"));
        }
    }
}
#[test]
fn apply_intent_deadline_and_checkpoint_are_visible_before_physical_apply() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    let mut b = Fake::default();
    ready(&j, &mut b);
    let s = j.status("a").unwrap();
    j.exclusive(|c| begin_apply(c, &s, 1_500)).unwrap();
    let c = Connection::open(path).unwrap();
    let s = read_status(&c, "a").unwrap();
    assert_eq!(s.phase, DeploymentPhase::ProvisionalIntent);
    assert_eq!(s.provisional_started_at_ms, Some(1_500));
    assert_eq!(s.expires_at_ms, 181_500);
    assert_eq!(s.checkpoint_digest, "a".repeat(64));
    assert!(s.intent_marker);
}
#[test]
fn backend_receives_the_exact_durable_apply_window() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    ready(&j, &mut b);
    j.provisional_apply("a", 1_500, &mut b).unwrap();
    assert_eq!(
        b.apply_window,
        Some(ApplyWindow {
            started_at_ms: 1_500,
            deadline_ms: 181_500
        })
    );
    let s = j.status("a").unwrap();
    assert_eq!(Some(1_500), s.provisional_started_at_ms);
    assert_eq!(181_500, s.expires_at_ms);
}
#[test]
fn preapply_discard_failure_stays_unfinished_and_retries_after_reboot() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    let mut b = Fake {
        fail: Some("discard"),
        ..Default::default()
    };
    j.prepare(&req("a"), 0).unwrap();
    j.preflight("a", &mut b).unwrap();
    j.stage("a", &mut b).unwrap();
    assert!(j.rollback("a", &mut b).is_err());
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::RollingBack);
    drop(j);
    let j = DeploymentJournal::open(path).unwrap();
    b.fail = None;
    j.recover(2_000, &mut b).unwrap();
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::RolledBack);
    assert_eq!(b.calls.iter().filter(|c| **c == "discard").count(), 2);
    assert!(!b.calls.contains(&"rollback"));
}
#[test]
fn confirm_requires_live_token_and_backend_success() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    let t = token(&j.status("a").unwrap());
    assert!(
        j.record_independent_confirmation("a", &t, t.expires_at_ms(), &mut b)
            .is_err()
    );
    assert!(!b.calls.contains(&"confirm"));
    assert_eq!(
        j.record_independent_confirmation("a", &t, 5_000, &mut b)
            .unwrap(),
        DeploymentPhase::Confirmed
    );
    assert_eq!(b.confirm_expires_at_ms, Some(t.expires_at_ms()));
    j.record_independent_confirmation("a", &t, 5_000, &mut b)
        .unwrap();
    assert_eq!(b.calls.iter().filter(|c| **c == "confirm").count(), 1);
    assert_eq!(j.status("a").unwrap().verification_rounds, 2);
    assert!(j.rollback("a", &mut b).is_err());
}
#[test]
fn lock_wait_expires_confirmation_before_any_backend_effect() {
    use std::{sync::mpsc, thread};
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    let t = token(&j.status("a").unwrap());
    let (waiter, locked) = mpsc::channel();
    thread::scope(|scope| {
        scope.spawn(|| {
            j.exclusive(|_| {
                waiter.send(()).unwrap();
                thread::sleep(Duration::from_millis(25));
                Ok(())
            })
            .unwrap()
        });
        locked.recv().unwrap();
        assert!(matches!(
            j.record_independent_confirmation("a", &t, t.expires_at_ms().saturating_sub(1), &mut b),
            Err(JournalError::Invalid(_))
        ));
    });
    assert_eq!(
        j.status("a").unwrap().phase,
        DeploymentPhase::AwaitingVerification
    );
    assert!(!b.calls.contains(&"confirm"));
}
#[test]
fn mismatched_apply_window_or_checkpoint_cannot_confirm() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    for change in 0..3 {
        let mut s = j.status("a").unwrap();
        match change {
            0 => s.expires_at_ms += 1,
            1 => s.provisional_started_at_ms = Some(1_499),
            _ => s.checkpoint_digest = "b".repeat(64),
        }
        let t = token(&s);
        assert!(
            j.record_independent_confirmation("a", &t, 5_000, &mut b)
                .is_err()
        );
    }
    assert!(!b.calls.contains(&"confirm"));
}
#[test]
fn confirm_error_keeps_intent_and_unknown_outcome_blocks_both_retry_and_rollback() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    let t = token(&j.status("a").unwrap());
    b.fail = Some("confirm");
    assert!(
        j.record_independent_confirmation("a", &t, 5_000, &mut b)
            .is_err()
    );
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::ConfirmIntent);
    b.fail = None;
    assert!(matches!(
        j.record_independent_confirmation("a", &t, 5_000, &mut b),
        Err(JournalError::UncertainConfirmation)
    ));
    assert!(matches!(
        j.rollback("a", &mut b),
        Err(JournalError::UncertainConfirmation)
    ));
    assert_eq!(b.calls.iter().filter(|c| **c == "confirm").count(), 1);
    assert!(!b.calls.contains(&"rollback"));
}
#[test]
fn expired_retry_only_reconciles_a_durable_confirmation_intent() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    let t = token(&j.status("a").unwrap());
    b.fail = Some("confirm");
    assert!(
        j.record_independent_confirmation("a", &t, 5_000, &mut b)
            .is_err()
    );
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::ConfirmIntent);
    b.fail = None;
    b.disposition = Some(BackendDisposition::Confirmed {
        checkpoint_digest: "a".repeat(64),
    });
    assert_eq!(
        j.record_independent_confirmation("a", &t, t.expires_at_ms(), &mut b)
            .unwrap(),
        DeploymentPhase::Confirmed
    );
    assert_eq!(b.calls.iter().filter(|c| **c == "confirm").count(), 1);
}
#[test]
fn crash_before_confirm_rolls_back_only_after_authoritative_provisional_observation() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    persist_confirm_intent(&j);
    drop(j);
    let j = DeploymentJournal::open(path).unwrap();
    b.disposition = Some(BackendDisposition::Provisional);
    j.recover(999_999, &mut b).unwrap();
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::RolledBack);
    assert!(!b.calls.contains(&"confirm"));
}
#[test]
fn crash_after_backend_confirm_reconciles_success_even_after_token_expiry() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    persist_confirm_intent(&j);
    drop(j);
    let j = DeploymentJournal::open(path).unwrap();
    b.disposition = Some(BackendDisposition::Confirmed {
        checkpoint_digest: "a".repeat(64),
    });
    j.recover(999_999, &mut b).unwrap();
    j.recover(999_999, &mut b).unwrap();
    assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::Confirmed);
    assert_eq!(j.status("a").unwrap().verification_rounds, 2);
    assert!(!b.calls.contains(&"confirm"));
    assert!(!b.calls.contains(&"rollback"));
}
#[test]
fn backend_confirmed_without_sealed_proof_or_with_wrong_checkpoint_is_rejected() {
    for case in 0..3 {
        let d = tempdir().unwrap();
        let j = DeploymentJournal::open(d.path().join("j")).unwrap();
        let mut b = Fake::default();
        applied(&j, &mut b);
        if case > 0 {
            persist_confirm_intent(&j);
        }
        if case == 1 {
            j.connection
                .lock()
                .unwrap()
                .execute("UPDATE deployments SET confirmation_proof=NULL", [])
                .unwrap();
        }
        b.disposition = Some(BackendDisposition::Confirmed {
            checkpoint_digest: if case == 2 {
                "b".repeat(64)
            } else {
                "a".repeat(64)
            },
        });
        assert!(j.recover(6_000, &mut b).is_err());
        assert_ne!(j.status("a").unwrap().phase, DeploymentPhase::Confirmed);
        assert!(!b.calls.contains(&"rollback"));
    }
}
#[test]
fn reconcile_failure_and_rollback_failure_are_restart_retryable() {
    for effect in ["reconcile", "rollback"] {
        let d = tempdir().unwrap();
        let path = d.path().join("j");
        let j = DeploymentJournal::open(&path).unwrap();
        let mut b = Fake::default();
        applied(&j, &mut b);
        b.fail = Some(effect);
        assert!(j.recover(6_000, &mut b).is_err());
        drop(j);
        let j = DeploymentJournal::open(path).unwrap();
        b.fail = None;
        j.recover(6_000, &mut b).unwrap();
        assert_eq!(j.status("a").unwrap().phase, DeploymentPhase::RolledBack);
        assert_eq!(b.calls.iter().filter(|c| **c == "apply").count(), 1);
    }
}
#[test]
fn already_rolled_back_does_not_repeat_backend_rollback() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    b.disposition = Some(BackendDisposition::AlreadyRolledBack);
    j.recover(6_000, &mut b).unwrap();
    assert!(!b.calls.contains(&"rollback"));
    j.rollback("a", &mut b).unwrap();
}
#[test]
fn concurrent_prepare_and_migration_allow_one_active_device_deployment() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let barrier = Arc::new(Barrier::new(4));
    let successes = Arc::new(AtomicUsize::new(0));
    std::thread::scope(|scope| {
        for n in 0..4 {
            let barrier = barrier.clone();
            let successes = successes.clone();
            let path = path.clone();
            scope.spawn(move || {
                barrier.wait();
                let j = DeploymentJournal::open(path).unwrap();
                if j.prepare(&req(&format!("a{n}")), 0).is_ok() {
                    successes.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
    });
    assert_eq!(successes.load(Ordering::SeqCst), 1);
}
#[test]
fn concurrent_duplicate_apply_handles_issue_one_physical_effect() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    ready(&j, &mut Fake::default());
    let barrier = Arc::new(Barrier::new(2));
    let calls = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let j = DeploymentJournal::open(path).unwrap();
                    let mut b = Fake::default();
                    barrier.wait();
                    j.provisional_apply("a", 1_500, &mut b).unwrap();
                    b.calls.iter().filter(|c| **c == "apply").count()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum::<usize>()
    });
    assert_eq!(calls, 1);
}
#[test]
fn concurrent_confirm_and_rollback_cannot_overwrite_each_other() {
    let d = tempdir().unwrap();
    let path = d.path().join("j");
    let j = DeploymentJournal::open(&path).unwrap();
    applied(&j, &mut Fake::default());
    let t = token(&j.status("a").unwrap());
    let a = DeploymentJournal::open(&path).unwrap();
    let b = DeploymentJournal::open(&path).unwrap();
    let barrier = Barrier::new(2);
    let (confirm, rollback) = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            let mut backend = Fake::default();
            barrier.wait();
            let result = a.record_independent_confirmation("a", &t, 5_000, &mut backend);
            (result, backend.calls)
        });
        let second = scope.spawn(|| {
            let mut backend = Fake::default();
            barrier.wait();
            let result = b.rollback("a", &mut backend);
            (result, backend.calls)
        });
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_ne!(confirm.0.is_ok(), rollback.0.is_ok());
    assert_ne!(
        confirm.1.contains(&"confirm"),
        rollback.1.contains(&"rollback")
    );
    assert!(matches!(
        j.status("a").unwrap().phase,
        DeploymentPhase::Confirmed | DeploymentPhase::RolledBack
    ));
}
#[test]
fn confirmation_audit_envelopes_survive_restart_and_tampering_is_rejected() {
    let d = tempdir().unwrap();
    let j = DeploymentJournal::open(d.path().join("j")).unwrap();
    let mut b = Fake::default();
    applied(&j, &mut b);
    persist_confirm_intent(&j);
    let c = j.connection.lock().unwrap();
    let raw: String = c
        .query_row("SELECT confirmation_proof FROM deployments", [], |r| {
            r.get(0)
        })
        .unwrap();
    let mut p: ConfirmationProof = serde_json::from_str(&raw).unwrap();
    assert!(p.signed_proofs_json.contains("signatures"));
    p.signed_proofs_json = "[]".into();
    c.execute(
        "UPDATE deployments SET confirmation_proof=?1",
        [serde_json::to_string(&p).unwrap()],
    )
    .unwrap();
    drop(c);
    assert!(j.recover(6_000, &mut b).is_err());
}
