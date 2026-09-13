# Network Intent language 1.0

Every construct below was introduced in 1.0. Unknown versions and constructs
are errors. The authoritative grammar and meaning are implemented in Idris;
there is no separate editor parser in this release.

## Lexical grammar and document

```text
network-language 1.0
network NAME {
  domain example.home.arpa
  ... declarations ...
}
```

One source has exactly one header and network. Scalar statements end at a
newline or closing brace. Blocks use `{` and `}`; single-field inline blocks are
supported. Semicolons are not statement delimiters. `#` and `//` introduce
line comments at token boundaries. Strings use double quotes and support
`\"`, `\\`, `\n`, `\r`, `\t` escapes. Control characters in descriptions are
rejected before target rendering. Formatting preserves comments/string values,
uses two-space indentation and ends with one newline.

Names use 1–63 ASCII letters, digits, hyphens or underscores. Names are unique
within typed namespaces: a host and VLAN may share a name. Device and host names
cannot collide because `connect name.eth0` would be ambiguous. VLAN names
`internet` and `gateway` are reserved. Port names are unique within their device
and permit ASCII alphanumerics, `/`, `.`, `-`, `_`; backends impose tighter limits.
Domain names are optional, at most 253 ASCII alphanumeric/hyphen/dot characters.

Input is limited to 1,048,576 Unicode characters, 32,768 tokens and 64 nested
blocks. Decimal tokens are limited to 20 digits before numeric conversion.
Malformed syntax produces `syntax.*`, invalid names `name.*`, and missing or
unsupported versions `version.*` diagnostics. There is no include mechanism,
embedded code, macro language, template execution or implicit version upgrade.

## VLANs and IPv4 addressing

```text
vlan servers 20 {
  subnet 10.0.20.0/23
  gateway +1
  dhcp +100 .. +199
  host nas +300
}
```

`subnet` is required exactly once. VLAN IDs range from 1 to 4094 and are unique.
IPv4 octets range from 0 to 255; prefix widths from 0 to 32. Prefixes must be
canonical (host bits zero). VLAN prefixes cannot overlap in the single v1
routing context. VRF and overlapping-prefix exceptions are not supported.

Gateway and DHCP are optional. A static host declaration gives one IPv4 address
and one implicit `eth0` interface; the user configures that IP on the host.
Without MAC information the compiler creates DNS records, not invented DHCP
reservations. `+N` means network-address plus N: `+300` above is `10.0.21.44`.
An absolute dotted IPv4 address can be used wherever a host address is expected.

Assignments must belong to the prefix. For `/0` through `/30`, network and
broadcast addresses cannot be assigned; `/31` follows point-to-point host
allocation and `/32` has a single host address. DHCP requires `/30` or larger
host space, ordered in-prefix endpoints and a gateway. Static addresses and
gateways must be unique and outside DHCP pools. `address.*` diagnostics identify
the failed property and related collision/prefix declarations.

Gateway/DHCP intent requires one router carrying every gateway VLAN. Multiple
routers are rejected in 1.0 instead of selecting an owner implicitly. OpenWrt
owns routed addresses and DHCP; Cisco's L2 profile owns only switching.

## Devices, ports and physical links

```text
router gateway {
  driver openwrt
  max-tagged-vlans 100
  port lan1 { access servers }
  port lan4 { trunk trusted servers iot }
}
switch core {
  driver cisco-ios
  port Gi1/0/1 {
    description "Gateway uplink"
    trunk trusted servers iot
    connect gateway.lan4
  }
  port Gi1/0/2 {
    access servers
    connect nas.eth0
  }
}
```

`device` is also accepted and has the same L2 role as `switch`; `router` is the
single routing/enforcement role. `driver` is required: `openwrt` or `cisco-ios`.
An unsupported driver fails. `max-tagged-vlans` is an optional assumed target
budget in 0–4094 (default 4094). `compile --max-tagged-vlans N` applies a lower
test/profile limit. A mismatch fails with the device/port and source span.

Every port has exactly one access or trunk declaration. Access is one untagged
VLAN; a trunk carries a nonempty, duplicate-free list of tagged VLANs, with no
native/untagged payload VLAN. Connected access ports must name the same VLAN;
connected trunks must have exactly the same VLAN set. Access/trunk mismatches
are errors. A host's implicit `eth0` must connect to an access port in its VLAN.

Connections resolve fully, reject self-links and multiple peers, and may be
declared on either or both endpoints. Reciprocal declarations produce one
undirected graph edge. Physical cycles are allowed. Connectivity, carrier state,
STP convergence and cable presence are not observations made by this compiler.

