//! Explicit adoption and pure three-way UCI planning for the separate apply
//! agent. This crate contains no UCI client, shell execution, network client,
//! or configuration mutation path.
use intent_protocol::{
    payload_digest,
    state::{sensitive_field, ConfigField, ConfigSection},
    DeviceBinding, RevisionRef,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub mod journal;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedPath {
    pub package: String,
    pub section: String,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptedOwnership {
    pub version: u32,
    pub device: DeviceBinding,
    pub revision: RevisionRef,
    pub baseline_digest: String,
    pub fields: Vec<OwnedPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanBinding {
    pub device: DeviceBinding,
    pub baseline_revision: RevisionRef,
    pub desired_revision: RevisionRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPlan {
    pub device: DeviceBinding,
    pub revision: RevisionRef,
    pub before: Vec<ConfigSection>,
    pub after: Vec<ConfigSection>,
    pub affected: Vec<OwnedPath>,
    pub conflicts: Vec<PlanConflict>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanConflict {
    pub path: Option<OwnedPath>,
    pub reason: &'static str,
}
impl ApplyPlan {
    pub fn is_noop(&self) -> bool {
        self.before == self.after && self.conflicts.is_empty()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApplyError {
    #[error("configuration has ambiguous duplicate section or field identities")]
    AmbiguousIdentity,
    #[error("adoption binding or baseline digest does not match")]
    AdoptionBinding,
    #[error("requested device profile, fencing generation, or revision differs from adoption")]
    Binding,
    #[error("adoption scope is empty or refers to a missing baseline field")]
    InvalidOwnership,
    #[error("secret values cannot be adopted or changed by this planner")]
    SecretChange,
}

pub fn config_digest(config: &[ConfigSection]) -> Result<String, ApplyError> {
    serde_json::to_vec(config)
        .map(|v| payload_digest(&v))
        .map_err(|_| ApplyError::AmbiguousIdentity)
}

/// Adoption is explicit: caller supplies the exact owned fields and the
/// current committed baseline. The digest locks that baseline for later
/// three-way conflict detection.
pub fn adopt(
    device: DeviceBinding,
    revision: RevisionRef,
    baseline: &[ConfigSection],
    fields: Vec<OwnedPath>,
) -> Result<AdoptedOwnership, ApplyError> {
    index_sections(baseline)?;
    if fields.is_empty() {
        return Err(ApplyError::InvalidOwnership);
    }
    let mut seen = HashSet::new();
    for path in &fields {
        if !seen.insert(key(path)) || find_field(baseline, path)?.is_none() {
            return Err(ApplyError::InvalidOwnership);
        }
    }
    Ok(AdoptedOwnership {
        version: 1,
        device,
        revision,
        baseline_digest: config_digest(baseline)?,
        fields,
    })
}

/// Computes a plan but never writes. On ownership or drift conflicts `after`
/// remains the current configuration, so an accidental caller cannot treat a
/// partial plan as safe to apply.
pub fn plan(
    ownership: &AdoptedOwnership,
    binding: &PlanBinding,
    baseline: &[ConfigSection],
    current: &[ConfigSection],
    desired: &[ConfigSection],
) -> Result<ApplyPlan, ApplyError> {
    index_sections(baseline)?;
    index_sections(current)?;
    index_sections(desired)?;
    if binding.device != ownership.device || binding.baseline_revision != ownership.revision {
        return Err(ApplyError::Binding);
    }
    if ownership.version != 1
        || binding.desired_revision.id.is_empty()
        || binding.desired_revision.source_digest.is_empty()
    {
        return Err(ApplyError::InvalidOwnership);
    }
    if config_digest(baseline)? != ownership.baseline_digest {
        return Err(ApplyError::AdoptionBinding);
    }
    let mut conflicts = Vec::new();
    let owned: HashSet<_> = ownership.fields.iter().map(key).collect();
    let mut paths = HashSet::new();
    for path in &ownership.fields {
        if !paths.insert(key(path)) || find_field(baseline, path)?.is_none() {
            return Err(ApplyError::InvalidOwnership);
        }
    }
    // Field ownership cannot authorize section create/delete/type changes or
    // reordering. Compare desired to baseline, not current, so unrelated
    // device drift stays preserved when desired did not request a change.
    if section_shape(baseline) != section_shape(desired) {
        conflicts.push(PlanConflict {
            path: None,
            reason: "desired section structure, type, or order outside adopted ownership",
        });
    } else {
        for baseline_section in baseline {
            let desired_section =
                find_section_by_id(desired, baseline_section).expect("matching section shape");
            for field in &baseline_section.fields {
                let path = OwnedPath {
                    package: baseline_section.package.clone(),
                    section: baseline_section.section.clone(),
                    field: field_name(field).into(),
                };
                if !owned.contains(&key(&path))
                    && find_field(desired, &path)?.as_ref() != Some(field)
                {
                    conflicts.push(PlanConflict {
                        path: Some(path),
                        reason: "desired change outside adopted ownership",
                    });
                }
            }
            for field in &desired_section.fields {
                let path = OwnedPath {
                    package: desired_section.package.clone(),
                    section: desired_section.section.clone(),
                    field: field_name(field).into(),
                };
                if !owned.contains(&key(&path))
                    && find_field(baseline, &path)?.as_ref() != Some(field)
                {
                    conflicts.push(PlanConflict {
                        path: Some(path),
                        reason: "desired change outside adopted ownership",
                    });
                }
            }
        }
    }
    let mut after = current.to_vec();
    let mut affected = Vec::new();
    for path in &ownership.fields {
        let baseline_section = find_section(baseline, path).ok_or(ApplyError::InvalidOwnership)?;
        if let Some(current_section) = find_section(current, path) {
            if current_section.kind != baseline_section.kind {
                conflicts.push(PlanConflict {
                    path: Some(path.clone()),
                    reason: "owned section kind drifted from adoption baseline",
                });
                continue;
            }
        }
        if let Some(desired_section) = find_section(desired, path) {
            if desired_section.kind != baseline_section.kind {
                conflicts.push(PlanConflict {
                    path: Some(path.clone()),
                    reason: "desired section kind differs from adoption baseline",
                });
                continue;
            }
        }
        let base = find_field(baseline, path)?.ok_or(ApplyError::InvalidOwnership)?;
        let cur = find_field(current, path)?;
        let want = find_field(desired, path)?;
        if is_secret(&base)
            || cur.as_ref().is_some_and(|v| is_secret(v))
            || want.as_ref().is_some_and(|v| is_secret(v))
        {
            if want.as_ref() != Some(&base) {
                return Err(ApplyError::SecretChange);
            }
            continue;
        }
        // A current change is acceptable only when desired already equals it;
        // this makes retry/idempotency safe without overwriting drift.
        if cur.as_ref() != Some(&base) && want != cur {
            conflicts.push(PlanConflict {
                path: Some(path.clone()),
                reason: "owned field drifted from adoption baseline",
            });
            continue;
        }
        if want != cur {
            set_field(&mut after, path, want)?;
            affected.push(path.clone());
        }
    }
    if !conflicts.is_empty() {
        after = current.to_vec();
        affected.clear();
    }
    Ok(ApplyPlan {
        device: ownership.device.clone(),
        revision: binding.desired_revision.clone(),
        before: current.to_vec(),
        after,
        affected,
        conflicts,
    })
}

fn key(p: &OwnedPath) -> (String, String, String) {
    (p.package.clone(), p.section.clone(), p.field.clone())
}
fn index_sections(config: &[ConfigSection]) -> Result<(), ApplyError> {
    let mut sections = HashSet::new();
    for s in config {
        if !sections.insert((s.package.as_str(), s.section.as_str())) {
            return Err(ApplyError::AmbiguousIdentity);
        }
        let mut fields = HashSet::new();
        for f in &s.fields {
            if !fields.insert(field_name(f)) {
                return Err(ApplyError::AmbiguousIdentity);
            }
        }
    }
    Ok(())
}
fn field_name(f: &ConfigField) -> &str {
    match f {
        ConfigField::Option { name, .. }
        | ConfigField::List { name, .. }
        | ConfigField::Redacted { name } => name,
    }
}
fn find_field(
    config: &[ConfigSection],
    path: &OwnedPath,
) -> Result<Option<ConfigField>, ApplyError> {
    let section = config
        .iter()
        .find(|s| s.package == path.package && s.section == path.section);
    Ok(section.and_then(|s| {
        s.fields
            .iter()
            .find(|f| field_name(f) == path.field)
            .cloned()
    }))
}
fn find_section<'a>(config: &'a [ConfigSection], path: &OwnedPath) -> Option<&'a ConfigSection> {
    config
        .iter()
        .find(|s| s.package == path.package && s.section == path.section)
}
fn find_section_by_id<'a>(
    config: &'a [ConfigSection],
    wanted: &ConfigSection,
) -> Option<&'a ConfigSection> {
    config
        .iter()
        .find(|s| s.package == wanted.package && s.section == wanted.section)
}
fn section_shape(config: &[ConfigSection]) -> Vec<(&str, &str, &str)> {
    config
        .iter()
        .map(|s| (s.package.as_str(), s.section.as_str(), s.kind.as_str()))
        .collect()
}
fn set_field(
    config: &mut Vec<ConfigSection>,
    path: &OwnedPath,
    value: Option<ConfigField>,
) -> Result<(), ApplyError> {
    let section = config
        .iter_mut()
        .find(|s| s.package == path.package && s.section == path.section)
        .ok_or(ApplyError::InvalidOwnership)?;
    let index = section
        .fields
        .iter()
        .position(|f| field_name(f) == path.field)
        .ok_or(ApplyError::InvalidOwnership)?;
    match value {
        Some(value) => section.fields[index] = value,
        None => {
            section.fields.remove(index);
        }
    }
    Ok(())
}
fn is_secret(f: &ConfigField) -> bool {
    matches!(f, ConfigField::Redacted { .. }) || sensitive_field(field_name(f))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> (DeviceBinding, RevisionRef) {
        (
            DeviceBinding {
                device_id: "r".into(),
                profile_digest: "p".into(),
                fencing_generation: 1,
            },
            RevisionRef {
                id: "rev".into(),
                source_digest: "src".into(),
            },
        )
    }
    fn section(fields: Vec<ConfigField>) -> ConfigSection {
        ConfigSection {
            package: "network".into(),
            section: "lan".into(),
            kind: "interface".into(),
            fields,
        }
    }
    fn opt(n: &str, v: &str) -> ConfigField {
        ConfigField::Option {
            name: n.into(),
            value: v.into(),
        }
    }
    fn list(n: &str, v: &[&str]) -> ConfigField {
        ConfigField::List {
            name: n.into(),
            values: v.iter().map(|x| (*x).into()).collect(),
        }
    }
    fn owned() -> OwnedPath {
        OwnedPath {
            package: "network".into(),
            section: "lan".into(),
            field: "proto".into(),
        }
    }
    #[test]
    fn preserves_unowned_and_scalar_list_order() {
        let (d, r) = binding();
        let base = vec![section(vec![
            opt("proto", "dhcp"),
            list("dns", &["1", "2"]),
            opt("mtu", "1500"),
        ])];
        let own = adopt(d.clone(), r.clone(), &base, vec![owned()]).unwrap();
        let desired = vec![section(vec![
            opt("proto", "static"),
            list("dns", &["1", "2"]),
            opt("mtu", "1500"),
        ])];
        let p = plan(
            &own,
            &PlanBinding {
                device: d,
                baseline_revision: r.clone(),
                desired_revision: r,
            },
            &base,
            &base,
            &desired,
        )
        .unwrap();
        assert_eq!(
            p.after[0].fields,
            vec![
                opt("proto", "static"),
                list("dns", &["1", "2"]),
                opt("mtu", "1500")
            ]
        );
    }
    #[test]
    fn drift_and_binding_rejected() {
        let (d, r) = binding();
        let base = vec![section(vec![opt("proto", "dhcp")])];
        let own = adopt(d.clone(), r.clone(), &base, vec![owned()]).unwrap();
        let cur = vec![section(vec![opt("proto", "pppoe")])];
        let want = vec![section(vec![opt("proto", "static")])];
        let p = plan(
            &own,
            &PlanBinding {
                device: d.clone(),
                baseline_revision: r.clone(),
                desired_revision: RevisionRef {
                    id: "next".into(),
                    source_digest: "next-src".into(),
                },
            },
            &base,
            &cur,
            &want,
        )
        .unwrap();
        assert!(!p.conflicts.is_empty());
        assert!(matches!(
            plan(
                &own,
                &PlanBinding {
                    device: DeviceBinding {
                        fencing_generation: 2,
                        ..d
                    },
                    baseline_revision: r.clone(),
                    desired_revision: r
                },
                &base,
                &cur,
                &want
            ),
            Err(ApplyError::Binding)
        ));
    }
    #[test]
    fn deletion_noop_and_secrets() {
        let (d, r) = binding();
        let base = vec![section(vec![opt("proto", "dhcp"), opt("mtu", "1500")])];
        let own = adopt(d.clone(), r.clone(), &base, vec![owned()]).unwrap();
        let p = plan(
            &own,
            &PlanBinding {
                device: d.clone(),
                baseline_revision: r.clone(),
                desired_revision: r.clone(),
            },
            &base,
            &base,
            &base,
        )
        .unwrap();
        assert!(p.is_noop());
        let desired = vec![section(vec![opt("mtu", "1500")])];
        assert!(plan(
            &own,
            &PlanBinding {
                device: d,
                baseline_revision: r.clone(),
                desired_revision: r
            },
            &base,
            &base,
            &desired
        )
        .unwrap()
        .after[0]
            .fields
            .iter()
            .all(|f| field_name(f) != "proto"));
        let (d, r) = binding();
        let secret = vec![section(vec![ConfigField::Redacted {
            name: "password".into(),
        }])];
        let o = adopt(
            d.clone(),
            r.clone(),
            &secret,
            vec![OwnedPath {
                package: "network".into(),
                section: "lan".into(),
                field: "password".into(),
            }],
        )
        .unwrap();
        assert!(matches!(
            plan(
                &o,
                &PlanBinding {
                    device: d,
                    baseline_revision: r.clone(),
                    desired_revision: r
                },
                &secret,
                &secret,
                &vec![section(vec![opt("password", "x")])]
            ),
            Err(ApplyError::SecretChange)
        ));
    }
    #[test]
    fn duplicate_identity_rejected() {
        let (d, r) = binding();
        let b = vec![section(vec![opt("proto", "a"), opt("proto", "b")])];
        assert!(matches!(
            adopt(d, r, &b, vec![owned()]),
            Err(ApplyError::AmbiguousIdentity)
        ));
    }
    #[test]
    fn blocks_unowned_requests_and_section_kind_drift() {
        let (d, r) = binding();
        let base = vec![section(vec![opt("proto", "dhcp"), opt("mtu", "1500")])];
        let ownership = adopt(d.clone(), r.clone(), &base, vec![owned()]).unwrap();
        let binding = PlanBinding {
            device: d.clone(),
            baseline_revision: r.clone(),
            desired_revision: r.clone(),
        };
        let unowned = vec![section(vec![opt("proto", "dhcp"), opt("mtu", "1400")])];
        assert!(!plan(&ownership, &binding, &base, &base, &unowned)
            .unwrap()
            .conflicts
            .is_empty());
        let mut changed = section(vec![opt("proto", "dhcp"), opt("mtu", "1500")]);
        changed.kind = "device".into();
        assert!(!plan(&ownership, &binding, &base, &[changed], &base)
            .unwrap()
            .conflicts
            .is_empty());
    }
    #[test]
    fn blocks_unowned_section_create_type_and_reorder_but_preserves_current_drift() {
        let (d, r) = binding();
        let mut wan = section(vec![opt("proto", "dhcp")]);
        wan.section = "wan".into();
        let base = vec![
            section(vec![opt("proto", "dhcp"), opt("mtu", "1500")]),
            wan.clone(),
        ];
        let ownership = adopt(d.clone(), r.clone(), &base, vec![owned()]).unwrap();
        let binding = PlanBinding {
            device: d,
            baseline_revision: r.clone(),
            desired_revision: r,
        };
        let mut current = base.clone();
        current[0].fields[1] = opt("mtu", "1400");
        assert!(plan(&ownership, &binding, &base, &current, &base)
            .unwrap()
            .is_noop());
        let mut typed = base.clone();
        typed[1].kind = "device".into();
        assert!(!plan(&ownership, &binding, &base, &current, &typed)
            .unwrap()
            .conflicts
            .is_empty());
        let mut added = base.clone();
        let mut extra = section(vec![]);
        extra.section = "extra".into();
        added.push(extra);
        assert!(!plan(&ownership, &binding, &base, &current, &added)
            .unwrap()
            .conflicts
            .is_empty());
        let reordered = vec![base[1].clone(), base[0].clone()];
        assert!(!plan(&ownership, &binding, &base, &current, &reordered)
            .unwrap()
            .conflicts
            .is_empty());
    }
}
