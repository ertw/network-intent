# Network home-wds

Language 3.0. State: **Desired**. Domain: unspecified.

Model invariants and dependency ordering are certificate-checked by the Idris semantic core. Target realization is conditional on documented profiles. Applied state, physical connectivity, service health and observations are **Unknown**.

Routing/policy owner: gateway.

| Wireless station | Upstream AP |
| --- | --- |
| satellite.wifinet3 | gateway.wifinet3 |

## Device gateway

Explicit dual-stack interface and zone intent. Dynamic addresses, delegated prefixes, link state, and radio operation are **Unknown**.

| Link | Kind | Members | STP |
| --- | --- | --- | --- |
| br-lan | bridge | lan2, lan3 | default |
| bond-wan | bond | lan1, wan | n/a |

| Interface | Attachment | Protocol | IPv4 | IPv6 assignment | Gateway | DNS |
| --- | --- | --- | --- | --- | --- | --- |
| loopback | lo | static | 127.0.0.1/8 | none | none |  |
| lan | br-lan | static | 10.9.8.1/24 | /60 | none |  |
| wan | bond-wan | dhcp |  | none | none |  |
| wan6 | bond-wan | dhcpv6 |  | none | none |  |
| modem | bond-wan | static | 192.168.100.2/24 | none | none |  |

| DHCP interface | Pool | Active leases | IPv6 server | Router advertisements |
| --- | --- | --- | --- | --- |
| lan | 10.9.8.100 .. 10.9.8.199 | 100 | server | server |
| wan | none | 0 | default | default |

| Zone | Interfaces | Input | Output | Forward | IPv4 NAT |
| --- | --- | --- | --- | --- | --- |
| lan | lan | ACCEPT | ACCEPT | ACCEPT | no |
| wan | wan, wan6, modem | REJECT | ACCEPT | DROP | yes |

Firewall rules retain declaration order.

| Rule | From | To | Protocols | Action |
| --- | --- | --- | --- | --- |
| Allow-DHCP-Renew | wan | router | udp | ACCEPT |
| Allow-Ping | wan | router | icmp | ACCEPT |
| Allow-IGMP | wan | router | igmp | ACCEPT |
| Allow-DHCPv6 | wan | router | udp | ACCEPT |
| Allow-MLD | wan | router | icmp | ACCEPT |
| Allow-ICMPv6-Input | wan | router | icmp | ACCEPT |
| Allow-ICMPv6-Forward | wan | any zone | icmp | ACCEPT |
| Allow-IPSec-ESP | wan | lan | esp | ACCEPT |
| Allow-ISAKMP | wan | lan | udp | ACCEPT |

| Wi-Fi interface | Mode | SSID | Radio | Interface | Security | Enabled | WDS | Hidden | BSSID | Credential reference |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| default_radio1 | ap | _iot | radio1 | lan | psk2 | yes | default | default | unspecified | secret://home/wifi/iot |
| wifinet3 | ap | _backhaul | radio2 | lan | sae | yes | yes | yes | unspecified | secret://home/wifi/backhaul |
| wifinet2 | ap | _ | radio0 | lan | sae | yes | default | default | unspecified | secret://home/wifi/main |

Secret references identify externally held credentials. Wireless templates require binding before installation; the compiler never resolves credentials.

## Device satellite

Explicit dual-stack interface and zone intent. Dynamic addresses, delegated prefixes, link state, and radio operation are **Unknown**.

| Link | Kind | Members | STP |
| --- | --- | --- | --- |
| br-lan | bridge | lan1, lan2, lan3 | enabled |

| Interface | Attachment | Protocol | IPv4 | IPv6 assignment | Gateway | DNS |
| --- | --- | --- | --- | --- | --- | --- |
| loopback | lo | static | 127.0.0.1/8 | none | none |  |
| lan | br-lan | static | 10.9.8.2/24 | /60 | 10.9.8.1 | 10.8.8.1 |
| wan | wan | dhcp |  | none | none |  |
| wan6 | wan | dhcpv6 |  | none | none |  |
| wwan | none | dhcp |  | none | none |  |

| DHCP interface | Pool | Active leases | IPv6 server | Router advertisements |
| --- | --- | --- | --- | --- |
| lan | 10.9.8.100 .. 10.9.8.249 | 0 | default | default |
| wan | none | 0 | default | default |

| Zone | Interfaces | Input | Output | Forward | IPv4 NAT |
| --- | --- | --- | --- | --- | --- |
| lan | lan, wwan | ACCEPT | ACCEPT | ACCEPT | no |
| wan | wan, wan6 | REJECT | ACCEPT | DROP | yes |

Firewall rules retain declaration order.

| Rule | From | To | Protocols | Action |
| --- | --- | --- | --- | --- |
| Allow-DHCP-Renew | wan | router | udp | ACCEPT |
| Allow-Ping | wan | router | icmp | ACCEPT |
| Allow-IGMP | wan | router | igmp | ACCEPT |
| Allow-DHCPv6 | wan | router | udp | ACCEPT |
| Allow-MLD | wan | router | icmp | ACCEPT |
| Allow-ICMPv6-Input | wan | router | icmp | ACCEPT |
| Allow-ICMPv6-Forward | wan | any zone | icmp | ACCEPT |
| Allow-IPSec-ESP | wan | lan | esp | ACCEPT |
| Allow-ISAKMP | wan | lan | udp | ACCEPT |

