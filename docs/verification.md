# Implementation and verification report

Language 2.0 adds explicit typed router intent to the VLAN/network compiler.
The current baseline accepts only 2.0. The original release's backwards-source
compatibility requirement has been removed; examples, tests, and schemas are
migrated together.

## Verification coverage

The full `make test` run completed successfully on the implementation host.

`make test` builds the compiler with Idris 2 totality checking, runs the public
CLI tests, builds/runs core tests, checks migration typestate and release
classification, and runs independent direct-API/compile-negative probes.

| Verification | Coverage |
| --- | --- |
| Existing CLI acceptance suite | 52 tests, migrated to 2.0 |
| Explicit router acceptance suite | 19 tests with boundary/negative subcases |
| Backup parity | 188 source options across network, DHCP, firewall, wireless; firewall rule order preserved |
| Reviewed target goldens | 4 VLAN/IOS artifacts plus 5 core-router artifacts |
| Core-router public API probes | 22 assertions including synthetic credential quoting |
| Existing public semantic API probes | 34 assertions |
| Graph oracle and certificate/witness tests | 640 decision cases and 640 certificate/witness cases |
| Additional graph/core foundations | 512 graph cases plus address, malformed-model, migration and AAA groups |
| Compile-negative bounds/reference/certificate probes | 13 fixtures, including IPv6 and router/Wi-Fi reference kinds |
| Migration debt/evidence compile-negative probes | 3 fixtures |
| Release classification | 6 tests using the 2.0 schema baseline |

## Router acceptance evidence

The source example is `examples/core-router.net`. Its fixtures contain only the
four networking packages from the supplied backup, with Wi-Fi passwords replaced
by redaction markers. Private keys, password hashes, and management configuration
are not copied into the repository.

The independent UCI oracle in `tests/router_parity.py` compares every original
option with actual CLI output. It normalizes anonymous section identities,
option order, singleton list syntax, IPv4 address/netmask notation, and IPv6
compression. The source modem netmask has its own report row mapping to the
emitted address prefix. Firewall rule order is compared separately and retained.

The only intended behavior change is LAN DHCP `limit 199` to `limit 100`, yielding
`10.9.8.100–10.9.8.199`. Two Wi-Fi options require external credential binding.
The [parity report](core-router-parity.md) records each mapping.

Tests cover finite versus offset/count DHCP syntax in both explicit router and
VLAN declarations, single-address and subnet-end boundaries, overflow, invalid
counts, overlap, cross-octet pools, and lease capacity. They also exercise
shared bond interfaces, missing/duplicate attachments, cycles, slave bindings,
protocol/address conflicts, IPv6 syntax and prefix constraints, firewall
protocol/family/port/type consistency, DHCP control traffic, and radio/security
constraints.

The source-kind and secret-kind probes test the public Idris boundary directly,
so source-parser checks cannot mask a missing certification check. The core
retains total definitions and checked bounds without `believe_me`, `assert_total`,
or partial semantic defaults.

## Secret artifacts

The wireless AST has a distinct secret option constructor. Rendering creates
`wireless.template` and `secret-bindings.json` with deterministic placeholders,
references, security modes, installation destinations, and source-map entries.
JSON readiness explicitly reports `requires-secret-binding`.

Tests check manifest/template consistency, source maps, deterministic output,
required/forbidden credentials, invalid-reference error redaction, secret-free
wireless output, and generic UCI quoting with synthetic credentials and SSIDs.
No retrieval, secret materialization, or deployment is implemented. See
[the secrets methodology](secrets.md).

## Reproduction

```sh
make test
./netc check examples/core-router.net --format json
./netc compile examples/core-router.net --target gateway --format json
./netc compile examples/home.net --all --format json
./netc release-check docs/schema/2.0.json docs/schema/2.0.json
```

`examples/generated/` contains current Markdown, Mermaid, semantic JSON and
intended output for inspection. These are compiler artifacts, not operational
snapshots. Goldens intentionally exclude volatile timestamps and secret values.

## Trust boundary and unverified integration

Verification uses macOS arm64, Idris 2 0.8.0, and Chez Scheme. Nix is unavailable
on this host, so the flake is not locally verified. No physical router, OpenWrt
VM, or Cisco emulator has accepted these artifacts during this work.

Firmware compatibility, native bonding support, LACP peer state, ISP leases and
prefix delegation, wireless capabilities/regulatory state, and operational
service health remain target assumptions. Existing configuration reconciliation,
external credential binding, and device application remain separate steps.
The core certifies its implemented predicates, not the completeness of a router
specification or real-world behavior. AAA/migration modules remain foundations,
not full AAA realization, observation adapters, or temporal deployment planners.
