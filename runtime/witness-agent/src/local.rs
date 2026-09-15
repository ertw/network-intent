//! Device-local observations use the installed identity/profile and read-only
//! rpcd session, never credentials or a device identity from the probe payload.
use crate::adapters::{ObservationError, ReadOnlyUbus, UbusTransport};
use intent_protocol::{assurance::*, check::validate_probe, state::*, DeviceBinding};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct LocalObservationContext {
    pub device: DeviceBinding,
    pub source: ProbeSource,
    pub rpcd_session: String,
}

/// Synchronous native calls must run on the witness's bounded blocking worker
/// pool. The native transport receives the remaining total invocation budget.
pub fn execute<T: UbusTransport>(
    spec: &ProbeSpec,
    local: &LocalObservationContext,
    transport: T,
) -> ProbeResult {
    let started_at_ms = crate::now_ms();
    let start = Instant::now();
    let result = if let Err(error) = validate_probe(spec) {
        (Outcome::Unsupported, error.to_string())
    } else if spec.device != local.device || spec.source != local.source {
        (
            Outcome::Unavailable,
            "probe does not match installed local device and witness source".into(),
        )
    } else {
        let budget = Duration::from_millis(spec.limits.timeout_ms);
        match ReadOnlyUbus::with_limits(
            transport,
            &local.rpcd_session,
            budget,
            spec.limits.max_response_bytes as usize,
        ) {
            Err(error) => observation_error(error),
            Ok(reader) => {
                let observed = observe(spec, &reader).unwrap_or_else(observation_error);
                if start.elapsed() > budget {
                    (
                        Outcome::Timeout,
                        "local observation exceeded invocation deadline".into(),
                    )
                } else {
                    observed
                }
            }
        }
    };
    ProbeResult {
        probe_id: spec.id.clone(),
        outcome: result.0,
        started_at_ms,
        finished_at_ms: crate::now_ms(),
        detail: result.1,
    }
}

fn observe<T: UbusTransport>(
    spec: &ProbeSpec,
    reader: &ReadOnlyUbus<T>,
) -> Result<(Outcome, String), ObservationError> {
    match &spec.expectation {
        Expectation::UciValue {
            package,
            section,
            option,
            form,
            values,
        } => {
            let sections = reader.committed_sections(package)?;
            let Some(section) = sections.iter().find(|s| &s.section == section) else {
                return Ok(verdict(false, "committed UCI section"));
            };
            let field = section.fields.iter().find(|f| match f {
                ConfigField::Option { name, .. }
                | ConfigField::List { name, .. }
                | ConfigField::Redacted { name } => name == option,
            });
            let matched = match (form, field) {
                (_, Some(ConfigField::Redacted { .. })) => {
                    return Ok((Outcome::Unavailable, "requested field is redacted".into()))
                }
                (UciFieldForm::Absent, None) => true,
                (UciFieldForm::Scalar, Some(ConfigField::Option { value, .. })) => {
                    values.as_slice() == [value.clone()]
                }
                (UciFieldForm::OrderedList, Some(ConfigField::List { values: actual, .. })) => {
                    values == actual
                }
                _ => false,
            };
            Ok(verdict(matched, "committed UCI field type and value"))
        }
        Expectation::UciSection {
            package,
            section,
            section_type,
        } => {
            let sections = reader.committed_sections(package)?;
            Ok(verdict(
                sections
                    .iter()
                    .any(|s| &s.section == section && &s.kind == section_type),
                "committed UCI section type",
            ))
        }
        Expectation::NetifdInterface {
            interface,
            up,
            addresses,
        } => {
            let collection = reader.interfaces()?;
            if collection.completeness != Completeness::Complete {
                return Ok((
                    Outcome::Unavailable,
                    "netifd interface observation is incomplete".into(),
                ));
            }
            let matches: Vec<_> = collection.facts.iter().filter(|fact| matches!(fact, OperationalFact::Interface {name, ..} if name == interface)).collect();
            match matches.as_slice() {
                [] => Ok(verdict(false, "netifd interface presence")),
                [OperationalFact::Interface {
                    up: actual_up,
                    addresses: actual,
                    ..
                }] => {
                    // Addresses are a set, unlike order-sensitive UCI list values.
                    let mut expected = addresses.iter().map(|a| parse_interface_address(a)).collect::<Option<Vec<_>>>().ok_or_else(|| ObservationError::Malformed("invalid expected address".into()))?;
                    expected.sort();
                    expected.dedup();
                    let mut actual = actual.iter().map(|a| parse_interface_address(a)).collect::<Option<Vec<_>>>().ok_or_else(|| ObservationError::Malformed("invalid observed address".into()))?;
                    actual.sort();
                    actual.dedup();
                    Ok(verdict(
                        up == actual_up && expected == actual,
                        "netifd interface status and exact address set",
                    ))
                }
                _ => Err(ObservationError::Malformed(
                    "duplicate netifd interface identity".into(),
                )),
            }
        }
        _ => Ok((
            Outcome::Unsupported,
            "expectation is not a device-local observation".into(),
        )),
    }
}

fn verdict(matched: bool, subject: &str) -> (Outcome, String) {
    (
        if matched {
            Outcome::Success
        } else {
            Outcome::Violation
        },
        format!(
            "{subject} {} expectation",
            if matched { "matches" } else { "differs from" }
        ),
    )
}
fn observation_error(error: ObservationError) -> (Outcome, String) {
    // Do not include RPC response content, field values, or session credentials.
    match error {
        ObservationError::Timeout => (Outcome::Timeout, "native ubus invocation timed out".into()),
        ObservationError::Malformed(_) | ObservationError::ResponseTooLarge => (
            Outcome::MalformedResponse,
            "native ubus response is malformed or exceeds limits".into(),
        ),
        ObservationError::UnsupportedMethod { .. } => (
            Outcome::Unsupported,
            "native ubus method is unsupported".into(),
        ),
        ObservationError::Unavailable(_) | ObservationError::Denied(_) => (
            Outcome::Unavailable,
            "read-only native ubus observation is unavailable or denied".into(),
        ),
    }
}
