# Implementation and verification report

The implemented release is the handoff's §50 compiler MVP and §56 architectural
acceptance criteria, plus small typed AAA/migration/state foundations. The
handoff explicitly defers the full later milestones; those are listed in the
README rather than represented as completed features.

## Consolidated result

`make test` completed successfully on macOS arm64 with Idris 2 0.8.0 and Chez
Scheme. It builds the compiler with totality checking, runs the public CLI suite,
builds/runs typed core tests, executes compile-negative tests, checks release
classification, and runs the independent semantic probes.

| Verification | Result |
| --- | --- |
| Public CLI acceptance tests | 52 passed |
| Exact reviewed backend artifact goldens | 4 passed: network, DHCP, firewall, IOS |
| Direct semantic API assertions | 34 passed |
| Independent graph decisions | 640/640 passed |
| Independent graph certificates/directed cycle witnesses | 640/640 passed |
| Additional exhaustive graph/core spike tests | 512 graph cases plus addressing, malformed-model rejection, migration and AAA groups passed |
| Forged bounds/certificate/reference compile-negative fixtures | 9 rejected as expected |
| Migration debt/evidence compile-negative fixtures | 3 rejected as expected |
| Release classification tests | 6 passed |
| Original source compatibility | Language 1.0 home example checks and compiles for both targets |
| Trusted project source escape-hatch search | No `believe_me`, `assert_total`, or partial semantic defaults found |

Independent agents designed/tested the public acceptance cases, reviewed target
contracts/security, and tested programmatic semantic APIs. The lead remained the
sole production-source implementer. Test agents edited designated test paths
only. All relevant reviews completed and findings were integrated.

## Acceptance criteria (§56)

| Criterion | Evidence |
| --- | --- |
| Human-readable external home configuration | `examples/home.net`, original pinned 1.0 fixture |
| Source-localized CIDR/VLAN/address/topology errors | JSON code/span/related-position assertions and negative fixtures |
| No unresolved string references in normalized IR | Kind-indexed numeric `Ref`, `PortRef`, checked inventory bounds |
| Global certificate-checked graph property | Exact-graph topological certificates; independent graph oracle/witness tests |
| Shared model across OpenWrt/Cisco | Both receive `StableNetwork` through one compile boundary |
| Renderers contain no generic network decisions | Generic UCI/IOS serialization in `Backend/Render.idr` |
| Explicit capability mismatch | Port/VLAN limits, interface labels, role/driver and lease-capacity tests |
| Provenance survives target compilation | Target JSON source maps with origin spans and expansion chains |
| Deterministic output | Repeated compilation and four exact goldens |
| Docs derive from the same model | Markdown/Mermaid generators take `StableNetwork` |
| Explicit source language version | Parser/resolver header checks; unknown versions rejected |
| Released-language compatibility fixtures | `tests/compatibility/1.0/home.net`, `docs/schema/1.0.json` |
| No unchecked secret material in artifacts | Literal secret source syntax unsupported; opaque typed reference API tested |
| No casual proof escape hatches | Total semantic modules; forged certificates rejected by Idris |
| Proven versus assumed/observed distinction | CLI, generated docs, target assumption metadata and explicit state types |

## Review issues resolved

- Malformed comma-only service lists can no longer become unrestricted allow rules.
- Public certification now checks model invariants independently of the parser.
- Prefix width and allocation size are tied by erased evidence.
- Gateway and DHCP declarations require a routing owner carrying their VLANs.
- Target name/interface restrictions and C1 control-character rejection are explicit.
- DHCP's required gateway DHCP/DNS traffic is documented; contradictory policy fails.
- Dnsmasq lease capacity covers the declared aggregate pools and respects a profile cap.
- Graph inventories require unique vertices; cycle witnesses follow actual directed edges.
- Out-of-inventory dependencies produce reference errors, not empty cycle diagnostics.

## Reproduction

```sh
make test
./netc check examples/home.net --format json
./netc compile examples/home.net --all --format json
./netc release-check docs/schema/1.0.json docs/schema/1.0.json
```

`examples/generated/` contains generated Markdown, Mermaid and intended-config
JSON for convenient inspection. These are sample build outputs, not deployment
state or operational snapshots.

## Remaining risks and deferred work

No Nix installation was present, so the pinned flake has not been evaluated or
built locally. No OpenWrt VM, Cisco emulator or physical device accepted these
artifacts during verification. Firmware behavior and profile assumptions require
integration testing on the intended hardware. Existing configuration reconciliation,
WAN configuration, NAT and IPv6 governance remain external to these artifacts.

The core certifies implemented predicates, not the completeness of the networking
specification or real-world service health. Idris, its totality checker, standard
library and runtime are part of the trust boundary. Migration/AAA modules are
foundation APIs: no full migration source planner, temporal orchestration,
observation adapter, secret materialization or AAA target realization is claimed.
