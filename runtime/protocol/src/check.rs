//! Independent structural and concrete coverage checking, not compiler validation
//! or cryptographic authorization. The controller obtains context from its own
//! admitted revision/profile records, never from the submitted plan.
use crate::assurance::*;
use crate::{RevisionRef, PROTOCOL_VERSION};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{code}: {subject}: {explanation}")]
pub struct CheckError {
    pub code: &'static str,
    pub subject: String,
    pub explanation: String,
}

fn error(code: &'static str, subject: &str, explanation: &str) -> CheckError {
    CheckError {
        code,
        subject: subject.into(),
        explanation: explanation.into(),
    }
}

pub struct CoverageContext<'a> {
    pub revision: &'a RevisionRef,
    pub epoch: u64,
    pub graph_version: u64,
    pub profiles: &'a [DeviceProfile],
    pub claims: &'a [Claim],
    pub now_ms: u64,
}

/// Borrow the checked plan so it cannot be mutated behind a cached verdict.
/// This token conveys coverage checking only, not deployment authorization.
pub struct CheckedPlan<'a>(&'a AssurancePlan);
impl<'a> CheckedPlan<'a> {
    pub fn plan(&self) -> &'a AssurancePlan {
        self.0
    }
}

pub fn validate_probe(p: &ProbeSpec) -> Result<(), CheckError> {
    let bad = |message| error("probe.binding", &p.id, message);
    if p.id.is_empty()
        || p.device.device_id.is_empty()
        || p.device.profile_digest.is_empty()
        || p.source.witness_id.is_empty()
        || p.source.location.is_empty()
    {
        return Err(bad(
            "A device/profile, witness identity and source location are required",
        ));
    }
    if p.source.bind_address.is_unspecified()
        || p.source.bind_address.is_multicast()
        || p.source
            .interface
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 64 || s.chars().any(char::is_control))
    {
        return Err(bad(
            "Use a concrete source address and valid interface binding",
        ));
    }
    if p.limits.timeout_ms == 0
        || p.limits.timeout_ms > 60_000
        || p.limits.max_response_bytes == 0
        || p.limits.max_response_bytes > 1_048_576
        || p.limits.max_attempts == 0
        || p.limits.max_attempts > 3
        || p.interval_ms < p.limits.timeout_ms
        || p.interval_ms > 86_400_000
    {
        return Err(error(
            "probe.limits",
            &p.id,
            "Set bounded timeout, response bytes, attempts and interval",
        ));
    }
    let local = matches!(p.primitive, Primitive::UciReadback | Primitive::NetifdState);
    match &p.endpoint {
        None if !local => return Err(bad("Network probes require a concrete IP endpoint")),
        Some(_) if local => {
            return Err(bad(
                "Device-local reads must not specify a network endpoint",
            ))
        }
        Some(e) => {
            if e.address.is_unspecified()
                || e.address.is_multicast()
                || (e.address.is_ipv4() != (e.family == IpFamily::V4))
                || e.address.is_ipv4() != p.source.bind_address.is_ipv4()
            {
                return Err(bad(
                    "Source address, endpoint and explicit IP family must agree",
                ));
            }
            if p.primitive == Primitive::Icmp {
                if e.port.is_some() {
                    return Err(bad("ICMP has no transport port"));
                }
            } else if e.port.is_none_or(|port| port == 0) {
                return Err(bad("A nonzero transport port is required"));
            }
        }
        None => {}
    }
    match (&p.primitive, &p.expectation) {
        (Primitive::TcpConnect, Expectation::TcpConnected)
        | (Primitive::Icmp, Expectation::IcmpEcho) => {}
        (
            Primitive::UdpDns | Primitive::TcpDns,
            Expectation::DnsAnswer {
                name,
                record_type,
                answers,
            },
        ) => {
            if name.is_empty()
                || name.len() > 253
                || name.chars().any(char::is_control)
                || !matches!(record_type.as_str(), "A" | "AAAA")
                || answers.is_empty()
                || answers.iter().any(|a| {
                    a.parse::<std::net::IpAddr>()
                        .map_or(true, |ip| ip.is_ipv4() != (record_type == "A"))
                })
            {
                return Err(bad(
                    "DNS requires a name, supported A/AAAA type and concrete expected answers",
                ));
            }
        }
        (
            Primitive::Http,
            Expectation::HttpStatus {
                scheme,
                host,
                path,
                status,
            },
        ) => {
            if !matches!(scheme.as_str(), "http" | "https")
                || host.is_empty()
                || host
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control() || "/@:#?\\".contains(c))
                || !path.starts_with('/')
                || path.starts_with("//")
                || path.chars().any(char::is_control)
                || !(100..=599).contains(status)
            {
                return Err(bad(
                    "HTTP requires explicit scheme, host, absolute path and valid expected status",
                ));
            }
        }
        (
            Primitive::UciReadback,
            Expectation::UciValue {
                package,
                section,
                option,
                form,
                values,
            },
        ) => {
            let identifier = |s: &str| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            };
            if !matches!(
                package.as_str(),
                "network" | "dhcp" | "firewall" | "wireless"
            ) || !identifier(section)
                || !identifier(option)
                || match form {
                    UciFieldForm::Scalar => values.len() != 1,
                    UciFieldForm::OrderedList => values.is_empty(),
                    UciFieldForm::Absent => !values.is_empty(),
                }
                || values.iter().any(|v| v.len() > 4096 || v.chars().any(char::is_control))
                || crate::state::sensitive_field(option)
            {
                return Err(bad(
                    "Readback needs an allowed package and concrete non-secret field expectation",
                ));
            }
        }
        (Primitive::UciReadback, Expectation::UciSection { package, section, section_type }) => {
            if !matches!(package.as_str(), "network" | "dhcp" | "firewall" | "wireless")
                || [section, section_type].iter().any(|s| s.is_empty() || s.len() > 128 || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
                return Err(bad("Readback needs an allowed package and concrete section type"));
            }
        }
        (
            Primitive::NetifdState,
            Expectation::NetifdInterface {
                interface,
                addresses,
                ..
            },
        ) => {
            if interface.is_empty()
                || interface.len() > 64
                || interface.chars().any(char::is_control)
                || addresses.iter().any(|a| parse_interface_address(a).is_none())
            {
                return Err(bad(
                    "netifd requires an explicit interface and valid expected addresses",
                ));
            }
        }
        _ => return Err(bad("Primitive and expectation do not agree")),
    }
    Ok(())
}

