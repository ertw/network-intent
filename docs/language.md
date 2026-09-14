# Network Intent language 2.0

Only language 2.0 is accepted. Version 1.0 has been retired; there is no compatibility adapter. Unknown versions and constructs
are errors. The authoritative grammar and meaning are implemented in Idris;
there is no separate editor parser in this release.

## Lexical grammar and document

```text
network-language 2.0
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

## VLAN shorthand and IPv4 addressing

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
canonical (host bits zero). VLAN prefixes cannot overlap in the single
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
routers are rejected in 2.0 instead of selecting an owner implicitly. OpenWrt
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
VLAN shorthand derives one internal DSA bridge from physical port memberships.
The explicit `routing` model below declares bridges, bonds and logical interfaces
directly. A router cannot combine explicit routing with VLAN port declarations.

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
compare catalogs and declared semantic changes. The current 2.0 examples and schema are the baseline. Older source versions are
rejected; release comparison does not establish a backwards-compatibility promise.


## Explicit router intent

The complete [core-router example](../examples/core-router.net) covers all four
networking packages from the backup. Declare this model inside one router:

```text
network-language 2.0
network sample {
  router gateway {
    driver openwrt
    routing {
      physical lan1
      physical lan2
      physical lan3
      physical wan
      bridge br-lan { members lan2 lan3 }
      bond bond-wan {
        members lan1 wan
        policy 802.3ad
        hash-policy layer3+4
        lacp-rate fast
        min-links 1
      }
      interface lan {
        attach br-lan
        protocol static
        address 10.9.8.1/24
      }
      interface wan {
        attach bond-wan
        protocol dhcp
      }
      firewall {
        input REJECT
        output ACCEPT
        forward REJECT
      }
      zone lan {
        interfaces lan
        input ACCEPT
        output ACCEPT
        forward ACCEPT
      }
      zone wan {
        interfaces wan
        input REJECT
        output ACCEPT
        forward DROP
        masquerade true
        mss-adjust true
      }
      forward lan -> wan
    }
  }
}
```

Physical links, bridges, and bonds share a device-local name namespace; logical
interfaces, zones, radios, and APs have distinct typed namespaces. Their resolved
references are numeric inventory IDs with distinct kinds. `lo` is the built-in
loopback attachment and cannot be redeclared. Link names must fit Linux's
15-character interface limit. Named UCI sections (logical interfaces, DHCP
sections, radios, APs) require ASCII letters, digits, or underscores at compilation.
Generated section-name collisions are errors, not silently renamed interfaces.

A bridge has nonempty `members` referencing declared physical links, bonds, or
bridges. A bond has at least two physical members and `policy 802.3ad`. Its
minimum active links cannot exceed membership. A link may have only one master;
duplicate memberships, missing references, and attachment cycles fail certification.
A logical interface attaches to a master or an independent link, never a slave.
Multiple logical interfaces can share the same attachment, as the WAN DHCP,
DHCPv6, and static modem interfaces do in the example.

Interfaces require `attach` and `protocol`. Protocols are `static`, `dhcp`,
`dhcpv6`, and `none`. Static interfaces require at least one `address` or
`address6`; dynamic/unnumbered interfaces cannot declare static addresses.
IPv4 interface addresses use host/prefix notation, while subnet declarations
remain canonical network prefixes. Static interface subnets cannot overlap
between logical interfaces in this single routing context. Duplicate addresses
and unusable IPv4 network/broadcast addresses are rejected.

`globals` optionally declares the ULA prefix, DHCP DUID, and packet steering.
IPv6 addresses use full or compressed hexadecimal notation. IPv6 network
prefixes have checked widths in 0..128 and canonical host bits; interface IPv6
addresses retain their host bits. A static downstream interface can declare
`ipv6-assignment 60`; DHCPv6 clients can declare `request-address try`,
`request-prefix auto`, and `no-release true`. Acquired addresses and delegated
prefixes remain **Unknown**, not fabricated static allocations.

### DNS, DHCP and router advertisements

`dns { ... }` and `odhcp { ... }` configure the respective daemons. All supported
settings are listed in the [typed setting catalog](router-settings.md).

```text
dhcp-server lan {
  interface lan
  dhcp start +100 max 100
  lease-time "12h"
  ipv4 server
  ipv6 server
  ra server
  ra-flags managed-config other-config
  ra-preference medium
}
dhcp-server wan {
  interface wan
  ignore true
}
```

Both explicit DHCP servers and VLAN shorthand accept `dhcp START .. END` or
`dhcp start +OFFSET max COUNT`. Endpoints are inclusive; count is positive;
`last = subnet base + offset + count - 1`. Finite endpoints can be absolute IPv4
addresses or `+OFFSET` values. Count form requires an explicit `+OFFSET`.
Both forms normalize to the same checked endpoints, and OpenWrt receives the
network-relative `start` and address-count `limit`.

IPv4 DHCP serving requires exactly one static IPv4 subnet, a valid pool, DNS
configuration, and permitted IPv4 router input for UDP/67 and TCP+UDP/53. Pools
cannot include interface addresses or network/broadcast addresses. One DHCP
section is permitted per interface. Ignored interfaces cannot enable servers.
An explicit lease capacity must cover all pools; if omitted, the backend adds
capacity only when the aggregate exceeds the profile's default of 150. The
profile permits at most 65,535 leases.

IPv6 DHCP/RA serving requires downstream IPv6 configuration and an `odhcp`
block. RA flags require an RA server. This profile configures those services;
it does not prove that the ISP grants the requested prefix or that clients
receive usable leases. IPv4 arithmetic is never reused as IPv6 pool allocation.

### Firewall zones and ordered rules

`firewall` and every zone require explicit `input`, `output`, and `forward`
verdicts: `ACCEPT`, `REJECT`, or `DROP`. A zone has a nonempty list of logical
`interfaces`; each interface belongs to at most one zone. `forward A -> B`
permits forwarding between distinct zones. `masquerade true` is explicit IPv4
NAT; `mss-adjust true` requests the firewall's MTU/MSS adjustment. Neither is
inferred from internet connectivity.

```text
firewall-rule Allow-DHCP-Renew {
  source wan
  protocols udp
  destination-port 68
  family ipv4
  action ACCEPT
}
firewall-rule Allow-ICMPv6-Forward {
  source wan
  destination any
  protocols icmp
  icmp-types packet-too-big time-exceeded
  family ipv6
  limit "1000/sec"
  action ACCEPT
}
```

A rule requires a source zone, protocols, and action. Omitted destination means
local router input; a named zone means forwarding, and `any` means forwarding
to any zone. Rules retain source order. Protocols are `all`, `tcp`, `udp`, `icmp`,
`icmpv6`, `igmp`, and `esp`; `all` must occur alone. Destination ports require
TCP/UDP. ICMP types require ICMP protocols and accept the documented named types
or numeric type/code values in 0..255. IPv6-specific names require `family ipv6`.
IGMP requires IPv4; explicit `icmpv6` requires IPv6. `icmp` with `family ipv6`
uses OpenWrt's IPv6 ICMP interpretation, matching the backup.

Families are `ipv4`, `ipv6`, or `any`; omission leaves firewall4 family inference
in place. A `source-prefix` must be a canonical prefix with a matching explicit
family. A rate limit is a positive count followed by `/s`, `/sec`, `/second`,
`/minute`, `/hour`, or `/day`. Stateful established/related handling remains a
firewall4 profile assumption; policy edits do not revoke existing connections.
DHCP-control validation conservatively requires unconditional input permission;
conditional ACCEPT rules cannot establish that permission for every client.

### Radios, access points and credentials

`radio NAME` requires `driver mac80211`, hardware `path`, `band`, `channel`, and
`width`. `country` is a two-letter uppercase code. Basic band/channel/width
constraints are checked; device capabilities, regulatory availability, DFS,
and actual radio state remain target assumptions.

`access-point NAME` requires a `radio`, bridged logical `interface`, `mode ap`,
`ssid`, and `security`. SSIDs contain 1..32 UTF-8 bytes. Supported security modes
are `none`, `psk2`, and `sae`. `disabled true` preserves a disabled AP; it does
not discard its configuration. Multiple radios and APs may share one bridge.

Secured APs require `credential secret://...`; open APs forbid credentials.
Literal passwords are rejected. The [secrets methodology](secrets.md) defines
identifier syntax, template artifacts, the binding manifest, and the external
consumer contract. The compiler never retrieves or materializes credentials.
