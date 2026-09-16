//! Bounded, observation-only probe execution.
//!
//! This crate never turns a failed or unavailable observation into success.  It
//! accepts the protocol `ProbeSpec` directly and returns its `ProbeResult`.

pub mod adapters;
pub mod local;
pub mod execution;
pub mod queue;

use hickory_proto::{
    op::{Message, MessageType, OpCode, Query, ResponseCode},
    rr::{DNSClass, Name, RData, RecordType},
    serialize::binary::{BinDecodable, BinEncodable, BinEncoder},
};
use intent_protocol::{
    assurance::{Endpoint, Expectation, IpFamily, Outcome, Primitive, ProbeResult, ProbeSpec},
    check::validate_probe,
};
use rand::random;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io,
    net::{IpAddr, SocketAddr},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpSocket, UdpSocket},
    time::{timeout_at, Instant},
};

const ICMP_PAYLOAD_LEN: usize = 16;

/// Validate the shared wire contract before checking the capabilities of this
/// particular witness build.  Execution must never relax admission rules.
pub fn validate(spec: &ProbeSpec) -> Result<(), String> {
    validate_probe(spec).map_err(|error| error.to_string())?;
    // This build has address binding but no portable SO_BINDTODEVICE equivalent.
    if spec.source.interface.is_some() {
        return Err("interface binding is unsupported on this witness platform; refusing source-address fallback".into());
    }
    if is_ipv4_broadcast(spec.source.bind_address) {
        return Err("source bind address must not be the IPv4 limited broadcast address".into());
    }
    if let Some(endpoint) = &spec.endpoint {
        if is_ipv4_broadcast(endpoint.address) {
            return Err("endpoint must not be the IPv4 limited broadcast address".into());
        }
    }
    if spec.primitive == Primitive::Icmp
        && spec.limits.max_response_bytes < (8 + ICMP_PAYLOAD_LEN) as u32
    {
        return Err("ICMP response limit cannot hold the authenticated echo payload".into());
    }
    Ok(())
}

pub async fn execute(spec: &ProbeSpec) -> ProbeResult {
    let started = now_ms();
    let result = match validate(spec) {
        Err(detail) => (Outcome::Unsupported, detail),
        Ok(()) => {
            let deadline = Instant::now() + duration(spec);
            let mut last = (Outcome::Unavailable, "no attempt made".to_owned());
            for _ in 0..spec.limits.max_attempts {
                if Instant::now() >= deadline {
                    last = (Outcome::Timeout, "probe invocation deadline elapsed".into());
                    break;
                }
                last = execute_once(spec, deadline).await;
                if !matches!(last.0, Outcome::Timeout | Outcome::Unreachable) {
                    break;
                }
            }
            last
        }
    };
    ProbeResult {
        probe_id: spec.id.clone(),
        outcome: result.0,
        started_at_ms: started,
        finished_at_ms: now_ms(),
        detail: result.1,
    }
}

/// The daemon supplies installed local context after signature and scope
/// verification. A remote payload can never select an rpcd session.
pub async fn execute_with_local(
    spec: &ProbeSpec,
    local: &local::LocalObservationContext,
) -> ProbeResult {
    if !matches!(
        spec.primitive,
        Primitive::UciReadback | Primitive::NetifdState
    ) {
        return execute(spec).await;
    }
    let spec = spec.clone();
    let local = local.clone();
    let probe_id = spec.id.clone();
    let started_at_ms = now_ms();
    match tokio::task::spawn_blocking(move || {
        local::execute(&spec, &local, adapters::native_ubus::NativeUbusTransport)
    })
    .await
    {
        Ok(result) => result,
        Err(_) => ProbeResult {
            probe_id,
            outcome: Outcome::Unavailable,
            started_at_ms,
            finished_at_ms: now_ms(),
            detail: "native observation worker failed".into(),
        },
    }
}

async fn execute_once(spec: &ProbeSpec, deadline: Instant) -> (Outcome, String) {
    match spec.primitive {
        Primitive::TcpConnect => tcp_connect(spec, deadline).await,
        Primitive::UdpDns => dns_probe(spec, false, deadline).await,
        Primitive::TcpDns => dns_probe(spec, true, deadline).await,
        Primitive::Http => http_probe(spec, deadline).await,
        Primitive::Icmp => icmp_probe(spec, deadline).await,
        Primitive::UciReadback => (
            Outcome::Unsupported,
            "UCI readback requires installed local observation context".into(),
        ),
        Primitive::NetifdState => (
            Outcome::Unsupported,
            "netifd observation requires installed local observation context".into(),
        ),
    }
}

