use intent_authorization::{
    Action, AuthorizationEngine, ControllerGrant, ControllerScope, Decision, TrustedBinding,
};
use intent_identity::AgentIdentity;
use intent_protocol::{AgentRole, DeviceBinding};

fn id(role: &str, domain: &str, device: &str) -> AgentIdentity {
    AgentIdentity::parse(&format!("spiffe://{domain}/agent/{role}/{device}/one")).unwrap()
}
fn binding(device: &str, profile: &str, fence: u64) -> DeviceBinding {
    DeviceBinding {
        device_id: device.into(),
        profile_digest: profile.into(),
        fencing_generation: fence,
    }
}
fn engine() -> (
    AuthorizationEngine,
    DeviceBinding,
    AgentIdentity,
    AgentIdentity,
    AgentIdentity,
) {
    let b = binding("router-a", "profile-a", 7);
    let controller = id("controller", "lab.local", "router-a");
    let witness = id("witness", "lab.local", "router-a");
    let apply = id("apply", "lab.local", "router-a");
    let grants = [&controller, &witness, &apply]
        .into_iter()
        .map(|identity| ControllerGrant {
            identity: identity.clone(),
            device_id: "router-a".into(),
            profile_digest: "profile-a".into(),
            fencing_generation: 7,
        })
        .collect();
    let e = AuthorizationEngine::new(
        ControllerScope::new(grants, vec![TrustedBinding(b.clone())]).unwrap(),
    )
    .unwrap();
    (e, b, controller, witness, apply)
}

#[test]
fn role_action_matrix_and_explicit_forbids() {
    let (e, b, controller, witness, apply) = engine();
    for a in [
        Action::AssignWitness,
        Action::AssignApply,
        Action::EnrollAgent,
        Action::RevokeAgent,
    ] {
        assert_eq!(e.authorize(&controller, a, &b).unwrap(), Decision::Allow);
    }
    assert_eq!(
        e.authorize(&witness, Action::SubmitEvidence, &b).unwrap(),
        Decision::Allow
    );
    assert_eq!(
        e.authorize(&witness, Action::ReadState, &b).unwrap(),
        Decision::Allow
    );
    assert_eq!(
        e.authorize(&apply, Action::SubmitApplyReceipt, &b).unwrap(),
        Decision::Allow
    );
    assert_eq!(
        e.authorize(&apply, Action::ReadState, &b).unwrap(),
        Decision::Allow
    );
    for (who, a) in [
        (&witness, Action::SubmitApplyReceipt),
        (&witness, Action::AssignApply),
        (&witness, Action::EnrollAgent),
        (&apply, Action::SubmitEvidence),
        (&apply, Action::AssignApply),
    ] {
        assert_eq!(e.authorize(who, a, &b).unwrap(), Decision::Deny);
    }
}

#[test]
fn trust_domain_device_grant_and_binding_fences_are_required() {
    let (e, b, _controller, witness, _apply) = engine();
    let foreign = id("witness", "evil.local", "router-a");
    assert_eq!(
        e.authorize(&foreign, Action::SubmitEvidence, &b).unwrap(),
        Decision::Deny
    );
    let other = id("witness", "lab.local", "router-b");
    assert_eq!(
        e.authorize(&other, Action::SubmitEvidence, &b).unwrap(),
        Decision::Deny
    );
    assert_eq!(
        e.authorize(
            &witness,
            Action::SubmitEvidence,
            &binding("router-a", "old", 7)
        )
        .unwrap(),
        Decision::Deny
    );
    assert_eq!(
        e.authorize(
            &witness,
            Action::SubmitEvidence,
            &binding("router-a", "profile-a", 6)
        )
        .unwrap(),
        Decision::Deny
    );
}

#[test]
fn role_attribute_cannot_be_spoofed_by_identity_text() {
    let (e, b, _controller, witness, _apply) = engine();
    assert_eq!(witness.role(), AgentRole::Witness);
    assert_eq!(
        e.authorize(&witness, Action::AssignApply, &b).unwrap(),
        Decision::Deny
    );
}

#[test]
fn duplicate_trusted_binding_is_rejected() {
    let b = binding("router-a", "profile-a", 7);
    let result = ControllerScope::new(
        Vec::new(),
        vec![TrustedBinding(b.clone()), TrustedBinding(b)],
    );
    assert!(result.is_err());
}
