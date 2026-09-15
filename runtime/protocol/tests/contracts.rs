use intent_protocol::{
    assurance::{
        AssurancePlan, Claim, DeviceProfile, Endpoint, Expectation, IpFamily, Primitive,
        ProbeRequirement, ProbeSource, ProbeSpec, ResourceLimits,
    },
    check::{check_plan, validate_probe, CoverageContext},
    state::{CollectionScope, Completeness, Datastore, ObservationMetadata},
    DeviceBinding, RevisionRef, SourceSpan, PROTOCOL_VERSION,
};
use std::net::{IpAddr, Ipv4Addr};

const NOW: u64 = 500;

fn binding() -> DeviceBinding {
    DeviceBinding {
        device_id: "router-a".into(),
        profile_digest: "profile-a".into(),
        fencing_generation: 3,
    }
}

fn revision() -> RevisionRef {
    RevisionRef {
        id: "rev-a".into(),
        source_digest: "source-a".into(),
    }
}

fn source() -> ProbeSource {
    ProbeSource {
        witness_id: "witness-a".into(),
        location: "lab".into(),
        bind_address: IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)),
        interface: Some("lan0".into()),
    }
}

fn endpoint() -> Endpoint {
    Endpoint {
        address: IpAddr::V4(Ipv4Addr::new(192, 0, 2, 20)),
        port: Some(443),
        family: IpFamily::V4,
    }
}

fn expectation() -> Expectation {
    Expectation::TcpConnected
}

fn probe() -> ProbeSpec {
    ProbeSpec {
        id: "probe-a".into(),
        primitive: Primitive::TcpConnect,
        device: binding(),
        source: source(),
        endpoint: Some(endpoint()),
        expectation: expectation(),
        limits: ResourceLimits {
            timeout_ms: 1_000,
            max_response_bytes: 1_024,
            max_attempts: 1,
        },
        interval_ms: 5_000,
        depends_on: vec![],
        claims: vec!["claim-a".into()],
    }
}

fn span() -> SourceSpan {
    SourceSpan {
        file: "intent.ni".into(),
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 12,
    }
}

fn requirement() -> ProbeRequirement {
    ProbeRequirement {
        primitive: Primitive::TcpConnect,
        source: source(),
        endpoint: Some(endpoint()),
        expectation: expectation(),
    }
}

fn claim() -> Claim {
    Claim {
        id: "claim-a".into(),
        device_id: "router-a".into(),
        kind: "reachability".into(),
        source: span(),
        requirements: vec![requirement()],
        unsupported_reason: None,
    }
}

fn profile() -> DeviceProfile {
    DeviceProfile {
        version: PROTOCOL_VERSION,
        device: binding(),
        firmware: "openwrt-test".into(),
        observed_at_ms: 100,
        expires_at_ms: 1_000,
        supported_primitives: vec![Primitive::TcpConnect],
        hardware_validated: true,
    }
}

fn plan() -> AssurancePlan {
    AssurancePlan {
        version: PROTOCOL_VERSION,
        id: "plan-a".into(),
        epoch: 7,
        revision: revision(),
        graph_version: 9,
        profiles: vec![profile()],
        claims: vec![claim()],
        probes: vec![probe()],
    }
}

fn check(plan: &AssurancePlan) -> Result<(), intent_protocol::check::CheckError> {
    let rev = revision();
    let profiles = vec![profile()];
    let claims = vec![claim()];
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    check_plan(plan, &context).map(|_| ())
}

#[test]
fn accepts_a_complete_plan_against_independent_authoritative_context() {
    let p = plan();
    let rev = revision();
    let profiles = vec![profile()];
    let claims = vec![claim()];
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert!(check_plan(&p, &context).is_ok());
}

#[test]
fn rejects_tampered_revision_and_profile_binding() {
    let mut p = plan();
    p.revision.id = "attacker-revision".into();
    assert_eq!(check(&p).unwrap_err().code, "plan.revision");

    let mut p = plan();
    p.profiles[0].device.profile_digest = "attacker-profile".into();
    assert_eq!(check(&p).unwrap_err().code, "plan.profiles");
}

#[test]
fn rejects_omitted_or_modified_authoritative_claims() {
    let mut p = plan();
    p.claims.clear();
    assert_eq!(check(&p).unwrap_err().code, "plan.claims");

    let mut p = plan();
    p.claims[0].kind = "weaker-claim".into();
    assert_eq!(check(&p).unwrap_err().code, "plan.claims");
}