async fn tcp_connect(spec: &ProbeSpec, deadline: Instant) -> (Outcome, String) {
    let endpoint = spec.endpoint.as_ref().unwrap();
    let addr = socket_addr(endpoint);
    let socket = match tcp_socket(endpoint.family) {
        Ok(socket) => socket,
        Err(e) => return io_outcome(e, "create TCP socket"),
    };
    if let Err(e) = socket.bind(SocketAddr::new(spec.source.bind_address, 0)) {
        return io_outcome(e, "bind TCP source");
    }
    match timeout_at(deadline, socket.connect(addr)).await {
        Err(_) => (Outcome::Timeout, "TCP connect deadline elapsed".into()),
        Ok(Err(e)) => io_outcome(e, "TCP connect"),
        Ok(Ok(_)) => (
            Outcome::Success,
            "TCP connection established from requested source".into(),
        ),
    }
}

async fn dns_probe(spec: &ProbeSpec, tcp: bool, deadline: Instant) -> (Outcome, String) {
    let (name, record_type, wanted) = match &spec.expectation {
        Expectation::DnsAnswer {
            name,
            record_type,
            answers,
        } => match dns_query(name, record_type) {
            Ok(q) => (q.0, q.1, answers),
            Err(e) => return (Outcome::Unsupported, e),
        },
        _ => unreachable!(),
    };
    let id = random::<u16>();
    let query = match make_dns_query(id, name.clone(), record_type) {
        Ok(q) => q,
        Err(e) => return (Outcome::MalformedResponse, e),
    };
    let response = if tcp {
        dns_tcp_io(spec, &query, deadline).await
    } else {
        dns_udp_io(spec, &query, deadline).await
    };
    let bytes = match response {
        Ok(bytes) => bytes,
        Err(v) => return v,
    };
    let message = match Message::from_bytes(&bytes) {
        Ok(m) => m,
        Err(e) => {
            return (
                Outcome::MalformedResponse,
                format!("DNS decode failed: {e}"),
            )
        }
    };
    if message.id() != id
        || message.message_type() != MessageType::Response
        || message.op_code() != OpCode::Query
        || message.queries().len() != 1
        || message.queries()[0] != Query::query(name.clone(), record_type)
    {
        return (
            Outcome::MalformedResponse,
            "DNS response does not exactly match request".into(),
        );
    }
    if message.truncated() {
        return (
            Outcome::MalformedResponse,
            "DNS response is truncated; refusing incomplete answer set".into(),
        );
    }
    match message.response_code() {
        ResponseCode::NXDomain => {
            return (Outcome::DnsNxDomain, "DNS server returned NXDOMAIN".into())
        }
        ResponseCode::ServFail => {
            return (
                Outcome::DnsServerFailure,
                "DNS server returned SERVFAIL".into(),
            )
        }
        ResponseCode::NoError => {}
        code => {
            return (
                Outcome::DnsServerFailure,
                format!("DNS server response code: {code}"),
            )
        }
    }
    let actual: Vec<String> = message
        .answers()
        .iter()
        .filter(|record| {
            record.name() == &name
                && record.record_type() == record_type
                && record.dns_class() == DNSClass::IN
        })
        .map(|r| render_rdata(r.data()))
        .collect();
    if wanted
        .iter()
        .all(|answer| actual.iter().any(|actual| actual == answer))
    {
        (
            Outcome::Success,
            "DNS response matched requested answers".into(),
        )
    } else {
        (
            Outcome::Violation,
            format!("DNS answers did not include expected values; got {actual:?}"),
        )
    }
}

