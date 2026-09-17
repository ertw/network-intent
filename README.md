# Network Intent DSL

An offline, vendor-neutral network-intent compiler. Idris 2 owns parsing, name
resolution, the typed semantic model, validation, graph certification, target
compilation, and rendering. Python is used only for tests and release tooling.

Language **3.0** and compiler **0.3.0** support VLAN switching and explicit router
intent. Versions 1.0 and 2.0 are no longer accepted; there is no compatibility adapter or
automatic migration command.

## Current status

`netc` is the existing **compiler release**: an offline checker and target compiler.
It does not observe live devices or install configuration.

This repository also contains an **in-progress broader suite**—runtime agents,
OpenWrt lab captures, identity/authorization foundations, and a fixture UI
harness. That work is not a ready service or deployment path. Current router
examples remain admission-blocked, and hardware acceptance is incomplete.

- Full objective: [implementation plan](docs/implementation-plan.md)
- Recorded progress: [goal progress](docs/goal-progress.md)
- Commands and verification layers: [developer guide](docs/developer-guide.md)
- Isolated OpenWrt lab: [OpenWrt lab](docs/openwrt-lab.md)
- Bounded Grok handoff (not the whole goal): [GROK_HANDOFF.md](GROK_HANDOFF.md)
- Component harness (development fixtures only, awaiting Astra integration): [frontend/README.md](frontend/README.md)

## Build and use

Requires Idris 2 **0.8.0**, its Chez Scheme backend, Make, and Python 3.10+.

```sh
make build
./netc check examples/core-router.net --format json
./netc compile examples/core-router.net --target gateway --format json
./netc docs examples/core-router.net
./netc graph examples/core-router.net
./netc export examples/core-router.net
./netc fmt examples/core-router.net
./netc compile examples/home.net --all --format json
make test
```

Commands write to stdout. They never install configuration or access devices.
`fmt` preserves comments and string values; save its output separately when
reviewing changes. Compilation JSON contains individual artifacts, source maps,
profile assumptions, and secret-binding readiness.

The included flake pins Nixpkgs and provides `nix develop`, `nix build .#netc`,
and `nix flake check`. Native macOS builds/tests are verified; Nix is not available
on the development host and its build remains unverified.

Runtime workspace tests, browser-compiler parity, and lab captures are documented
in the [developer guide](docs/developer-guide.md). They are not part of `make test`.

## Gateway and wireless satellite

[The two-device example](examples/wds-network.net) represents the supplied
`10.9.8.1` gateway and `10.9.8.2` bridged satellite in one network. `router`
selects the gateway; `device` gives the satellite independent configuration.
An explicit wireless link checks the WDS station against its upstream AP.

```sh
./netc check examples/wds-network.net
./netc compile examples/wds-network.net --all --format json
./netc docs examples/wds-network.net
./netc graph examples/wds-network.net
```

It covers STP, management gateway/DNS, unbound `wwan`, inactive DHCP settings,
and AP/station Wi-Fi. Segment validation detects address and DHCP conflicts
across declared wireless hops. The [parity report](docs/wds-network-parity.md)
accounts for both devices' networking settings. Gateway DHCP is corrected to
`.100–.199`; satellite DNS `10.8.8.1` is preserved. Six credential fields share
three external references. No live device access or installation is performed.

## Core-router example

[The core-router source](examples/core-router.net) retains parity with the earlier
OpenWrt regression backup's network, DHCP/DNS, firewall, and radio configuration:

- Untagged LAN bridge over `lan2`/`lan3`, with gateway `10.9.8.1/24`.
- LACP WAN bond over `lan1`/`wan`, carrying DHCP, DHCPv6, and modem access.
- IPv6 ULA configuration, delegated-prefix assignment, DHCPv6, and RA.
- Explicit firewall zones, IPv4 masquerading, MSS adjustment, and ordered rules.
- Three radio definitions and their AP settings, including the disabled AP.

The approved DHCP change is `dhcp start +100 max 100`, equivalent to
`dhcp +100 .. +199`. OpenWrt receives `start 100` and `limit 100`.

Wi-Fi credentials use opaque `secret://` references. Compilation produces a
**wireless template and binding manifest**, not installation-ready wireless
configuration. Read [the secrets methodology](docs/secrets.md) before consuming
these artifacts. Secret retrieval and materialization are outside this compiler.

The [parity report](docs/core-router-parity.md) accounts for all 188 backup
settings. Its only intended differences are the DHCP correction and two external
credential bindings. The tests compare sanitized backup fixtures against actual
compiler output, including firewall rule order.

## Implemented capabilities

- Versioned source, quoted strings, retained tokens, localized diagnostics,
  deterministic formatting, semantic JSON, Markdown inventories, and Mermaid.
- Checked IPv4/IPv6 addresses and prefixes; IPv4 pools as finite endpoints or
  network offset plus address count.
- VLANs, hosts, access/trunk ports, physical links, services, policy, on-link
  static routes, and dependency DAG certificates.
- Explicit typed links, bridges, bonds, logical interfaces, zones, radio/AP
  references, and typed DNS/DHCP settings.
- OpenWrt DSA VLAN and explicit dual-stack router profiles, plus Cisco IOS L2.
- Public certification that repeats model validation independently of parsing;
  target capability failures without partial output.
- Separate Intended, Applied, and Observed state concepts, with AAA and migration
  typestate foundations. Full AAA/migration realization is not implemented.

## Assurance and scope

`check` establishes implemented model invariants. `compile` additionally checks
its target profile. Neither establishes physical connectivity, runtime service
health, actual firmware acceptance, ISP delegation, or LACP peer state.

The explicit router profile owns the four generated networking packages. System,
management, upgrade, SSH identity, and certificate configuration is preserved
separately. Output is not an idempotent deployment delta or complete device image.
Existing conflicting configuration must be reconciled by the applying system.

VLAN shorthand derives one DSA bridge and IPv4 policy; its internet endpoint
requires an external WAN and does not infer NAT. Explicit routing declares its
interfaces, address families, NAT, and zone policies. See the
[language specification](docs/language.md), [typed setting catalog](docs/router-settings.md),
[backend contracts](docs/backends.md), and [verification notes](docs/verification.md).

VRFs, dynamic routing, VPNs, arbitrary vendor extensions, live observation,
deployment, and secret resolution remain outside this compiler release. In-tree
runtime, lab, and harness work toward observation and deployment is in progress;
it is not a ready service or deployment path. See [Current status](#current-status).

## Language release gate

```sh
./netc schema > current.json
./netc schema diff docs/schema/3.0.json current.json
./netc release-check docs/schema/3.0.json current.json
```

The catalog describes constructs, typed router fields, constraints, and semantics.
Release comparison requires appropriate version bumps for declared changes; it
is not a promise to accept older source versions. The current examples and tests
all use 3.0. Structural comparison cannot discover undeclared changes in meaning.
