# Native ubus build experiment — incomplete

This directory preserves the native smoke harness and a **non-working build
scaffold** for Astra to finish. It has not produced a runnable executable and
must not be used as evidence of native ABI or runtime acceptance. It is outside
all Grok handoff tasks.

The official 25.12.5 SDK archive passed its pinned SHA-256 check. Its GCC 14.3
cross compiler ran, and an exploratory build produced a target `libjson-c.a`.
The stock SDK does not contain the target libubus/libubox development headers
and link libraries assumed by the scaffold. Extracted VM shared libraries have
stripped section headers and were rejected as linker inputs. Subsequent libubox
configuration selected an unusable SDK pkg-config wrapper; an attempted package
installation in the build container failed. No native build job is running at
handoff preparation.

`build-native build VERSION` now checks for the missing development headers and
fails explicitly before touching previous output. Completing dependency staging,
source/ABI provenance, cross linking, and the Rust helper build is reserved for
Astra. Do not fabricate linker stubs or weaken compiler checks to make it pass.
Ignored experimental downloads, sources and logs are under
`lab/openwrt/state/native-build/`; they are not portable dependencies.

## Smoke harness contract

`intent_ubus_smoke.c` calls the production read-only C shim. Once correctly built,
`--self-test` must reject `uci.set`, read `uci.get network` on the actual ubus
socket, and reject the same read with a one-byte response budget (status 7001).
It must subsequently be run in each pinned VM; a host C syntax check proves none
of those runtime behaviors. This harness also does not test the supervising Rust
helper's deadline or identity enforcement.

The scaffold pins SDK hashes and an amd64 Debian container. Its final compile
runs without network, with dropped capabilities, and with source/SDK mounted
read-only. The currently documented build command is a diagnostic of incomplete
staging, **not a supported build workflow**.