async fn dns_udp_io(
    spec: &ProbeSpec,
    query: &[u8],
    deadline: Instant,
) -> Result<Vec<u8>, (Outcome, String)> {
    let endpoint = spec.endpoint.as_ref().unwrap();
    let bind = SocketAddr::new(spec.source.bind_address, 0);
    let socket = UdpSocket::bind(bind)
        .await
        .map_err(|e| io_outcome(e, "bind DNS UDP source"))?;
    timeout_at(deadline, socket.send_to(query, socket_addr(endpoint)))
        .await
        .map_err(|_| (Outcome::Timeout, "DNS UDP send deadline elapsed".into()))?
        .map_err(|e| io_outcome(e, "send DNS UDP query"))?;
    // UDP payloads are bounded by the transport.  A full buffer is rejected
    // below because Tokio's recv_from API does not expose MSG_TRUNC.
    let max_datagram = 65_507usize;
    let mut buf = vec![0_u8; (spec.limits.max_response_bytes as usize).min(max_datagram)];
    let received = timeout_at(deadline, socket.recv_from(&mut buf))
        .await
        .map_err(|_| (Outcome::Timeout, "DNS UDP response deadline elapsed".into()))?
        .map_err(|e| io_outcome(e, "receive DNS UDP response"))?;
    if received.1.ip() != endpoint.address || received.1.port() != endpoint.port.unwrap() {
        return Err((
            Outcome::MalformedResponse,
            "DNS UDP response arrived from a different endpoint".into(),
        ));
    }
    if received.0 == buf.len() {
        return Err((
            Outcome::MalformedResponse,
            "DNS UDP datagram reached the response limit and may be truncated".into(),
        ));
    }
    buf.truncate(received.0);
    Ok(buf)
}

async fn dns_tcp_io(
    spec: &ProbeSpec,
    query: &[u8],
    deadline: Instant,
) -> Result<Vec<u8>, (Outcome, String)> {
    let endpoint = spec.endpoint.as_ref().unwrap();
    let socket = tcp_socket(endpoint.family).map_err(|e| io_outcome(e, "create DNS TCP socket"))?;
    socket
        .bind(SocketAddr::new(spec.source.bind_address, 0))
        .map_err(|e| io_outcome(e, "bind DNS TCP source"))?;
    let mut stream = timeout_at(deadline, socket.connect(socket_addr(endpoint)))
        .await
        .map_err(|_| (Outcome::Timeout, "DNS TCP connect deadline elapsed".into()))?
        .map_err(|e| io_outcome(e, "connect DNS TCP"))?;
    let length = u16::try_from(query.len()).map_err(|_| {
        (
            Outcome::MalformedResponse,
            "DNS query too large for TCP framing".into(),
        )
    })?;
    timeout_at(deadline, async {
        stream.write_all(&length.to_be_bytes()).await?;
        stream.write_all(query).await
    })
    .await
    .map_err(|_| (Outcome::Timeout, "DNS TCP write deadline elapsed".into()))?
    .map_err(|e| io_outcome(e, "write DNS TCP query"))?;
    let mut prefix = [0_u8; 2];
    timeout_at(deadline, stream.read_exact(&mut prefix))
        .await
        .map_err(|_| (Outcome::Timeout, "DNS TCP frame deadline elapsed".into()))?
        .map_err(|e| io_outcome(e, "read DNS TCP frame"))?;
    let size = u16::from_be_bytes(prefix) as usize;
    if size == 0 || size > spec.limits.max_response_bytes as usize {
        return Err((
            Outcome::MalformedResponse,
            "DNS TCP frame exceeds response limit".into(),
        ));
    }
    let mut bytes = vec![0; size];
    timeout_at(deadline, stream.read_exact(&mut bytes))
        .await
        .map_err(|_| (Outcome::Timeout, "DNS TCP payload deadline elapsed".into()))?
        .map_err(|e| io_outcome(e, "read DNS TCP payload"))?;
    Ok(bytes)
}