pub fn check_plan<'a>(
    plan: &'a AssurancePlan,
    ctx: &CoverageContext<'_>,
) -> Result<CheckedPlan<'a>, CheckError> {
    if plan.version != PROTOCOL_VERSION
        || plan.id.is_empty()
        || plan.epoch != ctx.epoch
        || plan.graph_version != ctx.graph_version
        || &plan.revision != ctx.revision
    {
        return Err(error(
            "plan.revision",
            &plan.id,
            "Plan version, revision, epoch or graph version is not current",
        ));
    }
    if plan.claims != ctx.claims || plan.claims.is_empty() {
        return Err(error(
            "plan.claims",
            &plan.id,
            "Plan must contain every authoritative compiler claim exactly",
        ));
    }
    if plan.profiles != ctx.profiles {
        return Err(error(
            "plan.profiles",
            &plan.id,
            "Plan profile set differs from current device capabilities",
        ));
    }
    let mut profiles = HashMap::new();
    for profile in &plan.profiles {
        if profile.version != PROTOCOL_VERSION
            || profile.device.device_id.is_empty()
            || profile.device.profile_digest.is_empty()
            || profile.observed_at_ms > ctx.now_ms
            || profile.expires_at_ms <= ctx.now_ms
            || profile.observed_at_ms >= profile.expires_at_ms
            || profiles
                .insert(profile.device.device_id.as_str(), profile)
                .is_some()
        {
            return Err(error(
                "plan.profile-invalid",
                &profile.device.device_id,
                "Duplicate, missing, stale or future-dated profile",
            ));
        }
    }
    let mut claims = HashMap::new();
    for claim in &plan.claims {
        if let Some(reason) = &claim.unsupported_reason {
            return Err(error("plan.unsupported", &claim.id, reason));
        }
        if claim.id.is_empty()
            || claim.kind.is_empty()
            || claim.requirements.is_empty()
            || claims.insert(claim.id.as_str(), claim).is_some()
            || !profiles.contains_key(claim.device_id.as_str())
        {
            return Err(error(
                "plan.claim-invalid",
                &claim.id,
                "Claim lacks concrete requirements/current profile or has a duplicate ID",
            ));
        }
    }
    let mut probes = HashMap::new();
    for probe in &plan.probes {
        validate_probe(probe)?;
        if probes.insert(probe.id.as_str(), probe).is_some() {
            return Err(error(
                "plan.probe-duplicate",
                &probe.id,
                "Probe IDs must be unique",
            ));
        }
        let profile = profiles
            .get(probe.device.device_id.as_str())
            .ok_or_else(|| {
                error(
                    "plan.profile-missing",
                    &probe.id,
                    "No current profile for probe",
                )
            })?;
        if profile.device != probe.device
            || !profile.supported_primitives.contains(&probe.primitive)
        {
            return Err(error(
                "plan.primitive-unsupported",
                &probe.id,
                "Profile does not support this primitive or binding is stale",
            ));
        }
        if probe.claims.is_empty()
            || probe.claims.iter().collect::<HashSet<_>>().len() != probe.claims.len()
        {
            return Err(error(
                "plan.probe-claims",
                &probe.id,
                "Probe needs unique claim bindings",
            ));
        }
        for id in &probe.claims {
            let claim = claims.get(id.as_str()).ok_or_else(|| {
                error("plan.claim-missing", id, "Probe references an absent claim")
            })?;
            if claim.device_id != probe.device.device_id
                || !claim
                    .requirements
                    .iter()
                    .any(|r| matches_requirement(probe, r))
            {
                return Err(error(
                    "plan.expectation",
                    id,
                    "Probe does not match the claim's concrete endpoint, source and expectation",
                ));
            }
        }
    }
    for claim in &plan.claims {
        for requirement in &claim.requirements {
            if !plan.probes.iter().any(|p| {
                p.claims.contains(&claim.id)
                    && p.device.device_id == claim.device_id
                    && matches_requirement(p, requirement)
            }) {
                return Err(error("plan.uncovered", &claim.id, "Add the missing concrete probe with the required source, endpoint and expectation"));
            }
        }
    }
    // Kahn's algorithm avoids unbounded recursion on an untrusted dependency graph.
    let mut remaining = HashMap::new();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for probe in &plan.probes {
        if probe.depends_on.iter().collect::<HashSet<_>>().len() != probe.depends_on.len() {
            return Err(error("plan.dependency", &probe.id, "Duplicate dependency"));
        }
        remaining.insert(probe.id.as_str(), probe.depends_on.len());
        for dep in &probe.depends_on {
            if !probes.contains_key(dep.as_str()) {
                return Err(error("plan.dependency", &probe.id, "Missing dependency"));
            }
            children.entry(dep).or_default().push(&probe.id);
        }
    }
    let mut ready: Vec<&str> = remaining
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop() {
        visited += 1;
        for child in children.get(id).into_iter().flatten() {
            let n = remaining.get_mut(child).expect("checked dependency graph");
            *n -= 1;
            if *n == 0 {
                ready.push(child);
            }
        }
    }
    if visited != probes.len() {
        return Err(error(
            "plan.cycle",
            &plan.id,
            "Assurance dependencies contain a cycle",
        ));
    }
    Ok(CheckedPlan(plan))
}

fn matches_requirement(probe: &ProbeSpec, r: &ProbeRequirement) -> bool {
    probe.primitive == r.primitive
        && probe.source == r.source
        && probe.endpoint == r.endpoint
        && probe.expectation == r.expectation
}
