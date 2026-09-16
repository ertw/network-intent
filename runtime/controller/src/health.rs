//! Health state transitions over authenticated, assignment-bound evidence.
//! Serialize this record in the controller transaction that stores the evidence
//! and marks its inbox receipt processed. No browser state participates.
use intent_identity::evidence::VerifiedEvidence;
use intent_protocol::{assurance::Outcome, evidence::WitnessEvidence};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Degraded,
    Incident,
    Recovering,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthState {
    pub plan_id: String,
    pub plan_epoch: u64,
    pub graph_version: u64,
    pub probe_id: String,
    pub status: HealthStatus,
    pub incident_open: bool,
    pub consecutive_violations: u8,
    pub consecutive_successes: u8,
    pub last_evidence_id: Option<String>,
    pub last_finished_at_ms: Option<u64>,
    pub freshness_until_ms: Option<u64>,
    pub symptom: Option<Outcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthTransition {
    Updated,
    Opened,
    Recovered,
    Duplicate,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HealthError {
    #[error("evidence belongs to another plan epoch, graph or probe")]
    Scope,
    #[error("evidence completion does not follow the last observed invocation")]
    OutOfOrder,
    #[error("invalid freshness interval")]
    Interval,
}

impl HealthState {
    pub fn new(plan_id: String, plan_epoch: u64, graph_version: u64, probe_id: String) -> Self {
        Self {
            plan_id,
            plan_epoch,
            graph_version,
            probe_id,
            status: HealthStatus::Unknown,
            incident_open: false,
            consecutive_violations: 0,
            consecutive_successes: 0,
            last_evidence_id: None,
            last_finished_at_ms: None,
            freshness_until_ms: None,
            symptom: None,
        }
    }

    pub fn observe(
        &mut self,
        verified: &VerifiedEvidence,
        now_ms: u64,
    ) -> Result<HealthTransition, HealthError> {
        self.update(
            verified.evidence(),
            verified.outcome_at(now_ms),
            now_ms,
            verified.interval_ms(),
            verified.timeout_ms(),
        )
    }

    fn update(
        &mut self,
        e: &WitnessEvidence,
        outcome: Outcome,
        now_ms: u64,
        interval_ms: u64,
        timeout_ms: u64,
    ) -> Result<HealthTransition, HealthError> {
        if self.plan_id != e.plan_id
            || self.plan_epoch != e.plan_epoch
            || self.graph_version != e.graph_version
            || self.probe_id != e.result.probe_id
        {
            return Err(HealthError::Scope);
        }
        if interval_ms == 0 || timeout_ms == 0 {
            return Err(HealthError::Interval);
        }
        if self.last_evidence_id.as_ref() == Some(&e.evidence_id) {
            return Ok(HealthTransition::Duplicate);
        }
        if self
            .last_finished_at_ms
            .is_some_and(|t| e.result.started_at_ms <= t || e.result.finished_at_ms <= t)
        {
            return Err(HealthError::OutOfOrder);
        }
        // A long gap breaks consecutive success/violation streaks even if no
        // periodic expire call ran during the controller outage.
        self.expire(e.result.started_at_ms);
        self.last_evidence_id = Some(e.evidence_id.clone());
        self.last_finished_at_ms = Some(e.result.finished_at_ms);
        let fresh_until = e
            .result
            .finished_at_ms
            .saturating_add(interval_ms.saturating_mul(3))
            .saturating_add(timeout_ms);
        self.freshness_until_ms = Some(fresh_until);
        let outcome = if now_ms > fresh_until {
            Outcome::Unavailable
        } else {
            outcome
        };
        match outcome {
            Outcome::Success => {
                self.symptom = None;
                self.consecutive_violations = 0;
                self.consecutive_successes = self.consecutive_successes.saturating_add(1).min(2);
                if self.incident_open && self.consecutive_successes < 2 {
                    self.status = HealthStatus::Recovering;
                } else if self.incident_open {
                    self.incident_open = false;
                    self.status = HealthStatus::Healthy;
                    return Ok(HealthTransition::Recovered);
                } else {
                    self.status = HealthStatus::Healthy;
                }
            }
            Outcome::Violation
            | Outcome::Refused
            | Outcome::DnsNxDomain
            | Outcome::DnsServerFailure
            | Outcome::TlsFailure => {
                self.symptom = Some(outcome);
                self.consecutive_successes = 0;
                self.consecutive_violations = self.consecutive_violations.saturating_add(1).min(3);
                if self.consecutive_violations == 3 && !self.incident_open {
                    self.incident_open = true;
                    self.status = HealthStatus::Incident;
                    return Ok(HealthTransition::Opened);
                }
                self.status = if self.incident_open {
                    HealthStatus::Incident
                } else {
                    HealthStatus::Degraded
                };
            }
            // Ambiguity is retained; it cannot prove a policy or service claim.
            _ => self.make_unknown(Some(outcome)),
        }
        Ok(HealthTransition::Updated)
    }

    pub fn expire(&mut self, now_ms: u64) {
        if self.freshness_until_ms.is_none_or(|t| now_ms > t) {
            self.make_unknown(None);
        }
    }

    fn make_unknown(&mut self, symptom: Option<Outcome>) {
        self.status = HealthStatus::Unknown;
        self.consecutive_successes = 0;
        self.consecutive_violations = 0;
        self.symptom = symptom;
        // An open incident is never closed by missing evidence.
    }
}

/// Dependency health cannot be elevated by a healthy parent probe when any
/// required observation is unknown, degraded, incident or still recovering.
pub fn aggregate_health(required: &[HealthStatus]) -> HealthStatus {
    if required.is_empty() || required.contains(&HealthStatus::Unknown) {
        HealthStatus::Unknown
    } else if required.contains(&HealthStatus::Incident) {
        HealthStatus::Incident
    } else if required.contains(&HealthStatus::Degraded) {
        HealthStatus::Degraded
    } else if required.contains(&HealthStatus::Recovering) {
        HealthStatus::Recovering
    } else {
        HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intent_protocol::{
        assurance::ProbeResult, state::Completeness, DeviceBinding, RevisionRef,
    };
    fn observation(n: u64, outcome: Outcome) -> WitnessEvidence {
        WitnessEvidence {
            deployment: None,
            version: 1,
            evidence_id: format!("e{n}"),
            assignment_id: "a".into(),
            plan_id: "p".into(),
            plan_epoch: 1,
            graph_version: 1,
            revision: RevisionRef {
                id: "r".into(),
                source_digest: "d".into(),
            },
            device: DeviceBinding {
                device_id: "d".into(),
                profile_digest: "p".into(),
                fencing_generation: 1,
            },
            witness: "w".into(),
            probe_digest: "d".into(),
            completeness: Completeness::Complete,
            result: ProbeResult {
                probe_id: "probe".into(),
                outcome,
                started_at_ms: n * 30_000,
                finished_at_ms: n * 30_000 + 100,
                detail: "".into(),
            },
        }
    }
    fn state() -> HealthState {
        HealthState::new("p".into(), 1, 1, "probe".into())
    }
    fn feed(s: &mut HealthState, n: u64, outcome: Outcome) -> HealthTransition {
        let e = observation(n, outcome);
        s.update(&e, outcome, e.result.finished_at_ms, 30_000, 1_000)
            .unwrap()
    }
    #[test]
    fn incident_threshold_recovery_and_duplicate_across_restart() {
        let mut s = state();
        for n in 1..=2 {
            assert_eq!(
                feed(&mut s, n, Outcome::Violation),
                HealthTransition::Updated
            );
        }
        assert_eq!(
            feed(&mut s, 3, Outcome::Violation),
            HealthTransition::Opened
        );
        s = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(
            feed(&mut s, 3, Outcome::Violation),
            HealthTransition::Duplicate
        );
        assert_eq!(feed(&mut s, 4, Outcome::Success), HealthTransition::Updated);
        assert_eq!(s.status, HealthStatus::Recovering);
        assert_eq!(
            feed(&mut s, 5, Outcome::Success),
            HealthTransition::Recovered
        );
    }
    #[test]
    fn ambiguity_and_staleness_never_close_incident_or_accumulate_success() {
        let mut s = state();
        for n in 1..=3 {
            feed(&mut s, n, Outcome::Violation);
        }
        feed(&mut s, 4, Outcome::Success);
        feed(&mut s, 5, Outcome::Timeout);
        assert!(s.incident_open);
        assert_eq!(s.status, HealthStatus::Unknown);
        feed(&mut s, 6, Outcome::Success);
        assert_eq!(s.status, HealthStatus::Recovering);
        s.expire(400_000);
        assert!(s.incident_open);
        assert_eq!(s.consecutive_successes, 0);
        feed(&mut s, 15, Outcome::Success);
        assert_eq!(s.status, HealthStatus::Recovering);
    }
    #[test]
    fn rejects_out_of_order_and_foreign_plan_and_unknown_dependencies() {
        let mut s = state();
        feed(&mut s, 3, Outcome::Success);
        let e = observation(2, Outcome::Success);
        assert_eq!(
            s.update(&e, Outcome::Success, 100_000, 30_000, 1_000),
            Err(HealthError::OutOfOrder)
        );
        let mut e = observation(4, Outcome::Success);
        e.plan_epoch = 2;
        assert_eq!(
            s.update(&e, Outcome::Success, 150_000, 30_000, 1_000),
            Err(HealthError::Scope)
        );
        assert_eq!(
            aggregate_health(&[HealthStatus::Healthy, HealthStatus::Unknown]),
            HealthStatus::Unknown
        );
        assert_eq!(aggregate_health(&[]), HealthStatus::Unknown);
    }
}
