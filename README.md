# Network Intent DSL

An offline, vendor-neutral network-intent compiler. Idris 2 owns the external
parser, name resolution, semantic model, validation, graph certificates, target
compilation and rendering. There is no Python or JavaScript substitute for the
semantic core.

This is the **first useful release** described in §50 of the
[handoff](docs/implementation-handoff.md), with the architectural acceptance
criteria from §56. Language version **1.0** and compiler version **0.1.0** are
independent. Later milestones are listed explicitly below.

## Build and use

Requires Idris 2 **0.8.0**, its Chez Scheme backend, Make, and Python 3.10+ for
tests and release tooling. On macOS, `brew install idris2` installs the compiler.

```sh
make build
./netc check examples/home.net
./netc check examples/home.net --format json
./netc compile examples/home.net --target gateway
./netc compile examples/home.net --target core
./netc compile examples/home.net --all --format json
./netc docs examples/home.net
./netc graph examples/home.net
./netc graph examples/dependencies.net --dependencies
./netc fmt examples/home.net
./netc export examples/home.net
./netc explain address.static-dhcp-overlap
make test
```

`fmt` writes to stdout and preserves comments and string values. Save it to a
different file before replacing the input. Compilation writes intended output
to stdout; JSON contains individual artifact paths, contents, assumptions and
source maps. Nothing is installed on a network device.

The included flake pins Nixpkgs to a specific revision and provides `nix develop`,
`nix build .#netc`, and `nix flake check`. The native macOS build and tests were
executed during implementation; Nix evaluation/build has **not** been verified
on this host, which has no Nix installation.

## Implemented

- Explicit versioned source, comments, quoted descriptions, retained tokens,
  source-localized structured errors and deterministic formatting.
- IPv4 prefixes and network-relative allocation, VLANs, static hosts, gateways,
  DHCP, devices, access/trunk ports and physical links.
- Typed resolved references; addressing, namespace, ownership, link, policy and
  static-route invariants checked at the public certification boundary.
- Service declarations and built-ins, ordered stateful IPv4 policy, on-link
  static routes and service/route dependency graphs with checked topological
  certificates and directed cycle witnesses.
- OpenWrt DSA network/DHCP/firewall4 artifacts; Cisco IOS L2 VLAN/interface
  artifacts; quantitative and target-specific capability failures.
- Markdown inventories, physical/dependency Mermaid graphs, versioned semantic
  JSON, source maps, schema manifests and conservative release-bump enforcement.
- Small typed **API spikes** for AAA contracts, opaque secret references,
  migration obligations and separate applied/observed states. These are tested
  foundations, not exposed deployment or AAA/migration source features.

## Assurance and scope

`check` establishes the implemented model invariants and graph certificates.
`compile` additionally checks a documented target capability profile. Vendor
firmware, capability declarations, Idris/the standard library and the runtime
remain trust assumptions. Neither command proves physical connectivity,
service health, target acceptance or operational correctness.

OpenWrt is the single routed-policy owner in this release. Cisco consumes the
same model's L2 projection. A gateway or DHCP declaration requires a routing
owner; a Cisco router or unsupported profile fails compilation. Internet means
an external `wan` logical interface and upstream route assumed to exist; no NAT
is inferred. DHCP implies gateway DHCP/DNS control traffic, and contradictory
denials fail validation. Read the [language specification](docs/language.md) and
[backend contracts](docs/backends.md) before using generated configurations.

The handoff's later milestones remain out of scope: full IPv6 surface syntax,
NAT, VPNs, VRFs, bridge/LAG source constructs, dynamic routing, multicast, MTU
reasoning, full AAA/migration syntax and realization, observation/drift adapters,
deployment, raw vendor extensions, and browser/editor integration. Unsupported
constructs are rejected. There is no secret materialization or real-device I/O.

See [DESIGN.md](DESIGN.md) for boundaries and [verification.md](docs/verification.md)
for the consolidated acceptance evidence.

## Language release gate

```sh
./netc schema > current.json
./netc schema diff docs/schema/1.0.json current.json
./netc release-check docs/schema/1.0.json current.json
```

Optional additions require at least a minor language bump; removals, required
additions, changed contracts, constraints or semantics require a major bump.
An explicit `semanticBump` declaration may raise the minimum, never lower it.
Backend/compiler/export versions are independent. The baseline and original
1.0 example are retained as compatibility fixtures. Structural comparison
cannot discover undeclared changes in meaning; release review must declare them.