async fn http_probe(spec: &ProbeSpec, deadline: Instant) -> (Outcome, String) {
    let (scheme, host, path, status) = match &spec.expectation {
        Expectation::HttpStatus {
            scheme,
            host,
            path,
            status,
        } => (scheme, host, path, *status),
        _ => unreachable!(),
    };
    let endpoint = spec.endpoint.as_ref().unwrap();
    let authority = match host.parse::<IpAddr>() {
        Ok(ip) if ip != endpoint.address => {
            return (
                Outcome::Unsupported,
                "HTTP IP host must equal the concrete endpoint".into(),
            )
        }
        Ok(IpAddr::V6(_)) => format!("[{host}]"),
        Ok(IpAddr::V4(_)) => host.to_owned(),
        Err(_) => host.to_owned(),
    };
    let url = format!("{scheme}://{authority}:{}{}", endpoint.port.unwrap(), path);
    let client = match reqwest::Client::builder()
        .local_address(spec.source.bind_address)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .http2_max_header_list_size(spec.limits.max_response_bytes)
        .resolve(host, socket_addr(endpoint))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                Outcome::Unsupported,
                format!("build source-bound HTTP client: {e}"),
            )
        }
    };
    let response = match timeout_at(deadline, client.get(url).send()).await {
        Err(_) => return (Outcome::Timeout, "HTTP invocation deadline elapsed".into()),
        Ok(Err(error)) => return http_error_outcome(error, scheme),
        Ok(Ok(response)) => response,
    };
    let header_bytes = response
        .headers()
        .iter()
        .fold(2usize, |total, (name, value)| {
            total.saturating_add(name.as_str().len() + value.as_bytes().len() + 4)
        });
    if header_bytes > spec.limits.max_response_bytes as usize {
        return (
            Outcome::MalformedResponse,
            "HTTP response headers exceed response-byte limit".into(),
        );
    }
    let body_limit = spec.limits.max_response_bytes as usize - header_bytes;
    if response
        .content_length()
        .is_some_and(|n| n > body_limit as u64)
    {
        return (
            Outcome::MalformedResponse,
            "HTTP response body exceeds response-byte limit".into(),
        );
    }
    let observed_status = response.status();
    let mut body = response;
    let mut body_bytes = 0usize;
    loop {
        let chunk = match timeout_at(deadline, body.chunk()).await {
            Err(_) => {
                return (
                    Outcome::Timeout,
                    "HTTP invocation deadline elapsed while reading response".into(),
                )
            }
            Ok(Err(error)) => return http_error_outcome(error, scheme),
            Ok(Ok(None)) => break,
            Ok(Ok(Some(chunk))) => chunk,
        };
        body_bytes = body_bytes.saturating_add(chunk.len());
        if body_bytes > body_limit {
            return (
                Outcome::MalformedResponse,
                "HTTP response body exceeds response-byte limit".into(),
            );
        }
    }
    if observed_status.as_u16() == status {
        (
            Outcome::Success,
            format!("HTTP status {status} from requested endpoint"),
        )
    } else {
        (
            Outcome::Violation,
            format!("HTTP status {observed_status} did not equal {status}"),
        )
    }
}

fn http_error_outcome(error: reqwest::Error, scheme: &str) -> (Outcome, String) {
    if error.is_timeout() {
        return (Outcome::Timeout, format!("HTTP deadline elapsed: {error}"));
    }
    // reqwest exposes transport-connect errors but not a stable TLS error type.
    // Restrict TLS classification to an explicit TLS/certificate handshake cause;
    // connection refusal, routing and DNS failures remain transport failures.
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if scheme == "https"
        && ["tls", "ssl", "certificate", "handshake"]
            .iter()
            .any(|needle| lower.contains(needle))
    {
        return (Outcome::TlsFailure, format!("HTTPS TLS failed: {detail}"));
    }
    if error.is_builder() || error.is_request() || error.is_decode() {
        (
            Outcome::MalformedResponse,
            format!("HTTP protocol failure: {detail}"),
        )
    } else {
        (
            Outcome::Unreachable,
            format!("HTTP transport failed: {detail}"),
        )
    }
}

async fn icmp_probe(spec: &ProbeSpec, deadline: Instant) -> (Outcome, String) {
    let spec = spec.clone();
    match timeout_at(
        deadline,
        tokio::task::spawn_blocking(move || icmp_blocking(&spec, deadline)),
    )
    .await
    {
        Err(_) => (Outcome::Timeout, "ICMP invocation deadline elapsed".into()),
        Ok(Ok(v)) => v,
        Ok(Err(e)) => (Outcome::Unavailable, format!("ICMP task unavailable: {e}")),
    }
}

