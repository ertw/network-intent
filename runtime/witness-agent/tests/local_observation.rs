use intent_protocol::{
    assurance::{
        Expectation, Outcome, Primitive, ProbeSource, ProbeSpec, ResourceLimits, UciFieldForm,
    },
    DeviceBinding,
};
use intent_witness_agent::{
    adapters::{ObservationError, UbusTransport},
    local::{execute, LocalObservationContext},
};
use serde_json::Value;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone)]
struct Mock {
    replies: Arc<Mutex<Vec<Result<Value, ObservationError>>>>,
    calls: Arc<Mutex<Vec<(String, String, Value)>>>,
}
impl Mock {
    fn new(
        replies: Vec<Result<Value, ObservationError>>,
    ) -> (Self, Arc<Mutex<Vec<(String, String, Value)>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                replies: Arc::new(Mutex::new(replies)),
                calls: calls.clone(),
            },
            calls,
        )
    }
}
impl UbusTransport for Mock {
    fn invoke(
        &self,
        object: &str,
        method: &str,
        request: &Value,
        _timeout: Duration,
        _max: usize,
    ) -> Result<Value, ObservationError> {
        self.calls
            .lock()
            .unwrap()
            .push((object.into(), method.into(), request.clone()));
        self.replies.lock().unwrap().remove(0)
    }
}

fn binding() -> DeviceBinding {
    DeviceBinding {
        device_id: "router-a".into(),
        profile_digest: "profile-a".into(),
        fencing_generation: 4,
    }
}
fn source() -> ProbeSource {
    ProbeSource {
        witness_id: "witness-a".into(),
        location: "lab".into(),
        bind_address: "192.0.2.2".parse().unwrap(),
        interface: None,
    }
}
fn spec(expectation: Expectation) -> ProbeSpec {
    ProbeSpec {
        id: "probe-a".into(),
        primitive: match expectation {
            Expectation::NetifdInterface { .. } => Primitive::NetifdState,
            _ => Primitive::UciReadback,
        },
        device: binding(),
        source: source(),
        endpoint: None,
        expectation,
        limits: ResourceLimits {
            timeout_ms: 500,
            max_response_bytes: 4096,
            max_attempts: 1,
        },
        interval_ms: 60_000,
        depends_on: vec![],
        claims: vec![],
    }
}
fn context() -> LocalObservationContext {
    LocalObservationContext {
        device: binding(),
        source: source(),
        rpcd_session: "session-a".into(),
    }
}
fn access() -> Vec<Result<Value, ObservationError>> {
    let mut r = (0..5)
        .map(|_| Ok(serde_json::json!({"access":true})))
        .collect::<Vec<_>>();
    r.extend((0..7).map(|_| Ok(serde_json::json!({"access":false}))));
    r
}
fn uci_reply(_extra: Value) -> Value {
    serde_json::json!({"values":{"lan": {".type":"interface", ".name":"lan", ".index":1, "proto":"dhcp", "dns":["1.1.1.1","8.8.8.8"], "password":"secret"}, "other":{".type":"interface", ".name":"other", ".index":2, "proto":"static"}}})
}
fn uci_replies(extra: Value, changes: Value) -> Vec<Result<Value, ObservationError>> {
    let mut r = access();
    r.extend([
        Ok(serde_json::json!({"changes":changes.clone()})),
        Ok(uci_reply(extra)),
        Ok(serde_json::json!({"changes":changes})),
    ]);
    r
}
fn run(
    s: ProbeSpec,
    replies: Vec<Result<Value, ObservationError>>,
) -> (Outcome, Arc<Mutex<Vec<(String, String, Value)>>>) {
    let (mock, calls) = Mock::new(replies);
    (execute(&s, &context(), mock).outcome, calls)
}

#[test]
fn scalar_list_order_and_absence_are_distinct() {
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "proto".into(),
                form: UciFieldForm::Scalar,
                values: vec!["dhcp".into()]
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Success
    );
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "dns".into(),
                form: UciFieldForm::OrderedList,
                values: vec!["1.1.1.1".into(), "8.8.8.8".into()]
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Success
    );
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "dns".into(),
                form: UciFieldForm::OrderedList,
                values: vec!["8.8.8.8".into(), "1.1.1.1".into()]
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Violation
    );
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "missing".into(),
                form: UciFieldForm::Absent,
                values: vec![]
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Success
    );
}

#[test]
fn section_kind_redaction_staging_denial_and_malformed_are_not_success() {
    assert_eq!(
        run(
            spec(Expectation::UciSection {
                package: "network".into(),
                section: "lan".into(),
                section_type: "interface".into()
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Success
    );
    assert_eq!(
        run(
            spec(Expectation::UciSection {
                package: "network".into(),
                section: "lan".into(),
                section_type: "wrong".into()
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Violation
    );
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "password".into(),
                form: UciFieldForm::Scalar,
                values: vec!["secret".into()]
            }),
            uci_replies(serde_json::json!(null), serde_json::json!([]))
        )
        .0,
        Outcome::Unsupported
    );
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "proto".into(),
                form: UciFieldForm::Scalar,
                values: vec!["dhcp".into()]
            }),
            uci_replies(
                serde_json::json!(null),
                serde_json::json!(["lan.proto=static"])
            )
        )
        .0,
        Outcome::Unavailable
    );
    let mut denied = access();
    denied[0] = Ok(serde_json::json!({"access":false}));
    let (outcome, _) = run(
        spec(Expectation::UciValue {
            package: "network".into(),
            section: "lan".into(),
            option: "proto".into(),
            form: UciFieldForm::Scalar,
            values: vec!["dhcp".into()],
        }),
        denied,
    );
    assert_eq!(outcome, Outcome::Unavailable);
    let mut malformed = access();
    malformed.extend([
        Ok(serde_json::json!({"changes":[]})),
        Ok(serde_json::json!({"values":{"lan":{".type":"interface",".index":1,"proto":true}}})),
        Ok(serde_json::json!({"changes":[]})),
    ]);
    assert_eq!(
        run(
            spec(Expectation::UciValue {
                package: "network".into(),
                section: "lan".into(),
                option: "proto".into(),
                form: UciFieldForm::Scalar,
                values: vec!["dhcp".into()]
            }),
            malformed
        )
        .0,
        Outcome::MalformedResponse
    );
}

