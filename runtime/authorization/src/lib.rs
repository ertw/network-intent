//! Controller authorization using a fixed, versioned Cedar policy.
//!
//! `AuthorizationEngine` is constructed from controller-owned configuration.
//! Grants and current device bindings must be loaded from the authenticated
//! controller's trusted store; they are deliberately not part of an incoming
//! request. Callers must pass the exact authenticated identity and a binding
//! that is checked against that store on every decision.

use cedar_policy::{
    Authorizer, Context, Entities, Entity, EntityUid, PolicySet, Request, Schema, ValidationMode,
    Validator,
};
use intent_identity::AgentIdentity;
use intent_protocol::{AgentRole, DeviceBinding};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use thiserror::Error;

pub const AUTHORIZATION_VERSION: &str = "intent-authorization/v1";

/// The only operations exposed by the runtime authorization boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    AssignWitness,
    SubmitEvidence,
    AssignApply,
    SubmitApplyReceipt,
    ReadState,
    EnrollAgent,
    RevokeAgent,
}

impl Action {
    fn cedar_name(self) -> &'static str {
        match self {
            Self::AssignWitness => "AssignWitness",
            Self::SubmitEvidence => "SubmitEvidence",
            Self::AssignApply => "AssignApply",
            Self::SubmitApplyReceipt => "SubmitApplyReceipt",
            Self::ReadState => "ReadState",
            Self::EnrollAgent => "EnrollAgent",
            Self::RevokeAgent => "RevokeAgent",
        }
    }
}

/// A grant persisted by the controller. This is configuration, not request data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerGrant {
    pub identity: AgentIdentity,
    pub device_id: String,
    pub profile_digest: String,
    pub fencing_generation: u64,
}

/// A trusted current binding. The request binding is accepted only if it is
/// byte-for-byte equal to one of these records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedBinding(pub DeviceBinding);

#[derive(Clone, Debug, Default)]
pub struct ControllerScope {
    grants: Vec<ControllerGrant>,
    bindings: HashMap<String, DeviceBinding>,
}

impl ControllerScope {
    /// Build scope from controller-owned records after their integrity and
    /// freshness checks. Do not populate this from an untrusted request.
    pub fn new(
        grants: Vec<ControllerGrant>,
        bindings: Vec<TrustedBinding>,
    ) -> Result<Self, AuthorizationError> {
        let mut current = HashMap::new();
        for binding in bindings.into_iter().map(|b| b.0) {
            if current.insert(binding.device_id.clone(), binding).is_some() {
                return Err(AuthorizationError::Policy(
                    "duplicate trusted device binding".into(),
                ));
            }
        }
        Ok(Self {
            grants,
            bindings: current,
        })
    }
}

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("authorization policy or schema is invalid: {0}")]
    Policy(String),
    #[error("authorization evaluation failed: {0}")]
    Evaluation(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Allow,
    Deny,
}

pub struct AuthorizationEngine {
    authorizer: Authorizer,
    policies: PolicySet,
    schema: Schema,
    scope: ControllerScope,
}

impl AuthorizationEngine {
    pub fn new(scope: ControllerScope) -> Result<Self, AuthorizationError> {
        let schema =
            Schema::from_json_str(SCHEMA).map_err(|e| AuthorizationError::Policy(e.to_string()))?;
        let policies =
            PolicySet::from_str(POLICY).map_err(|e| AuthorizationError::Policy(e.to_string()))?;
        let validation = Validator::new(schema.clone()).validate(&policies, ValidationMode::Strict);
        if !validation.validation_passed() {
            return Err(AuthorizationError::Policy(validation.to_string()));
        }
        Ok(Self {
            authorizer: Authorizer::new(),
            policies,
            schema,
            scope,
        })
    }

    pub fn authorize(
        &self,
        authenticated: &AgentIdentity,
        action: Action,
        resource: &DeviceBinding,
    ) -> Result<Decision, AuthorizationError> {
        let trusted = self
            .scope
            .bindings
            .get(&resource.device_id)
            .ok_or_else(|| {
                AuthorizationError::Evaluation("device has no trusted current binding".into())
            })?;
        if trusted != resource {
            return Ok(Decision::Deny);
        }
        let grant = self.scope.grants.iter().find(|g| {
            g.identity == *authenticated
                && g.device_id == resource.device_id
                && g.profile_digest == resource.profile_digest
                && g.fencing_generation == resource.fencing_generation
        });
        let grant = match grant {
            Some(g) => g,
            None => return Ok(Decision::Deny),
        };
        let principal = EntityUid::from_str(&format!("Agent::\"{}\"", authenticated))
            .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?;
        let device = EntityUid::from_str(&format!("Device::\"{}\"", resource.device_id))
            .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?;
        let action_uid = EntityUid::from_str(&format!("Action::\"{}\"", action.cedar_name()))
            .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?;
        let fencing_generation = i64::try_from(resource.fencing_generation).map_err(|_| {
            AuthorizationError::Evaluation("fencing generation exceeds Cedar Long".into())
        })?;
        let entities = Entities::from_entities(
            [
                Entity::new(
                    principal.clone(),
                    HashMap::from([
                        (
                            "role".to_string(),
                            cedar_policy::RestrictedExpression::new_string(
                                role_name(authenticated.role()).to_owned(),
                            ),
                        ),
                        (
                            "trust_domain".to_string(),
                            cedar_policy::RestrictedExpression::new_string(
                                authenticated.trust_domain().to_owned(),
                            ),
                        ),
                        (
                            "device_id".to_string(),
                            cedar_policy::RestrictedExpression::new_string(
                                authenticated.device().to_owned(),
                            ),
                        ),
                    ]),
                    HashSet::new(),
                )
                .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?,
                Entity::new(
                    device.clone(),
                    HashMap::from([
                        (
                            "device_id".to_string(),
                            cedar_policy::RestrictedExpression::new_string(
                                resource.device_id.clone(),
                            ),
                        ),
                        (
                            "profile_digest".to_string(),
                            cedar_policy::RestrictedExpression::new_string(
                                resource.profile_digest.clone(),
                            ),
                        ),
                        (
                            "fencing_generation".to_string(),
                            cedar_policy::RestrictedExpression::new_long(fencing_generation),
                        ),
                    ]),
                    HashSet::new(),
                )
                .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?,
            ],
            Some(&self.schema),
        )
        .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?;
        let request = Request::new(
            principal,
            action_uid,
            device,
            Context::empty(),
            Some(&self.schema),
        )
        .map_err(|e| AuthorizationError::Evaluation(e.to_string()))?;
        let response = self
            .authorizer
            .is_authorized(&request, &self.policies, &entities);
        let errors = response
            .diagnostics()
            .errors()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(AuthorizationError::Evaluation(errors.join("; ")));
        }
        let _ = grant;
        Ok(if response.decision() == cedar_policy::Decision::Allow {
            Decision::Allow
        } else {
            Decision::Deny
        })
    }
}

fn role_name(role: AgentRole) -> &'static str {
    match role {
        AgentRole::Controller => "controller",
        AgentRole::Witness => "witness",
        AgentRole::Apply => "apply",
    }
}

pub const SCHEMA: &str = include_str!("schema.cedar.json");
pub const POLICY: &str = include_str!("policy.cedar");