#[cfg(unix)]
fn icmp_blocking(spec: &ProbeSpec, deadline: Instant) -> (Outcome, String) {
    let endpoint = spec.endpoint.as_ref().unwrap();
    let (domain, protocol, request_type, response_type) = match endpoint.family {
        IpFamily::V4 => (Domain::IPV4, Protocol::ICMPV4, 8u8, 0u8),
        IpFamily::V6 => (Domain::IPV6, Protocol::ICMPV6, 128u8, 129u8),
    };
    let socket = match Socket::new(domain, Type::RAW, Some(protocol)) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            return (
                Outcome::Unavailable,
                "native ICMP requires raw-socket permission".into(),
            )
        }
        Err(e) => return io_outcome(e, "create native ICMP socket"),
    };
    if let Err(e) = socket.bind(&SocketAddr::new(spec.source.bind_address, 0).into()) {
        return io_outcome(e, "bind ICMP source");
    }
    let id = random::<u16>();
    let seq = random::<u16>();
    let payload = random::<[u8; ICMP_PAYLOAD_LEN]>();
    let mut packet = vec![
        request_type,
        0,
        0,
        0,
        (id >> 8) as u8,
        id as u8,
        (seq >> 8) as u8,
        seq as u8,
    ];
    packet.extend_from_slice(&payload);
    if endpoint.family == IpFamily::V4 {
        let sum = checksum(&packet);
        packet[2] = (sum >> 8) as u8;
        packet[3] = sum as u8;
    }
    let remaining = match deadline.checked_duration_since(Instant::now()) {
        Some(duration) if !duration.is_zero() => duration,
        _ => return (Outcome::Timeout, "ICMP invocation deadline elapsed".into()),
    };
    if let Err(e) = socket.set_read_timeout(Some(remaining)) {
        return io_outcome(e, "set ICMP deadline");
    }
    if let Err(e) = socket.send_to(&packet, &socket_addr(endpoint).into()) {
        return io_outcome(e, "send ICMP echo");
    }
    let mut buf =
        vec![std::mem::MaybeUninit::<u8>::uninit(); spec.limits.max_response_bytes as usize];
    match socket.recv_from(&mut buf) {
        Ok((n, peer)) => {
            // `recv_from` initializes exactly the returned bytes.
            let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };
            validate_icmp_reply(
                endpoint.family,
                endpoint.address,
                peer.as_socket().map(|p| p.ip()),
                bytes,
                response_type,
                id,
                seq,
                &payload,
            )
        }
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            (Outcome::Timeout, "ICMP echo deadline elapsed".into())
        }
        Err(e) => io_outcome(e, "receive ICMP echo"),
    }
}
#[cfg(not(unix))]
fn icmp_blocking(_: &ProbeSpec, _: Instant) -> (Outcome, String) {
    (
        Outcome::Unavailable,
        "native ICMP is unsupported on this platform".into(),
    )
}

fn validate_icmp_reply(
    family: IpFamily,
    expected_peer: IpAddr,
    peer: Option<IpAddr>,
    bytes: &[u8],
    response_type: u8,
    id: u16,
    seq: u16,
    payload: &[u8],
) -> (Outcome, String) {
    let offset = if family == IpFamily::V4 && bytes.first().is_some_and(|first| first >> 4 == 4) {
        let header_len = ((bytes[0] & 0x0f) as usize) * 4;
        if header_len < 20 || header_len > bytes.len() {
            return (
                Outcome::MalformedResponse,
                "ICMP reply has an invalid IPv4 header length".into(),
            );
        }
        header_len
    } else {
        0
    };
    if peer != Some(expected_peer)
        || bytes.len() != offset + 8 + payload.len()
        || bytes.get(offset) != Some(&response_type)
        || bytes.get(offset + 1) != Some(&0)
        || bytes.get(offset + 4..offset + 6) != Some(&id.to_be_bytes())
        || bytes.get(offset + 6..offset + 8) != Some(&seq.to_be_bytes())
        || bytes.get(offset + 8..) != Some(payload)
    {
        (
            Outcome::MalformedResponse,
            "ICMP reply did not exactly match request".into(),
        )
    } else {
        (
            Outcome::Success,
            "native ICMP echo reply exactly matched request".into(),
        )
    }
}

