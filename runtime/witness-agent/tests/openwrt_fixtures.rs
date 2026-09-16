use intent_witness_agent::adapters::{parse_netifd_devices, parse_netifd_interfaces, parse_netifd_wireless, parse_uci_get};
use intent_protocol::state::{ConfigField, OperationalFact};
use serde_json::Value;

fn fixture(version: &str, name: &str) -> Value {
    let path = format!("{}/../../lab/openwrt/fixtures/{version}/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn pinned_openwrt_fixtures_preserve_typed_config_and_runtime_facts() {
    for version in ["25.12.5", "24.10.8"] {
        let sections = parse_uci_get("network", &fixture(version, "uci-network")).unwrap();
        assert_eq!(sections.iter().map(|s| s.section.as_str()).collect::<Vec<_>>(), ["loopback", "globals", "cfg030f15", "lan"]);
        assert!(sections[3].fields.iter().any(|field| matches!(field, ConfigField::Option { name, value } if name == "proto" && value == "static")));
        assert!(sections[2].fields.iter().any(|field| matches!(field, ConfigField::List { name, values } if name == "ports" && values == &["eth0"])), "{version}");

        let interfaces = parse_netifd_interfaces(&fixture(version, "netifd-interfaces")).unwrap();
        assert_eq!(interfaces.completeness, intent_protocol::state::Completeness::Complete, "{version}");
        let lan = interfaces.facts.iter().find(|fact| matches!(fact, OperationalFact::Interface { name, .. } if name == "lan")).unwrap();
        assert!(matches!(lan, OperationalFact::Interface { addresses, .. } if addresses.iter().any(|address| address.ends_with("/60") && address.contains(":"))), "{version}");
        let loopback = interfaces.facts.iter().find(|fact| matches!(fact, OperationalFact::Interface { name, .. } if name == "loopback")).unwrap();
        assert!(matches!(loopback, OperationalFact::Interface { addresses, .. } if addresses.iter().any(|address| address == "127.0.0.1/8")), "{version}");

        let devices = parse_netifd_devices(&fixture(version, "netifd-devices")).unwrap();
        assert!(devices.facts.iter().any(|fact| matches!(fact, OperationalFact::Device { name, members, .. } if name == "br-lan" && members == &["eth0"])), "{version}");
        if version == "24.10.8" {
            assert!(parse_netifd_wireless(&fixture(version, "netifd-wireless")).unwrap().facts.is_empty());
        }
    }
}

#[test]
fn prefix_assignment_shape_is_strict_and_missing_is_partial() {
    let base = serde_json::json!({"interface":[{"interface":"lan","up":true,"ipv4-address":[],"ipv6-address":[],"ipv6-prefix-assignment":[]}]});
    assert!(matches!(parse_netifd_interfaces(&base).unwrap().completeness, intent_protocol::state::Completeness::Complete));
    let mut missing = base.clone();
    missing["interface"][0].as_object_mut().unwrap().remove("ipv6-prefix-assignment");
    assert!(matches!(parse_netifd_interfaces(&missing).unwrap().completeness, intent_protocol::state::Completeness::Partial { .. }));
    let malformed = serde_json::json!({"interface":[{"interface":"lan","up":true,"ipv4-address":[],"ipv6-address":[],"ipv6-prefix-assignment":[{"local-address":{"address":"not-an-ip","mask":64}}]}]});
    assert!(parse_netifd_interfaces(&malformed).is_err());
}
