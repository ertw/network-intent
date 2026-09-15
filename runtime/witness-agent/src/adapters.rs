//! Read-only OpenWrt observation contracts and strict response parsers.
use intent_protocol::state::{Completeness, ConfigField, ConfigSection, OperationalFact};
use serde_json::Value;
use std::{
    collections::HashSet,
    fmt,
    time::{Duration, Instant},
};

#[path = "native_ubus.rs"]
pub mod native_ubus;

const READ_PERMISSIONS: [(&str, &str); 5] = [
    ("uci", "get"),
    ("uci", "changes"),
    ("network.interface", "dump"),
    ("network.device", "status"),
    ("network.wireless", "status"),
];
const WRITE_METHODS: [&str; 7] = [
    "set", "add", "delete", "commit", "apply", "confirm", "rollback",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    UnsupportedMethod { object: String, method: String },
    Unavailable(String),
    Timeout,
    Denied(String),
    ResponseTooLarge,
    Malformed(String),
}
impl fmt::Display for ObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ObservationError {}

/// This boundary has no generic RPC method: all invocations are checked twice.
pub trait UbusTransport {
    fn invoke(
        &self,
        object: &str,
        method: &str,
        request: &Value,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<Value, ObservationError>;
}
pub fn read_only_method(object: &str, method: &str) -> bool {
    matches!(
        (object, method),
        ("session", "access")
            | ("uci", "changes")
            | ("uci", "get")
            | ("network.interface", "dump")
            | ("network.device", "status")
            | ("network.wireless", "status")
    )
}

pub struct ReadOnlyUbus<T> {
    transport: T,
    session_id: String,
    invocation_timeout: Duration,
    max_response_bytes: usize,
}
impl<T: UbusTransport> ReadOnlyUbus<T> {
    /// Limits are supplied by the signed probe assignment. There is no local
    /// default: a caller must make the resource budget explicit.
    pub fn with_limits(
        transport: T,
        session_id: impl Into<String>,
        invocation_timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<Self, ObservationError> {
        let session_id = session_id.into();
        if session_id.is_empty()
            || session_id.len() > 128
            || session_id.chars().any(char::is_control)
        {
            return Err(ObservationError::Malformed(
                "invalid rpcd session ID".into(),
            ));
        }
        if invocation_timeout.is_zero() || max_response_bytes == 0 {
            return Err(ObservationError::Malformed(
                "read-only ubus limits must be non-zero".into(),
            ));
        }
        Ok(Self {
            transport,
            session_id,
            invocation_timeout,
            max_response_bytes,
        })
    }
    fn deadline(&self) -> Instant {
        Instant::now() + self.invocation_timeout
    }
    fn call_until(
        &self,
        deadline: Instant,
        object: &str,
        method: &str,
        request: Value,
    ) -> Result<Value, ObservationError> {
        if !read_only_method(object, method) {
            return Err(ObservationError::UnsupportedMethod {
                object: object.into(),
                method: method.into(),
            });
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ObservationError::Timeout);
        }
        self.transport
            .invoke(object, method, &request, remaining, self.max_response_bytes)
    }
    pub fn verify_read_session(&self) -> Result<(), ObservationError> {
        self.authorize_until(self.deadline())
    }
    fn authorize_until(&self, deadline: Instant) -> Result<(), ObservationError> {
        for (object, method) in READ_PERMISSIONS {
            self.require_access(deadline, object, method, true)?;
        }
        for method in WRITE_METHODS {
            self.require_access(deadline, "uci", method, false)?;
        }
        Ok(())
    }
    fn require_access(
        &self,
        deadline: Instant,
        object: &str,
        method: &str,
        expected: bool,
    ) -> Result<(), ObservationError> {
        let value = self.call_until(
            deadline,
            "session",
            "access",
            serde_json::json!({"ubus_rpc_session":self.session_id,"scope":"ubus","object":object,"function":method}),
        )?;
        match value.get("access").and_then(Value::as_bool) {
            Some(value) if value == expected => Ok(()),
            Some(_) if expected => Err(ObservationError::Denied(
                "rpcd session lacks required read access".into(),
            )),
            Some(_) => Err(ObservationError::Denied(
                "rpcd session grants forbidden UCI write access".into(),
            )),
            None => Err(ObservationError::Malformed(
                "session access reply lacks boolean access".into(),
            )),
        }
    }
    /// rpcd `uci get` overlays its session delta. A committed label is only
    /// truthful after empty `changes` checks immediately before and after it.
    pub fn committed_sections(
        &self,
        package: &str,
    ) -> Result<Vec<ConfigSection>, ObservationError> {
        validate_package(package)?;
        let deadline = self.deadline();
        self.authorize_until(deadline)?;
        self.assert_empty_staging(deadline, package)?;
        let reply = self.call_until(
            deadline,
            "uci",
            "get",
            serde_json::json!({"config":package,"ubus_rpc_session":self.session_id}),
        )?;
        let sections = parse_uci_get(package, &reply)?;
        self.assert_empty_staging(deadline, package)?;
        Ok(sections)
    }
    /// This is the merged rpcd session view and must be stored as SessionStaging.
    pub fn session_staging_sections(
        &self,
        package: &str,
    ) -> Result<Vec<ConfigSection>, ObservationError> {
        validate_package(package)?;
        let deadline = self.deadline();
        self.authorize_until(deadline)?;
        parse_uci_get(
            package,
            &self.call_until(
                deadline,
                "uci",
                "get",
                serde_json::json!({"config":package,"ubus_rpc_session":self.session_id}),
            )?,
        )
    }
    fn assert_empty_staging(
        &self,
        deadline: Instant,
        package: &str,
    ) -> Result<(), ObservationError> {
        let reply = self.call_until(
            deadline,
            "uci",
            "changes",
            serde_json::json!({"config":package,"ubus_rpc_session":self.session_id}),
        )?;
        let changes = reply
            .get("changes")
            .ok_or_else(|| ObservationError::Malformed("uci changes reply lacks changes".into()))?;
        if changes.as_array().is_some_and(|v| v.is_empty())
            || changes.as_object().is_some_and(|v| v.is_empty())
        {
            Ok(())
        } else {
            Err(ObservationError::Unavailable(
                "rpcd session contains staging changes; refusing committed label".into(),
            ))
        }
    }
    pub fn interfaces(&self) -> Result<OperationalCollection, ObservationError> {
        let deadline = self.deadline();
        self.authorize_until(deadline)?;
        parse_netifd_interfaces(&self.call_until(
            deadline,
            "network.interface",
            "dump",
            serde_json::json!({"ubus_rpc_session":self.session_id}),
        )?)
    }
    pub fn devices(&self) -> Result<OperationalCollection, ObservationError> {
        let deadline = self.deadline();
        self.authorize_until(deadline)?;
        parse_netifd_devices(&self.call_until(
            deadline,
            "network.device",
            "status",
            serde_json::json!({"ubus_rpc_session":self.session_id}),
        )?)
    }
    pub fn wireless(&self) -> Result<OperationalCollection, ObservationError> {
        let deadline = self.deadline();
        self.authorize_until(deadline)?;
        parse_netifd_wireless(&self.call_until(
            deadline,
            "network.wireless",
            "status",
            serde_json::json!({"ubus_rpc_session":self.session_id}),
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalCollection {
    pub facts: Vec<OperationalFact>,
    pub completeness: Completeness,
}

pub fn parse_uci_get(package: &str, reply: &Value) -> Result<Vec<ConfigSection>, ObservationError> {
    validate_package(package)?;
    let values = reply
        .get("values")
        .and_then(Value::as_object)
        .ok_or_else(|| ObservationError::Malformed("uci get reply lacks object values".into()))?;
    let mut parsed = Vec::with_capacity(values.len());
    let mut indices = HashSet::new();
    for (section_id, raw) in values {
        validate_identifier(section_id, "UCI section ID")?;
        let object = raw.as_object().ok_or_else(|| {
            ObservationError::Malformed(format!("section {section_id} is not an object"))
        })?;
        let kind = required_string(object.get(".type"), ".type")?;
        validate_identifier(kind, "UCI section type")?;
        if object
            .get(".name")
            .is_some_and(|v| v.as_str() != Some(section_id.as_str()))
        {
            return Err(ObservationError::Malformed(format!(
                "section {section_id} has mismatched .name"
            )));
        }
        let index = object
            .get(".index")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ObservationError::Malformed(format!("section {section_id} lacks integer .index"))
            })?;
        if !indices.insert(index) {
            return Err(ObservationError::Malformed(
                "duplicate UCI section .index".into(),
            ));
        }
        let mut fields = Vec::new();
        for (name, value) in object {
            if name.starts_with('.') {
                if !matches!(name.as_str(), ".type" | ".name" | ".index" | ".anonymous") {
                    return Err(ObservationError::Malformed(format!(
                        "unknown UCI metadata field {name}"
                    )));
                }
                continue;
            }
            validate_identifier(name, "UCI option")?;
            fields.push(parse_uci_field(name, value)?);
        }
        parsed.push((
            index,
            ConfigSection {
                package: package.into(),
                section: section_id.clone(),
                kind: kind.into(),
                fields,
            },
        ));
    }
    parsed.sort_by_key(|(index, _)| *index);
    Ok(parsed.into_iter().map(|(_, section)| section).collect())
}
fn parse_uci_field(name: &str, value: &Value) -> Result<ConfigField, ObservationError> {
    if intent_protocol::state::sensitive_field(name) {
        return match value {
            Value::String(_) | Value::Array(_) => Ok(ConfigField::Redacted { name: name.into() }),
            _ => Err(ObservationError::Malformed(format!(
                "sensitive UCI field {name} is not string/list"
            ))),
        };
    }
    match value {
        Value::String(value) => Ok(ConfigField::Option {
            name: name.into(),
            value: value.clone(),
        }),
        Value::Array(values) => Ok(ConfigField::List {
            name: name.into(),
            values: values
                .iter()
                .map(|v| required_string(Some(v), name).map(str::to_owned))
                .collect::<Result<_, _>>()?,
        }),
        _ => Err(ObservationError::Malformed(format!(
            "UCI field {name} is not string/list"
        ))),
    }
}

pub fn parse_netifd_interfaces(reply: &Value) -> Result<OperationalCollection, ObservationError> {
    let entries = reply
        .get("interface")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ObservationError::Malformed("network.interface dump lacks interface array".into())
        })?;
    let mut facts = Vec::with_capacity(entries.len());
    let mut missing = Vec::new();
    for raw in entries {
        let o = raw.as_object().ok_or_else(|| {
            ObservationError::Malformed("interface entry is not an object".into())
        })?;
        let name = required_string(o.get("interface"), "interface")?;
        let up = required_bool(o.get("up"), "interface up")?;
        let device = optional_string(o.get("device"), "interface device")?.map(str::to_owned);
        let interface_addresses = parse_addresses(o.get("ipv4-address"), o.get("ipv6-address"))?;
        // netifd does not promise address arrays for every interface state.
        // Absence is unknown addressing, never proof of an empty address set.
        if o.get("ipv4-address").is_none() || o.get("ipv6-address").is_none() {
            missing.push(format!("interface:{name}:address-family"));
        }
        facts.push(OperationalFact::Interface {
            name: name.into(),
            up,
            device,
            addresses: interface_addresses,
        });
    }
    Ok(OperationalCollection {
        facts,
        completeness: if missing.is_empty() {
            Completeness::Complete
        } else {
            Completeness::Partial { missing }
        },
    })
}
pub fn parse_netifd_devices(reply: &Value) -> Result<OperationalCollection, ObservationError> {
    // netifd's `device_dump_status(..., NULL)` writes one named table per
    // present device; it does not wrap them in a `devices` array.
    let entries = reply.as_object().ok_or_else(|| {
        ObservationError::Malformed("network.device status is not an object".into())
    })?;
    let mut facts = Vec::with_capacity(entries.len());
    for (name, raw) in entries {
        validate_identifier(name, "device name")?;
        let o = raw
            .as_object()
            .ok_or_else(|| ObservationError::Malformed("device entry is not an object".into()))?;
        let carrier = match o.get("carrier") {
            Some(v) => Some(required_bool(Some(v), "device carrier")?),
            None => None,
        };
        facts.push(OperationalFact::Device {
            name: name.clone(),
            present: required_bool(o.get("present"), "device present")?,
            carrier,
            members: string_array(o.get("bridge-members"), "bridge-members")?,
        });
    }
    Ok(OperationalCollection {
        facts,
        completeness: Completeness::Complete,
    })
}
pub fn parse_netifd_wireless(reply: &Value) -> Result<OperationalCollection, ObservationError> {
    let radios = reply.as_object().ok_or_else(|| {
        ObservationError::Malformed("network.wireless status is not an object".into())
    })?;
    let mut facts = Vec::with_capacity(radios.len());
    for (name, raw) in radios {
        let o = raw.as_object().ok_or_else(|| {
            ObservationError::Malformed(format!("wireless radio {name} is not an object"))
        })?;
        facts.push(OperationalFact::Wireless {
            name: name.clone(),
            up: required_bool(o.get("up"), "wireless up")?,
            pending: required_bool(o.get("pending"), "wireless pending")?,
        });
    }
    Ok(OperationalCollection {
        facts,
        completeness: Completeness::Complete,
    })
}
fn parse_addresses(
    v4: Option<&Value>,
    v6: Option<&Value>,
) -> Result<Vec<String>, ObservationError> {
    let mut addresses = Vec::new();
    for (value, family) in [(v4, 4), (v6, 6)] {
        let Some(value) = value else { continue };
        for entry in value
            .as_array()
            .ok_or_else(|| ObservationError::Malformed("address field is not an array".into()))?
        {
            let o = entry.as_object().ok_or_else(|| {
                ObservationError::Malformed("address entry is not an object".into())
            })?;
            let address = required_string(o.get("address"), "address")?;
            let mask = o
                .get("mask")
                .and_then(Value::as_u64)
                .ok_or_else(|| ObservationError::Malformed("address mask missing".into()))?;
            if address
                .parse::<std::net::IpAddr>()
                .map_or(true, |ip| (family == 4) != ip.is_ipv4())
                || mask > if family == 4 { 32 } else { 128 }
            {
                return Err(ObservationError::Malformed("invalid netifd address".into()));
            }
            addresses.push(format!("{address}/{mask}"));
        }
    }
    Ok(addresses)
}
fn string_array(value: Option<&Value>, field: &str) -> Result<Vec<String>, ObservationError> {
    match value {
        None => Ok(Vec::new()),
        Some(value) => value
            .as_array()
            .ok_or_else(|| ObservationError::Malformed(format!("{field} is not an array")))?
            .iter()
            .map(|v| required_string(Some(v), field).map(str::to_owned))
            .collect(),
    }
}
fn required_string<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a str, ObservationError> {
    value
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .ok_or_else(|| ObservationError::Malformed(format!("missing/invalid {field}")))
}
fn optional_string<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<Option<&'a str>, ObservationError> {
    match value {
        None => Ok(None),
        Some(value) => required_string(Some(value), field).map(Some),
    }
}
fn required_bool(value: Option<&Value>, field: &str) -> Result<bool, ObservationError> {
    value
        .and_then(Value::as_bool)
        .ok_or_else(|| ObservationError::Malformed(format!("missing/invalid {field}")))
}
fn validate_package(package: &str) -> Result<(), ObservationError> {
    if matches!(package, "network" | "dhcp" | "firewall" | "wireless") {
        Ok(())
    } else {
        Err(ObservationError::Malformed(
            "package is outside read-only witness scope".into(),
        ))
    }
}
fn validate_identifier(value: &str, field: &str) -> Result<(), ObservationError> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Ok(())
    } else {
        Err(ObservationError::Malformed(format!("invalid {field}")))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque};
    struct Mock {
        calls: RefCell<Vec<(String, String, Value)>>,
        limits: RefCell<Vec<(Duration, usize)>>,
        replies: RefCell<VecDeque<Result<Value, ObservationError>>>,
    }
    impl Mock {
        fn new(replies: Vec<Result<Value, ObservationError>>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                limits: RefCell::new(Vec::new()),
                replies: RefCell::new(replies.into()),
            }
        }
    }
    impl UbusTransport for Mock {
        fn invoke(
            &self,
            object: &str,
            method: &str,
            request: &Value,
            timeout: Duration,
            max_response_bytes: usize,
        ) -> Result<Value, ObservationError> {
            self.calls
                .borrow_mut()
                .push((object.into(), method.into(), request.clone()));
            self.limits.borrow_mut().push((timeout, max_response_bytes));
            self.replies
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err(ObservationError::Unavailable("unexpected call".into())))
        }
    }
    fn uci_values() -> Value {
        serde_json::json!({"values":{"wan":{".type":"interface",".name":"wan",".index":2,"proto":"dhcp"},"lan":{".type":"interface",".name":"lan",".index":1,"password":"leak","secret_list":["leak1","leak2"],"dns":["1.1.1.1","8.8.8.8"]}}})
    }
    fn authorization_replies() -> Vec<Result<Value, ObservationError>> {
        let mut replies = READ_PERMISSIONS
            .iter()
            .map(|_| Ok(serde_json::json!({"access":true})))
            .collect::<Vec<_>>();
        replies.extend(
            WRITE_METHODS
                .iter()
                .map(|_| Ok(serde_json::json!({"access":false}))),
        );
        replies
    }
    fn reader(mock: Mock) -> ReadOnlyUbus<Mock> {
        ReadOnlyUbus::with_limits(mock, "session-a", Duration::from_secs(1), 4096).unwrap()
    }
    #[test]
    fn uci_parser_preserves_index_and_redacts_before_serialization() {
        let sections = parse_uci_get("network", &uci_values()).unwrap();
        assert_eq!(
            sections
                .iter()
                .map(|s| s.section.as_str())
                .collect::<Vec<_>>(),
            ["lan", "wan"]
        );
        assert!(sections[0]
            .fields
            .iter()
            .any(|f| matches!(f, ConfigField::Redacted { name } if name == "password")));
        assert!(sections[0]
            .fields
            .iter()
            .any(|f| matches!(f, ConfigField::Redacted { name } if name == "secret_list")));
        assert!(!serde_json::to_string(&sections).unwrap().contains("leak"));
    }
    #[test]
    fn malformed_uci_never_becomes_empty_success() {
        assert!(parse_uci_get(
            "network",
            &serde_json::json!({"values":{"lan":{".type":"interface"}}})
        )
        .is_err());
        assert!(parse_uci_get(
            "network",
            &serde_json::json!({"values":{"lan":{".type":"interface",".index":1,"bad":true}}})
        )
        .is_err());
    }
    #[test]
    fn committed_read_checks_staging_before_and_after() {
        let mut replies = authorization_replies();
        replies.extend([
            Ok(serde_json::json!({"changes":[]})),
            Ok(uci_values()),
            Ok(serde_json::json!({"changes":[]})),
        ]);
        let ubus = reader(Mock::new(replies));
        assert_eq!(ubus.committed_sections("network").unwrap().len(), 2);
        let mut replies = authorization_replies();
        replies.extend([Ok(
            serde_json::json!({"changes":["network.lan.proto=dhcp"]}),
        )]);
        let ubus = reader(Mock::new(replies));
        assert!(matches!(
            ubus.committed_sections("network"),
            Err(ObservationError::Unavailable(_))
        ));
    }
    #[test]
    fn only_fixed_read_methods_are_permitted() {
        assert!(read_only_method("uci", "get"));
        for method in [
            "set", "add", "delete", "commit", "apply", "confirm", "rollback",
        ] {
            assert!(!read_only_method("uci", method));
        }
    }
    #[test]
    fn session_requires_every_read_permission_and_rejects_every_uci_write() {
        let mock = Mock::new(authorization_replies());
        let ubus = reader(mock);
        ubus.verify_read_session().unwrap();
        let calls = ubus.transport.calls.borrow();
        assert_eq!(calls.len(), READ_PERMISSIONS.len() + WRITE_METHODS.len());
        for ((_, method, request), (object, expected_method)) in
            calls[..READ_PERMISSIONS.len()].iter().zip(READ_PERMISSIONS)
        {
            assert_eq!(method, "access");
            assert_eq!(request["object"], object);
            assert_eq!(request["function"], expected_method);
        }
        let denied_read = Mock::new(vec![Ok(serde_json::json!({"access":false}))]);
        assert!(matches!(
            reader(denied_read).verify_read_session(),
            Err(ObservationError::Denied(_))
        ));
        let mut replies = authorization_replies();
        replies[READ_PERMISSIONS.len()] = Ok(serde_json::json!({"access":true}));
        assert!(matches!(
            reader(Mock::new(replies)).verify_read_session(),
            Err(ObservationError::Denied(_))
        ));
    }
    #[test]
    fn netifd_calls_authorize_then_send_session_and_use_assignment_limits() {
        let mut replies = authorization_replies();
        replies.push(Ok(serde_json::json!({"interface":[]})));
        let reader = reader(Mock::new(replies));
        assert!(matches!(
            reader.interfaces().unwrap().completeness,
            Completeness::Complete
        ));
        let calls = reader.transport.calls.borrow();
        let (_, _, request) = calls.last().unwrap();
        assert_eq!(request["ubus_rpc_session"], "session-a");
        let limits = reader.transport.limits.borrow();
        assert!(limits.iter().all(|(_, bytes)| *bytes == 4096));
        assert!(limits.windows(2).all(|pair| pair[1].0 <= pair[0].0));
        assert!(ReadOnlyUbus::with_limits(Mock::new(vec![]), "s", Duration::ZERO, 1).is_err());
        assert!(
            ReadOnlyUbus::with_limits(Mock::new(vec![]), "s", Duration::from_secs(1), 0).is_err()
        );
    }
    #[test]
    fn netifd_parsers_reject_malformed_and_only_complete_typed_data() {
        let interfaces = parse_netifd_interfaces(&serde_json::json!({"interface":[{"interface":"lan","up":true,"device":"br-lan","ipv4-address":[{"address":"192.0.2.1","mask":24}]}]})).unwrap();
        assert!(parse_netifd_interfaces(
            &serde_json::json!({"interface":[{"interface":"lan","up":"yes"}]})
        )
        .is_err());
        assert!(matches!(
            interfaces.completeness,
            Completeness::Partial { .. }
        ));
        let complete = parse_netifd_interfaces(&serde_json::json!({"interface":[{"interface":"wan","up":false,"ipv4-address":[],"ipv6-address":[]}]})).unwrap();
        assert!(matches!(complete.completeness, Completeness::Complete));
        assert!(parse_netifd_devices(
            &serde_json::json!({"br-lan":{"present":true,"bridge-members":["lan1"]}})
        )
        .is_ok());
        assert!(parse_netifd_devices(&serde_json::json!({"devices":[]})).is_err());
        assert!(
            parse_netifd_wireless(&serde_json::json!({"radio0":{"up":true,"pending":false}}))
                .is_ok()
        );
    }
}