fn make_dns_query(id: u16, name: Name, record_type: RecordType) -> Result<Vec<u8>, String> {
    let mut message = Message::new();
    message
        .set_id(id)
        .set_message_type(MessageType::Query)
        .set_op_code(OpCode::Query)
        .set_recursion_desired(false)
        .add_query(Query::query(name, record_type));
    let mut bytes = Vec::new();
    message
        .emit(&mut BinEncoder::new(&mut bytes))
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}
fn dns_query(name: &str, kind: &str) -> Result<(Name, RecordType), String> {
    let name = Name::from_ascii(name).map_err(|e| format!("invalid DNS name: {e}"))?;
    let record_type = kind
        .parse::<RecordType>()
        .map_err(|_| format!("unsupported DNS record type: {kind}"))?;
    Ok((name, record_type))
}
fn render_rdata(data: &RData) -> String {
    data.to_string()
}
fn is_ipv4_broadcast(address: IpAddr) -> bool {
    matches!(address, IpAddr::V4(v4) if v4.octets() == [255, 255, 255, 255])
}
fn socket_addr(endpoint: &Endpoint) -> SocketAddr {
    SocketAddr::new(endpoint.address, endpoint.port.unwrap_or(0))
}
fn tcp_socket(family: IpFamily) -> io::Result<TcpSocket> {
    match family {
        IpFamily::V4 => TcpSocket::new_v4(),
        IpFamily::V6 => TcpSocket::new_v6(),
    }
}
fn duration(spec: &ProbeSpec) -> Duration {
    Duration::from_millis(spec.limits.timeout_ms)
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn io_outcome(error: io::Error, action: &str) -> (Outcome, String) {
    let outcome = match error.kind() {
        io::ErrorKind::ConnectionRefused => Outcome::Refused,
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Outcome::Timeout,
        io::ErrorKind::AddrNotAvailable
        | io::ErrorKind::NetworkUnreachable
        | io::ErrorKind::HostUnreachable
        | io::ErrorKind::NotConnected => Outcome::Unreachable,
        io::ErrorKind::PermissionDenied => Outcome::Unavailable,
        _ => Outcome::Unreachable,
    };
    (outcome, format!("{action}: {error}"))
}
fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    for pair in bytes.chunks(2) {
        sum += u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)]) as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::rr::{rdata::A, Record};
    use intent_protocol::{
        assurance::{ProbeSource, ResourceLimits},
        DeviceBinding,
    };
    use tokio::net::TcpListener;

    fn spec(
        primitive: Primitive,
        endpoint: Option<SocketAddr>,
        expectation: Expectation,
    ) -> ProbeSpec {
        ProbeSpec {
            id: "test".into(),
            primitive,
            device: DeviceBinding {
                device_id: "d".into(),
                profile_digest: "p".into(),
                fencing_generation: 1,
            },
            source: ProbeSource {
                witness_id: "w".into(),
                location: "local".into(),
                bind_address: "127.0.0.1".parse().unwrap(),
                interface: None,
            },
            endpoint: endpoint.map(|address| Endpoint {
                address: address.ip(),
                port: Some(address.port()),
                family: IpFamily::V4,
            }),
            expectation,
            limits: ResourceLimits {
                timeout_ms: 100,
                max_response_bytes: 4096,
                max_attempts: 1,
            },
            interval_ms: 100,
            depends_on: vec![],
            claims: vec![],
        }
    }

    fn dns_expectation() -> Expectation {
        Expectation::DnsAnswer {
            name: "example.test.".into(),
            record_type: "A".into(),
            answers: vec!["127.0.0.1".into()],
        }
    }
    #[tokio::test]
    async fn tcp_success_and_refusal_are_distinct() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let accept = tokio::spawn(async move {
            let _ = listener.accept().await;
        });
        assert_eq!(
            execute(&spec(
                Primitive::TcpConnect,
                Some(address),
                Expectation::TcpConnected
            ))
            .await
            .outcome,
            Outcome::Success
        );
        accept.await.unwrap();
        let unused = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        assert_eq!(
            execute(&spec(
                Primitive::TcpConnect,
                Some(unused),
                Expectation::TcpConnected
            ))
            .await
            .outcome,
            Outcome::Refused
        );
    }
    #[tokio::test]
    async fn http_status_and_no_redirect_following() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut b = [0; 512];
            let _ = s.read(&mut b).await;
            s.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://example.invalid/\r\nContent-Length: 0\r\n\r\n").await.unwrap();
        });
        let result = execute(&spec(
            Primitive::Http,
            Some(address),
            Expectation::HttpStatus {
                scheme: "http".into(),
                host: "probe.test".into(),
                path: "/".into(),
                status: 200,
            },
        ))
        .await;
        assert_eq!(result.outcome, Outcome::Violation);
        server.await.unwrap();
    }
    #[tokio::test]
    async fn malformed_and_timeout_udp_dns_are_not_success() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let malformed = tokio::spawn(async move {
            let mut b = [0; 512];
            let (_, peer) = socket.recv_from(&mut b).await.unwrap();
            socket.send_to(b"bad", peer).await.unwrap();
        });
        let expectation = dns_expectation();
        assert_eq!(
            execute(&spec(Primitive::UdpDns, Some(address), expectation))
                .await
                .outcome,
            Outcome::MalformedResponse
        );
        malformed.await.unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let sink = tokio::spawn(async move {
            let mut b = [0; 512];
            let _ = socket.recv_from(&mut b).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        });
        let expectation = dns_expectation();
        assert_eq!(
            execute(&spec(Primitive::UdpDns, Some(address), expectation))
                .await
                .outcome,
            Outcome::Timeout
        );
        sink.await.unwrap();
    }
    #[tokio::test]
    async fn udp_dns_success_and_nxdomain_are_distinct() {
        for (code, expected) in [
            (ResponseCode::NoError, Outcome::Success),
            (ResponseCode::NXDomain, Outcome::DnsNxDomain),
        ] {
            let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let address = socket.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let mut b = [0; 512];
                let (n, peer) = socket.recv_from(&mut b).await.unwrap();
                let request = Message::from_bytes(&b[..n]).unwrap();
                let mut response = Message::new();
                response
                    .set_id(request.id())
                    .set_message_type(MessageType::Response)
                    .set_op_code(OpCode::Query)
                    .set_response_code(code)
                    .add_query(request.queries()[0].clone());
                if code == ResponseCode::NoError {
                    response.add_answer(Record::from_rdata(
                        request.queries()[0].name().clone(),
                        30,
                        RData::A(A::new(127, 0, 0, 1)),
                    ));
                }
                let mut bytes = Vec::new();
                response.emit(&mut BinEncoder::new(&mut bytes)).unwrap();
                socket.send_to(&bytes, peer).await.unwrap();
            });
            let expectation = dns_expectation();
            assert_eq!(
                execute(&spec(Primitive::UdpDns, Some(address), expectation))
                    .await
                    .outcome,
                expected
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn tcp_dns_success_frame_limit_malformed_and_timeout_are_distinct() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_tcp_dns_request(&mut stream).await;
            let bytes = dns_response(&request, ResponseCode::NoError, false, true);
            stream
                .write_all(&(bytes.len() as u16).to_be_bytes())
                .await
                .unwrap();
            stream.write_all(&bytes).await.unwrap();
        });
        assert_eq!(
            execute(&spec(Primitive::TcpDns, Some(address), dns_expectation()))
                .await
                .outcome,
            Outcome::Success
        );
        server.await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_tcp_dns_request(&mut stream).await;
            stream.write_all(&9u16.to_be_bytes()).await.unwrap();
        });
        let mut limited = spec(Primitive::TcpDns, Some(address), dns_expectation());
        limited.limits.max_response_bytes = 8;
        assert_eq!(execute(&limited).await.outcome, Outcome::MalformedResponse);
        server.await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_tcp_dns_request(&mut stream).await;
            stream.write_all(&0u16.to_be_bytes()).await.unwrap();
        });
        assert_eq!(
            execute(&spec(Primitive::TcpDns, Some(address), dns_expectation()))
                .await
                .outcome,
            Outcome::MalformedResponse
        );
        server.await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_tcp_dns_request(&mut stream).await;
            tokio::time::sleep(Duration::from_millis(150)).await;
        });
        assert_eq!(
            execute(&spec(Primitive::TcpDns, Some(address), dns_expectation()))
                .await
                .outcome,
            Outcome::Timeout
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn dns_rejects_truncated_or_unrelated_answers_and_empty_expectations() {
        let listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut packet = [0; 512];
            let (size, peer) = listener.recv_from(&mut packet).await.unwrap();
            let request = Message::from_bytes(&packet[..size]).unwrap();
            let bytes = dns_response(&request, ResponseCode::NoError, true, true);
            listener.send_to(&bytes, peer).await.unwrap();
        });
        assert_eq!(
            execute(&spec(Primitive::UdpDns, Some(address), dns_expectation()))
                .await
                .outcome,
            Outcome::MalformedResponse
        );
        server.await.unwrap();

        let listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut packet = [0; 512];
            let (size, peer) = listener.recv_from(&mut packet).await.unwrap();
            let request = Message::from_bytes(&packet[..size]).unwrap();
            let bytes = dns_response(&request, ResponseCode::NoError, false, false);
            listener.send_to(&bytes, peer).await.unwrap();
        });
        assert_eq!(
            execute(&spec(Primitive::UdpDns, Some(address), dns_expectation()))
                .await
                .outcome,
            Outcome::Violation
        );
        server.await.unwrap();

        let mut empty = spec(Primitive::UdpDns, Some(address), dns_expectation());
        if let Expectation::DnsAnswer { answers, .. } = &mut empty.expectation {
            answers.clear();
        }
        assert!(validate(&empty).is_err());
    }

    #[tokio::test]
    async fn http_enforces_body_limit_and_concrete_ip_host() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, peer) = listener.accept().await.unwrap();
            assert_eq!(peer.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
            let mut request = [0; 512];
            let _ = stream.read(&mut request).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n0123456789")
                .await
                .unwrap();
        });
        let mut limited = spec(
            Primitive::Http,
            Some(address),
            Expectation::HttpStatus {
                scheme: "http".into(),
                host: "probe.test".into(),
                path: "/".into(),
                status: 200,
            },
        );
        limited.limits.max_response_bytes = 4;
        assert_eq!(execute(&limited).await.outcome, Outcome::MalformedResponse);
        server.await.unwrap();

        let mut mismatch = spec(
            Primitive::Http,
            Some(address),
            Expectation::HttpStatus {
                scheme: "http".into(),
                host: "127.0.0.2".into(),
                path: "/".into(),
                status: 200,
            },
        );
        assert_eq!(execute(&mismatch).await.outcome, Outcome::Unsupported);
        mismatch.source.bind_address = "255.255.255.255".parse().unwrap();
        assert!(validate(&mismatch).is_err());
    }

    #[test]
    fn icmp_reply_validator_rejects_corruption() {
        let id = 0x1234;
        let sequence = 0x5678;
        let payload = [9, 8, 7];
        let mut packet = vec![0, 0, 0, 0, 0x12, 0x34, 0x56, 0x78];
        packet.extend_from_slice(&payload);
        assert_eq!(
            validate_icmp_reply(
                IpFamily::V4,
                "127.0.0.1".parse().unwrap(),
                Some("127.0.0.1".parse().unwrap()),
                &packet,
                0,
                id,
                sequence,
                &payload
            )
            .0,
            Outcome::Success
        );
        packet[7] ^= 1;
        assert_eq!(
            validate_icmp_reply(
                IpFamily::V4,
                "127.0.0.1".parse().unwrap(),
                Some("127.0.0.1".parse().unwrap()),
                &packet,
                0,
                id,
                sequence,
                &payload
            )
            .0,
            Outcome::MalformedResponse
        );
        let invalid_ipv4 = [0x41; 20];
        assert_eq!(
            validate_icmp_reply(
                IpFamily::V4,
                "127.0.0.1".parse().unwrap(),
                Some("127.0.0.1".parse().unwrap()),
                &invalid_ipv4,
                0,
                id,
                sequence,
                &payload
            )
            .0,
            Outcome::MalformedResponse
        );
    }

    async fn read_tcp_dns_request(stream: &mut tokio::net::TcpStream) -> Message {
        let mut prefix = [0; 2];
        stream.read_exact(&mut prefix).await.unwrap();
        let mut bytes = vec![0; u16::from_be_bytes(prefix) as usize];
        stream.read_exact(&mut bytes).await.unwrap();
        Message::from_bytes(&bytes).unwrap()
    }

    fn dns_response(
        request: &Message,
        code: ResponseCode,
        truncated: bool,
        requested_answer: bool,
    ) -> Vec<u8> {
        let mut response = Message::new();
        response
            .set_id(request.id())
            .set_message_type(MessageType::Response)
            .set_op_code(OpCode::Query)
            .set_response_code(code)
            .set_truncated(truncated)
            .add_query(request.queries()[0].clone());
        if requested_answer {
            response.add_answer(Record::from_rdata(
                request.queries()[0].name().clone(),
                30,
                RData::A(A::new(127, 0, 0, 1)),
            ));
        } else {
            response.add_answer(Record::from_rdata(
                Name::from_ascii("other.test.").unwrap(),
                30,
                RData::A(A::new(127, 0, 0, 1)),
            ));
        }
        let mut bytes = Vec::new();
        response.emit(&mut BinEncoder::new(&mut bytes)).unwrap();
        bytes
    }
    #[test]
    fn rejects_missing_required_network_binding_before_effect() {
        let mut s = spec(Primitive::TcpConnect, None, Expectation::TcpConnected);
        assert!(validate(&s).is_err());
        s.limits.max_attempts = 0;
        assert!(validate(&s).is_err());
        s.limits.max_attempts = 1;
        s.endpoint = Some(Endpoint {
            address: "127.0.0.1".parse().unwrap(),
            port: Some(9),
            family: IpFamily::V4,
        });
        s.source.interface = Some("lo".into());
        assert!(validate(&s).is_err());
        s.source.interface = None;
        s.source.bind_address = "0.0.0.0".parse().unwrap();
        assert!(validate(&s).is_err());
        s.source.bind_address = "127.0.0.1".parse().unwrap();
        s.limits.max_attempts = 4;
        assert!(validate(&s).is_err());
    }
}