Errors use `topology.*`, `reference.*`, and `backend.capability-mismatch`.
Descriptions are optional single-line values; target limits and quoting apply.
Bridge/LAG/logical-role source declarations are later milestones; the OpenWrt
compiler derives one internal DSA bridge from physical port memberships.

## Services and policy

```text
service dashboard {
  tcp 8123
  depends dns
}
policy {
  trusted -> internet allow
  trusted -> servers allow dashboard, ssh
  iot -> gateway allow dns, ntp
  iot -> trusted deny
}
```

Services declare one or more TCP/UDP ports in 1–65535. Repeated transport
declarations are permitted; duplicate transport/port pairs normalize once.
Built-ins are `dns` (UDP and TCP 53), `ntp` (UDP 123), `http` (TCP 80), `https`
(TCP 443), and `ssh` (TCP 22); these names cannot be redefined. `depends` names
one or more services. Dependencies are declaration prerequisites and must form
a DAG; they do not imply operational availability or open firewall paths.

Policy source must be a VLAN. Destination is another VLAN, `gateway` (local
traffic to the routing owner), or `internet` (forwarded traffic to external WAN).
Both VLANs must have gateways and be carried by the router. Same-VLAN rules are
rejected because routed policy cannot govern arbitrary bridged traffic.

The exact default profile is:

1. IPv4 connection initiation, evaluated in declaration order; a matching rule
   accepts or drops new traffic. A supplied service list is comma-separated,
   nonempty and fully resolved. Omitted services means all IPv4 protocols.
2. Existing/related return traffic is accepted by firewall4's stateful profile.
   New local-input and forwarded traffic default to drop. Router-originated
   output is accepted. Policy changes do not revoke already established flows.
3. Gateway rules have a source zone and no destination zone. Inter-VLAN and
   internet rules have both source and destination zones.
4. Declaring DHCP also declares its required gateway control traffic: UDP 67
   plus UDP/TCP 53 for gateway DNS advertised to clients. These permissions are
   emitted before general rules. A gateway denial overlapping them is rejected
   as `policy.dhcp-control-conflict`, never silently overridden.
5. Addresses refer to untranslated identities. No SNAT/DNAT/masquerading or
   port-forwarding transformation is inferred. Internet requires an external
   `wan` logical interface and an upstream route, recorded as target assumptions.
6. The profile does not claim IPv6, multicast discovery, interface-control
   protocols or service health. IPv6 must be disabled or separately governed.

The Cisco L2 target cannot enforce this stateful profile. It receives the L2
projection of the same certified model; the declared OpenWrt router owns the
policy. Selecting a Cisco router as enforcer fails capability analysis.

## Static routes and dependency certification

```text
route upstream {
  destination 192.0.2.0/24
  via 10.0.20.254
  vlan servers
  device gateway
  metric 10
  depends bootstrap
}
```

Destination, next hop, VLAN and device are required. Metric defaults to zero
and must fit in an unsigned 32-bit integer. The next hop must be a usable on-link
address in the selected VLAN and cannot equal the router's own gateway address.
The device must be a router carrying that VLAN. `depends` is optional and names
other routes as installation prerequisites. Dependencies are certified acyclic.

This first release deliberately rejects off-link recursive next hops; dependency
graphs do not turn an unreachable next hop into a reachable one. Dynamic routing,
VRFs and protocol convergence are later milestones. Errors use `routing.*` and
`reference.*`; cycle errors contain a closed directed witness with declaration
spans. OpenWrt emits static route sections; the Cisco L2 profile cannot own routes.

## Output, errors and versions

`check --format json` has `ok`, `diagnostics`, assurance labels and a semantic
snapshot. Diagnostics contain stable codes, severity, primary span, title,
detail, related spans and a fixes list (currently empty). Semantic validation
accumulates independent model errors; parsing/resolution report the first error.
Offsets/columns count Unicode characters. No error-recovery editor parser is
claimed. Nonzero exit status means failure; compilation emits no partial target
output when any requested target fails.

`docs` and `graph` derive from the certified model, labeled **Desired**.
`compile` produces **Intended**, conditional artifacts; it makes no **Applied** or
**Observed** claim. Exports are versioned views, not serialized proof objects and
are not accepted as authoritative compiler inputs. Re-elaborate source to obtain
fresh certificates. `fmt` parses/resolves source and emits canonical layout.

The compiler emits its public language catalog with `schema`. Release tools
compare catalogs and declared semantic changes. The canonical 1.0 fixture and
manifest must continue to pass under 1.0 interpretation. Future semantics must
not silently reinterpret them. Full grammar/schema migration is deferred until
another language version exists.