#[test]
fn mismatched_local_binding_or_source_does_zero_rpc() {
    let (mock, calls) = Mock::new(vec![]);
    let mut c = context();
    c.device.profile_digest = "stale".into();
    let result = execute(
        &spec(Expectation::UciSection {
            package: "network".into(),
            section: "lan".into(),
            section_type: "interface".into(),
        }),
        &c,
        mock,
    );
    assert_eq!(result.outcome, Outcome::Unavailable);
    assert!(calls.lock().unwrap().is_empty());
    let (mock, calls) = Mock::new(vec![]);
    let mut c = context();
    c.device.fencing_generation = 3;
    let result = execute(
        &spec(Expectation::UciSection {
            package: "network".into(),
            section: "lan".into(),
            section_type: "interface".into(),
        }),
        &c,
        mock,
    );
    assert_eq!(result.outcome, Outcome::Unavailable);
    assert!(calls.lock().unwrap().is_empty());
    let (mock, calls) = Mock::new(vec![]);
    let mut c = context();
    c.source.witness_id = "other".into();
    let result = execute(
        &spec(Expectation::UciSection {
            package: "network".into(),
            section: "lan".into(),
            section_type: "interface".into(),
        }),
        &c,
        mock,
    );
    assert_eq!(result.outcome, Outcome::Unavailable);
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn timeout_and_netifd_exactness_never_promote_unknown() {
    let (outcome, _) = run(
        spec(Expectation::UciSection {
            package: "network".into(),
            section: "lan".into(),
            section_type: "interface".into(),
        }),
        vec![Err(ObservationError::Timeout)],
    );
    assert_eq!(outcome, Outcome::Timeout);
    let mut r = access();
    r.push(Ok(serde_json::json!({"interface":[{"interface":"lan","up":true,"ipv4-address":[{"address":"192.0.2.1","mask":24}],"ipv6-address":[]}]})));
    let (mock, _) = Mock::new(r);
    let s = spec(Expectation::NetifdInterface {
        interface: "lan".into(),
        up: true,
        addresses: vec!["192.0.2.1/24".into()],
    });
    assert_eq!(execute(&s, &context(), mock).outcome, Outcome::Success);
    let mut r = access();
    r.push(Ok(serde_json::json!({"interface":[{"interface":"lan","up":true,"ipv4-address":[{"address":"192.0.2.1","mask":24}]}]})));
    let (mock, _) = Mock::new(r);
    assert_eq!(execute(&s, &context(), mock).outcome, Outcome::Unavailable);
    let mut r = access();
    r.push(Ok(serde_json::json!({"interface":[{"interface":"lan","up":true,"ipv4-address":[],"ipv6-address":[]},{"interface":"lan","up":true,"ipv4-address":[],"ipv6-address":[]}]})));
    let (mock, _) = Mock::new(r);
    assert_eq!(
        execute(&s, &context(), mock).outcome,
        Outcome::MalformedResponse
    );
}

#[test]
fn netifd_compares_ip_semantics_and_rejects_bad_prefixes_before_rpc() {
    let mut replies = access();
    replies.push(Ok(serde_json::json!({"interface":[{
        "interface":"lan", "up":true, "ipv4-address":[],
        "ipv6-address":[{"address":"2001:0db8:0:0:0:0:0:1","mask":64}]
    }]})));
    let (mock, _) = Mock::new(replies);
    let equivalent = spec(Expectation::NetifdInterface {
        interface: "lan".into(),
        up: true,
        addresses: vec!["2001:db8::1/64".into()],
    });
    assert_eq!(
        execute(&equivalent, &context(), mock).outcome,
        Outcome::Success
    );

    let mut replies = access();
    replies.push(Ok(serde_json::json!({"interface":[{
        "interface":"lan", "up":true,
        "ipv4-address":[{"address":"192.0.2.1","mask":24}], "ipv6-address":[]
    }]})));
    let (mock, _) = Mock::new(replies);
    let mismatch = spec(Expectation::NetifdInterface {
        interface: "lan".into(),
        up: true,
        addresses: vec!["192.0.2.2/24".into()],
    });
    assert_eq!(
        execute(&mismatch, &context(), mock).outcome,
        Outcome::Violation
    );

    for address in [
        "192.0.2.1/33",
        "2001:db8::1/129",
        "192.0.2.1/bad",
        "192.0.2.1/24/1",
    ] {
        let (mock, calls) = Mock::new(vec![]);
        let malformed = spec(Expectation::NetifdInterface {
            interface: "lan".into(),
            up: true,
            addresses: vec![address.into()],
        });
        assert_eq!(
            execute(&malformed, &context(), mock).outcome,
            Outcome::Unsupported
        );
        assert!(
            calls.lock().unwrap().is_empty(),
            "malformed {address} reached RPC"
        );
    }
}
