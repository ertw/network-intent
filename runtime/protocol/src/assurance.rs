use crate::{DeviceBinding, RevisionRef, SourceSpan};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Primitive {
    TcpConnect,
    UdpDns,
    TcpDns,
    Http,
    Icmp,
    UciReadback,
    NetifdState,
}

impl Primitive {
    pub fn default_interval_ms(self) -> u64 {
        match self {
            Self::UciReadback => 60_000,
            Self::NetifdState => 15_000,
            _ => 30_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpFamily {
    V4,
    V6,
}

/// Host address plus prefix; a bare address denotes a full-width host prefix.
pub fn parse_interface_address(value: &str) -> Option<(IpAddr, u8)> {
    let mut parts = value.split('/');
    let address: IpAddr = parts.next()?.parse().ok()?;
    let width = if address.is_ipv4() { 32 } else { 128 };
    let prefix = match parts.next() {
        None => width,
        Some(p) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => p.parse::<u8>().ok()?,
        _ => return None,
    };
    if parts.next().is_some() || prefix > width { return None; }
    Some((address, prefix))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSource {
    pub witness_id: String,
    pub location: String,
    pub bind_address: IpAddr,
    pub interface: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub address: IpAddr,
    pub port: Option<u16>,
    pub family: IpFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UciFieldForm {
    Scalar,
    OrderedList,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expectation {
    TcpConnected,
    DnsAnswer {
        name: String,
        record_type: String,
        answers: Vec<String>,
    },
    HttpStatus {
        scheme: String,
        host: String,
        path: String,
        status: u16,
    },
    IcmpEcho,
    UciValue {
        package: String,
        section: String,
        option: String,
        form: UciFieldForm,
        values: Vec<String>,
    },
    UciSection {
        package: String,
        section: String,
        section_type: String,
    },
    NetifdInterface {
        interface: String,
        up: bool,
        addresses: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub timeout_ms: u64,
    pub max_response_bytes: u32,
    pub max_attempts: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSpec {
    pub id: String,
    pub primitive: Primitive,
    pub device: DeviceBinding,
    pub source: ProbeSource,
    pub endpoint: Option<Endpoint>,
    pub expectation: Expectation,
    pub limits: ResourceLimits,
    pub interval_ms: u64,
    pub depends_on: Vec<String>,
    pub claims: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Violation,
    Timeout,
    Refused,
    Unreachable,
    DnsNxDomain,
    DnsServerFailure,
    TlsFailure,
    MalformedResponse,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeResult {
    pub probe_id: String,
    pub outcome: Outcome,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceProfile {
    pub version: u32,
    pub device: DeviceBinding,
    pub firmware: String,
    pub observed_at_ms: u64,
    pub expires_at_ms: u64,
    pub supported_primitives: Vec<Primitive>,
    pub hardware_validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: String,
    pub device_id: String,
    pub kind: String,
    pub source: SourceSpan,
    pub requirements: Vec<ProbeRequirement>,
    pub unsupported_reason: Option<String>,
}

/// Concrete requirements supplied by compiler admission and explicit bindings,
/// never inferred from the untrusted submitted plan itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeRequirement {
    pub primitive: Primitive,
    pub source: ProbeSource,
    pub endpoint: Option<Endpoint>,
    pub expectation: Expectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssurancePlan {
    pub version: u32,
    pub id: String,
    pub epoch: u64,
    pub revision: RevisionRef,
    pub graph_version: u64,
    pub profiles: Vec<DeviceProfile>,
    pub claims: Vec<Claim>,
    pub probes: Vec<ProbeSpec>,
}

#[cfg(test)]
mod address_tests {
    use super::*;
    #[test]
    fn interface_addresses_preserve_host_bits_and_validate_prefix() {
        assert_eq!(parse_interface_address("2001:0db8::1/64"), parse_interface_address("2001:db8:0:0:0:0:0:1/64"));
        assert_ne!(parse_interface_address("192.0.2.1/24"), parse_interface_address("192.0.2.0/24"));
        for invalid in ["192.0.2.1/33", "::1/129", "::1/-1", "::1/64/64", "::1/"] {
            assert_eq!(parse_interface_address(invalid), None);
        }
    }
}
