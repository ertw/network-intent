# Developer guide

This guide records how to check the in-tree compiler and runtime on a local
checkout, and which recorded evidence layers exist. It does not install
toolchains, operate devices, or claim a ready observation or deployment
service. The [root README](../README.md) remains the compiler-release entry
point.

## Local toolchains

Use `nix develop` for the pinned toolchain on macOS ARM64 and Linux x64.
See the [Nix build commands](../README.md#build-and-use) for packages and checks.

| Tool | Recorded requirement | Where recorded |
|---|---|---|
| Idris 2 | **0.8.0**, Chez Scheme backend | [README](../README.md), [verification notes](verification.md) |
| Make | used by the targets below | [Makefile](../Makefile) |
| Python | **3.10+** | [README](../README.md) |
| Rust / Cargo | minimum **1.89** | [Cargo.toml](../Cargo.toml) `workspace.package.rust-version` |
| Node | invoked by the browser-parity script (`node -e`) | [scripts/check_browser_parity.py](../scripts/check_browser_parity.py) |

Compiler package version is **0.3.0** ([network-intent.ipkg](../network-intent.ipkg)).
The flake provides native `netc` and `runtime` packages on both supported
systems. `nix flake check` runs compiler acceptance and Rust workspace tests;
frontend commands and browser parity remain separate checks in the development
shell. Foreign-platform evaluation is available with
`nix flake check --all-systems --no-build`; actual builds require that platform.

QEMU, GnuPG, and OpenWrt image tooling are lab-only. Their workflow lives in
the [OpenWrt lab](openwrt-lab.md) document and is not a `Makefile` target.

## Commands

The [Makefile](../Makefile) defines exactly these phony targets: `build`,
`test`, `test-runtime`, `test-browser`, and `clean`. Do not treat other names
as executable Make targets.

| Command | What it runs | What a pass establishes |
|---|---|---|
| `make build` | `idris2 --build network-intent.ipkg` | Native `netc` compiler build with totality checking. No device access. |
| `make test` | `build`, then `python3 tests/run.py`, `tests/router.py`, `tests/wireless.py`, `idris2 --build core-tests.ipkg` and that executable, then `scripts/check_typestate.py`, `check_release.py`, `check_core_probes.py` | Source/compiler checking recorded in [verification notes](verification.md). Not physical connectivity, firmware acceptance, or runtime services. |
| `make test-browser` | `python3 scripts/check_browser_parity.py` | Builds [browser.ipkg](../browser.ipkg) and compares eight fixtures against a temporary native evaluator ([browser compiler](browser-compiler.md)). Compiler JSON parity only; not a product UI. |
| `make test-runtime` | `build`, then `cargo test --workspace` | Host-side Rust workspace tests after a compiler build. Some tests invoke the native compiler. Not Linux `native-ubus` feature compilation or live adapter execution. |
| `cargo test --workspace` | Cargo workspace tests from the repository root | Same cargo step as `make test-runtime`, without the Makefile's preceding `make build`. |
| `make clean` | `rm -rf build` | Deletes Idris build output. Not a verification target. |

`make test-runtime` depends on `make build` because admission tests execute the
authoritative native compiler
([compiler-admission log](verification-runs/compiler-admission-2026-09-15.log)).

Frontend npm scripts, native OpenWrt adapter builds, QEMU lab launches, and
hardware acceptance are **not** Makefile targets. Lab commands are documented
in [openwrt-lab.md](openwrt-lab.md); treat them as isolated lab operations, not
as everyday compiler checks.

## Verification layers

Keep these layers distinct. A pass in one layer does not establish the others.
Counts below are historical recordings; this guide does not re-run those jobs.

### 1. Source and compiler checking — recorded

`make test` on the implementation host. Coverage and counts are recorded in
[verification notes](verification.md) and
[compiler-full-2026-09-15.log](verification-runs/compiler-full-2026-09-15.log).
`check` / `compile` certify implemented model and target predicates. They do
not establish physical connectivity, runtime service health, firmware
acceptance, ISP delegation, or LACP peer state ([README](../README.md)).

`make test-browser` recorded eight native/browser parity fixtures
([browser-parity-2026-09-15.log](verification-runs/browser-parity-2026-09-15.log),
[goal progress](goal-progress.md)). Compiler success is not revision admission
([browser compiler](browser-compiler.md)).

**Admission of current router examples remains blocked** because semantic and
operational assurance coverage is incomplete
([goal progress](goal-progress.md), [GROK_HANDOFF.md](../GROK_HANDOFF.md)).

### 2. Runtime host tests — recorded at `d33310c`

**165** Rust workspace tests passed at runtime foundation commit `d33310c`
([goal progress](goal-progress.md),
[runtime-integration-2026-09-16.log](verification-runs/runtime-integration-2026-09-16.log)).
That run includes protocol/controller/identity/authorization tests, apply-agent
planner and journal tests, witness host tests (including native-helper
fixtures), and parsers over committed OpenWrt fixtures.

These are host tests. They do not compile the Linux `native-ubus` helper against
OpenWrt libubus, do not execute that helper on a guest, and do not prove a
production write path.

Identity, Cedar, and mTLS foundations exist in-tree; they are not complete
services ([goal progress](goal-progress.md)). Apply-agent planning and a guarded
deployment journal exist; there is no native UCI transport or production write
path ([apply-agent README](../runtime/apply-agent/README.md)). Runtime contracts
are initial implementation contracts, not an acceptance claim
([runtime contracts](runtime-contracts.md)).

### 3. Captured VM response fixtures — recorded

Pinned OpenWrt **25.12.5** and **24.10.8** x86/64 images passed SHA-256 and
release-signature verification and booted in isolated QEMU. Baseline serial
transcripts and parsed fixtures are committed
([openwrt-lab.md](openwrt-lab.md),
[25.12.5 baseline log](verification-runs/openwrt-25.12.5-baseline.log),
[24.10.8 baseline log](verification-runs/openwrt-24.10.8-baseline.log),
`lab/openwrt/fixtures/<version>/`).

Those captures use direct root ubus reads. They record real response shapes and
boot behavior. They do **not** prove witness read-only identity enforcement or
compiled native adapter execution.

Host tests that consume those fixtures (`intent-witness-agent` `openwrt_fixtures`,
two passed in the 2026-09-16 workspace log) check parsers against committed
bytes, not a live VM.

### 4. Stock-ubus permission tests — recorded

Both pinned releases passed `check-witness-permissions.sh` on 2026-09-16 using
the stock native ubus CLI
([25.12.5 permissions log](verification-runs/openwrt-25.12.5-permissions.log),
[24.10.8 permissions log](verification-runs/openwrt-24.10.8-permissions.log),
[handoff review](verification-runs/handoff-review-2026-09-16.md)).

That establishes disposable-uid / ubusd ACL / rpcd session denial behavior on
those images. It does **not** establish execution of this repository's compiled
C/Rust adapter.

### 5. Native adapter execution — pending

- Host native observation helper tests passed (shell fixtures in
  `intent-witness-agent`; see [NATIVE-UBUS.md](../runtime/witness-agent/NATIVE-UBUS.md)
  and the 165-test workspace log).
- Linux `native-ubus` feature compilation and actual helper execution on
  OpenWrt are unverified ([goal progress](goal-progress.md),
  [NATIVE-UBUS.md](../runtime/witness-agent/NATIVE-UBUS.md)).
- The [native build experiment](../lab/openwrt/build-native/README.md) is
  incomplete. No runnable native artifact has been produced.

### 6. Physical acceptance — incomplete

There is no dedicated physical lab device and no enabled hardware profile
([GROK_HANDOFF.md](../GROK_HANDOFF.md), [goal progress](goal-progress.md),
[openwrt-lab.md](openwrt-lab.md)). Compiler artifacts have not been accepted on
physical hardware.

## Frontend harness

`frontend/` is a **fixture component harness** for isolated UI work. It awaits
Astra integration and is not the product application, a compiler bridge, or a
source of admission decisions. Commands live in
[frontend/README.md](../frontend/README.md).

G02 npm scripts (`npm ci`, `npm run check`, `npm test`, `npm run build`,
`npm run test:browser` from `frontend/`) are **not** Makefile targets. This
guide does not re-run those scripts or treat them as accepted product UI.
Coordinator results for this batch belong in the turn report.

## Contribution flow

Work on the broader suite is currently a **bounded Grok batch**, not an open
implementation of the [full objective](implementation-plan.md). Authorization
and task list: [GROK_HANDOFF.md](../GROK_HANDOFF.md). Review-gate status:
[REVIEW-GATES.md](grok/REVIEW-GATES.md) (Grok does not edit that file).

1. Implement only the authorized task packet.
2. Leave an **uncommitted** diff and a task report with **actual** check
   results (including skipped checks).
3. The user triggers **Astra review**
   ([Astra review prompt](grok/ASTRA-REVIEW.md)). Passing local tests does not
   open a review gate or mark the goal finished.

Do **not** resume the automatic goal runner. The
`resume-network-intent-assurance` heartbeat is paused
([goal progress](goal-progress.md)). Do not enable it, manage Codex
automations, or start another coordinator.
