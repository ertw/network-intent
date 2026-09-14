# Target realization contracts

Both target compilers accept the same `StableNetwork`. They perform capability
checks, build typed target ASTs, then use generic renderers. JSON compilation
includes intended-state labels, profile assumptions and provenance chains.
Capability metadata is an assumption about firmware/hardware, not a discovery
result. No live firmware or device has been tested in this implementation.

## OpenWrt: `openwrt-dsa-fw4-ipv4-v2`

Assumes DSA/netifd, firewall4 and dnsmasq, and matching Linux physical port labels.
Names are at most 15 ASCII characters and cannot contain `/` or be `.`/`..`.
This follows Linux interface constraints and the [OpenWrt DSA model](https://openwrt.org/docs/guide-user/network/dsa/dsa-mini-tutorial).

- `network`: one `br-net` bridge with VLAN filtering, bridge-VLAN sections,
  `port:u*` access membership and `port:t` tagged membership. Routed logical
  interfaces use `br-net.ID`, static IPv4 and the prefix-derived netmask.
  L2-only devices use `proto none`. Static routes use resolved interface IDs.
- `dhcp`: the router emits dnsmasq configuration, DHCP ranges and static DNS
  records. Start is relative to the prefix base; limit is inclusive range size.
  Lease time is 12 hours. Aggregate lease capacity is explicitly at least the
  declared pool total (minimum 150), with a conservative profile maximum of
  65,535. No MAC reservation is fabricated. [OpenWrt DNS/DHCP settings](https://openwrt.org/docs/guide-user/base-system/dhcp)
- `firewall`: IPv4 stateful default-drop input/forwarding, accepted router output,
  one zone per carried VLAN, source-only local gateway rules and source/destination
  forwarding rules. Named service transport expansions preserve rule order and
  source provenance. Required DHCP/DNS permissions are explicit and conflicting
  denials are rejected. [OpenWrt firewall semantics](https://openwrt.org/docs/guide-user/firewall/firewall_configuration)

Section identifiers derive from numeric IDs, so `foo-bar` and `foo_bar` cannot
collide through name sanitization. Values use single quotes, with apostrophes
escaped using libuci's export convention. All control/line characters are
rejected. [UCI configuration syntax](https://openwrt.org/docs/guide-user/base-system/uci),
[libuci exporter](https://lxr.openwrt.org/source/uci/file.c)

The generated scope owns its bridge/interfaces and, for the router, DHCP and
firewall packages. It is not an idempotent deployment delta or complete hardware
image. Existing conflicting settings must be reconciled before deployment.
In particular, internet policy assumes an existing external logical `wan` and
upstream route; it never adds NAT. IPv6 must be disabled or governed separately.

## Cisco: `cisco-ios-l2-v2`

Assumes a Catalyst-style IOS L2 target supporting explicit access/trunk modes,
allowed VLAN lists, static negotiation control and native tagging. VLAN names
are limited to 32 ASCII characters, and VLANs 1002–1005 are rejected for this
profile. Generic IDs 1–4094 remain valid in the semantic model.
[Cisco VLAN commands](https://www.cisco.com/c/en/us/td/docs/switches/lan/catalyst9300/software/release/16-9/command_reference/b_169_9300_cr/vlan_commands.html)

Supported interface-family labels include Ethernet, FastEthernet,
GigabitEthernet, TenGigabitEthernet and the explicitly listed higher-speed
families/abbreviations in the capability checker, followed by numeric slot/port
segments. Validating syntax does not establish that a physical port exists.

The compiler emits VLAN/name commands, interface descriptions, access mode and
VLAN commands, or trunk mode/allowed-VLAN/nonegotiate commands. Tagged-only intent
requires `vlan dot1q tag native`, which affects all trunks and is explicitly part
of the owned profile. Native-tagging behavior for ordinary payload does not
constitute a claim about every L2 control protocol.
[Cisco native-tagging command](https://www.cisco.com/c/en/us/td/docs/ios/lanswitch/command/reference/lsw_book/lsw_u1.html)

Cisco receives only its owned L2 projection; routing, DHCP and stateful policy
belong to the network's router. A Cisco device declared as `router` fails, rather
than approximating stateful policy with stateless ACLs. Actual command acceptance
and behavior need a lab/simulator test for the chosen firmware before deployment.

## Realization limitations

Quantitative limits include tagged VLANs per port and OpenWrt DHCP lease count.
Other hardware budgets, licensing, firmware interactions and dynamic capability
discovery remain future work. The backends neither deploy nor inspect existing
configuration, and therefore do not infer deletion, adoption or successful apply.
The configuration artifacts and tests support confidence in the documented
abstract profile, not a formal theorem of vendor firmware behavior.


## OpenWrt explicit router: `openwrt-router-fw4-dualstack-v2`

The `routing` model owns `network`, `dhcp`, `firewall`, and declared wireless
configuration. Its typed bridge/bond/interface graph replaces the derived VLAN
bridge for that router. Physical labels and radio paths must match the target.
The profile assumes native netifd bonding, firewall4, dnsmasq, odhcpd, and
mac80211/wpad supporting the declared radio and security settings. Actual
firmware/package versions are not present in the backup and are not discovered.

- Network output preserves declared link and logical-interface names, static
  addresses, DHCP/DHCPv6 clients, globals, and explicit bond parameters.
  Bonding uses native `config device` / `type bonding`, not the older
  protocol-based bonding package. Multiple interfaces may share a bond.
- DHCP output preserves typed dnsmasq, pool, DHCPv6/RA, and odhcpd options.
  Address/count form emits the same range as finite endpoints. Derived capacity
  is emitted only when an unspecified default of 150 would be insufficient.
- Firewall output preserves zone verdicts, forwarding, explicit IPv4 NAT,
  MSS adjustment, family/protocol matches, ICMP types, and rule order. Generated
  zone/rule section IDs are safe numeric identifiers; their semantic names stay
  in `option name`. `REJECT` is never approximated with `DROP`.
- Wireless output preserves radio/AP identifiers and configuration. Secret-bearing
  UCI fields have a dedicated AST constructor. They produce a `.template`
  artifact and binding manifest with readiness metadata. See
  [secrets.md](secrets.md); secret resolution and installation are external.

The [parity report](core-router-parity.md) compares all 188 supplied options.
Equivalent IPv4 netmask/CIDR notation, IPv6 compression, singleton list forms,
option ordering, and anonymous section IDs are normalized. Firewall rule order
is preserved. The only intended behavior change is the approved DHCP count.

The profile makes no Applied or Observed claim. ISP leases/prefix delegation,
LACP peer negotiation, wireless capabilities/regulatory state, external secrets,
and services remain unknown. Management/OS settings are outside owned scope.
An applying system must reconcile existing configuration before replacement.

References: [native netifd bonding](https://lxr.openwrt.org/source/netifd/bonding.c),
[firewall4 option handling](https://lxr.openwrt.org/source/firewall4/root/usr/share/ucode/fw4.uc),
[UCI syntax and identifiers](https://openwrt.org/docs/guide-user/base-system/uci),
[DHCP/DNS options](https://openwrt.org/docs/guide-user/base-system/dhcp).