#[test]
fn validate_probe_rejects_missing_endpoint_source_and_family_mismatch() {
    let mut p = probe();
    p.endpoint = None;
    assert_eq!(validate_probe(&p).unwrap_err().code, "probe.binding");

    let mut p = probe();
    p.source.witness_id.clear();
    assert_eq!(validate_probe(&p).unwrap_err().code, "probe.binding");

    let mut p = probe();
    p.endpoint.as_mut().unwrap().family = IpFamily::V6;
    assert_eq!(validate_probe(&p).unwrap_err().code, "probe.binding");
}

#[test]
fn validate_probe_rejects_mismatched_expectation() {
    let mut p = probe();
    p.expectation = Expectation::IcmpEcho;
    assert_eq!(validate_probe(&p).unwrap_err().code, "probe.binding");
}

#[test]
fn rejects_missing_coverage_even_when_the_plan_claim_is_authoritative() {
    let mut second = requirement();
    second.endpoint = Some(Endpoint {
        address: IpAddr::V4(Ipv4Addr::new(192, 0, 2, 21)),
        port: Some(443),
        family: IpFamily::V4,
    });
    let authoritative_claim = Claim {
        requirements: vec![requirement(), second],
        ..claim()
    };
    let mut p = plan();
    p.claims = vec![authoritative_claim.clone()];
    let revision = revision();
    let profiles = vec![profile()];
    let claims = vec![authoritative_claim];
    let context = CoverageContext {
        revision: &revision,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.uncovered"
    );
}

#[test]
fn rejects_unsupported_claims_and_primitives() {
    let mut p = plan();
    p.claims[0].unsupported_reason = Some("compiler lacks witness".into());
    let rev = revision();
    let profiles = vec![profile()];
    let claims = p.claims.clone();
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.unsupported"
    );

    let mut p = plan();
    p.profiles[0].supported_primitives.clear();
    let rev = revision();
    let profiles = p.profiles.clone();
    let claims = vec![claim()];
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.primitive-unsupported"
    );
}

#[test]
fn rejects_duplicate_ids_dangling_dependencies_and_cycles() {
    let mut p = plan();
    p.probes.push(probe());
    assert_eq!(check(&p).unwrap_err().code, "plan.probe-duplicate");

    let mut p = plan();
    p.probes[0].depends_on = vec!["missing".into()];
    assert_eq!(check(&p).unwrap_err().code, "plan.dependency");

    let mut p = plan();
    p.probes[0].depends_on = vec!["probe-a".into()];
    assert_eq!(check(&p).unwrap_err().code, "plan.cycle");

    let mut p = plan();
    p.claims.push(claim());
    let rev = revision();
    let profiles = vec![profile()];
    let claims = p.claims.clone();
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.claim-invalid"
    );

    let mut p = plan();
    p.profiles.push(profile());
    let rev = revision();
    let profiles = p.profiles.clone();
    let claims = vec![claim()];
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.profile-invalid"
    );
}

#[test]
fn rejects_stale_and_future_profiles() {
    let mut p = plan();
    p.profiles[0].expires_at_ms = NOW;
    let rev = revision();
    let profiles = p.profiles.clone();
    let claims = vec![claim()];
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.profile-invalid"
    );

    let mut p = plan();
    p.profiles[0].observed_at_ms = NOW + 1;
    let rev = revision();
    let profiles = p.profiles.clone();
    let claims = vec![claim()];
    let context = CoverageContext {
        revision: &rev,
        epoch: 7,
        graph_version: 9,
        profiles: &profiles,
        claims: &claims,
        now_ms: NOW,
    };
    assert_eq!(
        check_plan(&p, &context).err().unwrap().code,
        "plan.profile-invalid"
    );
}

#[test]
fn freshness_rejects_stale_incomplete_and_future_observations() {
    let scope = |completeness| CollectionScope {
        objects: vec!["network.interface".into()],
        datastore: Datastore::Operational,
        session_id: None,
        completeness,
    };
    let metadata = |collected_at_ms, received_at_ms, completeness| ObservationMetadata {
        version: PROTOCOL_VERSION,
        id: "obs".into(),
        device: binding(),
        collector_identity: "witness".into(),
        collected_at_ms,
        received_at_ms,
        interval_ms: 100,
        invocation_timeout_ms: 10,
        scope: scope(completeness),
        origin: "ubus".into(),
    };
    assert!(metadata(450, 460, Completeness::Complete).fresh_at(NOW));
    assert!(!metadata(150, 160, Completeness::Complete).fresh_at(NOW));
    assert!(!metadata(
        450,
        460,
        Completeness::Partial {
            missing: vec!["wireless".into()]
        }
    )
    .fresh_at(NOW));
    assert!(!metadata(501, 501, Completeness::Complete).fresh_at(NOW));
}
