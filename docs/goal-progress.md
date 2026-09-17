# Network Intent — current checkpoint

Updated: 2026-09-16. The full objective remains incomplete.

## Current work arrangement

The user requested a reviewed, committed baseline and detailed instructions for
Grok 4.6 in Cursor. [GROK_HANDOFF.md](../GROK_HANDOFF.md) limits that work to nine
component, fixture-index and documentation tasks. The first batch is G01, G02
and G09. Subsequent batches require an explicit user turn and recorded Astra
review. Grok leaves its changes uncommitted for the user's manual Astra review.

The `resume-network-intent-assurance` heartbeat is **paused** to prevent automatic
implementation while Grok owns the checkout. Do not resume it implicitly. The
[full implementation plan](implementation-plan.md) remains the eventual objective;
it does not expand the current Grok authorization. No credits have been purchased
or reset credits redeemed. The user has no dedicated physical lab device.

## Committed runtime foundation

Branch prepared for handoff: `codex/network-intent-assurance`.

- `71387b0`: compiler/runtime foundation; full `make test`, eight native/browser
  parity fixtures and 102 Rust workspace tests passed at that baseline.
- `d33310c`: durable deployment confirmation, witness queue, guarded apply journal
  and native observation boundary hardening. **165 Rust workspace tests passed**;
  see [integration log](verification-runs/runtime-integration-2026-09-16.log).
- Idris evaluates models, targets, realization witnesses and diagnostics. Rust
  admission uses sealed compiler results and checks coverage, bindings, profile
  freshness and dependency graphs. **Current router examples remain admission
  blocked** because semantic and operational assurance coverage is incomplete.
- Protocol state preserves intended, configured, staging, configuration-in-use
  and operational distinctions. Identity/evidence foundations include Ed25519
  DSSE, PKI, Rustls mTLS and Cedar authorization; these are not complete services.
- Durable storage, evidence ingress and monitoring exist. The witness queue uses
  assignment fences, leases, immutable receipts and an atomic outbox. Deployment
  confirmation requires two fresh, complete, ordered signed rounds including a
  trusted LAN management contract.
- The guarded journal serializes backend effects across processes, permits one
  unfinished deployment per device, persists confirmation intent/proof, accounts
  for lock wait in deadlines and reconciles uncertain effects without blind
  retries. A real native UCI apply backend and service are still pending.
- Native observation uses a fixed-path helper, bounded private IPC, deadlines,
  kill/reap and four execution slots. Host checks passed. Linux native feature
  compilation and actual execution remain unverified.

## OpenWrt lab evidence

Both pinned x86/64 images (25.12.5 and 24.10.8) passed SHA-256 and release-signature
verification and booted in isolated QEMU. Baseline transcripts and parsed fixtures
are committed. Wireless capability metadata distinguishes an absent object on
25.12.5 from an empty response on 24.10.8.

Both releases also passed the disposable nonroot permission checks:

- [25.12.5 transcript](verification-runs/openwrt-25.12.5-permissions.log)
- [24.10.8 transcript](verification-runs/openwrt-24.10.8-permissions.log)

These checks establish actual uid switching, allowed reads, denied mutation calls,
read-only session permissions and denied reads after session revocation. They use
the stock ubus CLI; **they do not establish execution of our native C/Rust adapter**.
See [lab instructions and evidence limits](openwrt-lab.md).

The [native build experiment](../lab/openwrt/build-native/README.md) remains
incomplete. Its pinned SDK lacks the assumed libubus/libubox development staging;
the scaffold now rejects that condition before replacing output. No runnable
native artifact has been produced. Experimental downloads and SDK state are
ignored, local and not portable dependencies. No lab VM or native build container
was running at handoff preparation.

## Queued work and remaining scope

No frontend exists at this checkpoint. Grok's first batch creates an isolated
Svelte component harness, a deterministic index of existing fixtures, and accurate
developer documentation. All nine tasks are pending; none is accepted merely
because its task packet exists. [Review gates](grok/REVIEW-GATES.md) record status.

Astra retains compiler semantics and complete assurance generation, authoritative
state/graph projection and comparison, semantic editing/history, services and
transport, identity/authorization, durable state, native cross compilation and
execution, timed apply/rollback and recovery, telemetry, packaging and hardware
acceptance. No physical hardware profile is enabled. The next step is the bounded
Grok batch, followed by the user's manual Astra review.

Earlier chronology is archived in
[2026-09-15 progress history](progress-history/2026-09-15.md); its old process IDs,
usage snapshots and resumption instructions are historical, not current actions.