| Wi-Fi interface | Mode | SSID | Radio | Interface | Security | Enabled | WDS | Hidden | BSSID | Credential reference |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| wifinet3 | sta | _backhaul | radio2 | lan | sae | yes | yes | default | E8:9F:80:69:CB:54 | secret://home/wifi/backhaul |
| wifinet1 | ap | _ | radio0 | lan | sae | yes | default | default | unspecified | secret://home/wifi/main |
| wifinet2 | ap | _iot | radio1 | lan | psk2 | yes | default | default | unspecified | secret://home/wifi/iot |

Secret references identify externally held credentials. Wireless templates require binding before installation; the compiler never resolves credentials.

## VLANs and addressing

| VLAN | ID | IPv4 prefix | Gateway | DHCP |
| --- | --- | --- | --- | --- |

## Hosts

Static IP assignments must be configured on the hosts. No MAC-bound reservations are inferred.

| Host | VLAN | Address |
| --- | --- | --- |

## Devices and ports

| Device | Driver | Port | Membership | Connects to |
| --- | --- | --- | --- | --- |

## Services

| Service | Transport ports | Dependencies |
| --- | --- | --- |
| dns | udp/53, tcp/53 |  |
| ntp | udp/123 |  |
| http | tcp/80 |  |
| https | tcp/443 |  |
| ssh | tcp/22 |  |

## Static routes

| Route | Destination | Next hop | Device | VLAN | Metric |
| --- | --- | --- | --- | --- | --- |

## VLAN shorthand policy matrix

For VLAN shorthand: IPv4 connection initiation; established/related return traffic is accepted. New input and forwarding default to deny. Rules use declaration order. Gateway means local input; other destinations mean forwarding. DHCP requires gateway UDP/67 and TCP+UDP/53; contradictory denials are rejected. NAT is not inferred. Existing flows are not revoked.

| From | To | Action | Services |
| --- | --- | --- | --- |

## AAA and migration

AAA realization and migration source syntax are deferred beyond language 3.0. No AAA assurance or operational observation is inferred. This document represents a stable model with no migration debt.

## Physical topology

```mermaid
flowchart LR
  %% Desired physical topology; not observed connectivity
  r1ap0 -. WDS .-> r0ap1
  r0link0["lan1"]
  r0link1["lan2"]
  r0link2["lan3"]
  r0link3["wan"]
  r0link4["br-lan bridge"]
  r0link5["bond-wan bond"]
  r0link6["lo"]
  r0link1 --> r0link4
  r0link2 --> r0link4
  r0link0 --> r0link5
  r0link3 --> r0link5
  r0iface0["loopback / static"]
  r0link6 --> r0iface0
  r0iface1["lan / static"]
  r0link4 --> r0iface1
  r0iface2["wan / dhcp"]
  r0link5 --> r0iface2
  r0iface3["wan6 / dhcpv6"]
  r0link5 --> r0iface3
  r0iface4["modem / static"]
  r0link5 --> r0iface4
  r0radio0["radio0"]
  r0radio1["radio1"]
  r0radio2["radio2"]
  r0ap0["default_radio1 / _iot"]
  r0radio1 --> r0ap0
  r0ap0 --> r0iface1
  r0ap1["wifinet3 / _backhaul"]
  r0radio2 --> r0ap1
  r0ap1 --> r0iface1
  r0ap2["wifinet2 / _"]
  r0radio0 --> r0ap2
  r0ap2 --> r0iface1
  r1link0["lan1"]
  r1link1["lan2"]
  r1link2["lan3"]
  r1link3["wan"]
  r1link4["br-lan bridge"]
  r1link5["lo"]
  r1link0 --> r1link4
  r1link1 --> r1link4
  r1link2 --> r1link4
  r1iface0["loopback / static"]
  r1link5 --> r1iface0
  r1iface1["lan / static"]
  r1link4 --> r1iface1
  r1iface2["wan / dhcp"]
  r1link3 --> r1iface2
  r1iface3["wan6 / dhcpv6"]
  r1link3 --> r1iface3
  r1iface4["wwan / dhcp"]
  r1radio0["radio0"]
  r1radio1["radio1"]
  r1radio2["radio2"]
  r1ap0["wifinet3 / _backhaul"]
  r1radio2 --> r1ap0
  r1ap0 --> r1iface1
  r1ap1["wifinet1 / _"]
  r1radio0 --> r1ap1
  r1ap1 --> r1iface1
  r1ap2["wifinet2 / _iot"]
  r1radio1 --> r1ap2
  r1ap2 --> r1iface1
```

## Dependency graph

```mermaid
flowchart LR
  s0["dns"]
  s1["ntp"]
  s2["http"]
  s3["https"]
  s4["ssh"]
```
