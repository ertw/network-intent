use intent_controller::admission::{generate_claims, CompilerRunner, PlanningBindings};
use intent_protocol::payload_digest;
use std::{collections::HashMap, path::PathBuf};

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }
fn compiler() -> CompilerRunner {
    let path = root().join("netc");
    CompilerRunner::new(path).unwrap()
}

#[tokio::test]
async fn executes_authoritative_native_compiler_and_binds_exact_source() {
    let source = std::fs::read_to_string(root().join("examples/core-router.net")).unwrap();
    let draft = compiler().compile(&source).await.unwrap();
    assert_eq!(draft.source_digest(), payload_digest(source.as_bytes()));
    assert_eq!(draft.targets().len(), 1);
    assert_eq!(draft.witnesses().len(), 1);
    assert!(draft.witnesses()[0].claims.iter().any(|c| c.kind == "addressing"));
    assert!(draft.blockers().iter().any(|b| b.contains("firewall")));
    assert!(generate_claims(&draft, &PlanningBindings { sources: HashMap::new() }).is_err());

    let changed = format!("# source-preserved comment\n{source}");
    let changed_draft = compiler().compile(&changed).await.unwrap();
    assert_ne!(draft.source_digest(), changed_draft.source_digest());
}

#[tokio::test]
async fn invalid_source_cannot_become_a_compiled_draft() {
    assert!(compiler().compile("network-language 3.0\nnetwork bad { vlan x 5000 {} }\n").await.is_err());
    assert!(compiler().compile(&"x".repeat(1_048_577)).await.is_err());
}

#[tokio::test]
async fn every_target_has_a_checked_witness() {
    let source = std::fs::read_to_string(root().join("examples/wds-network.net")).unwrap();
    let draft = compiler().compile(&source).await.unwrap();
    assert_eq!(draft.targets().len(), 2);
    assert_eq!(draft.witnesses().len(), 2);
    let names: Vec<_> = draft.witnesses().iter().map(|w| w.target.as_str()).collect();
    assert!(names.contains(&"gateway") && names.contains(&"satellite"));
}
