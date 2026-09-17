# Grok 4.6 handoff — bounded implementation only

Read this file first. The user will run Grok from Cursor, may enable multiple
agents, and will manually ask Astra to review after each turn. **Do not implement
the whole Network Intent goal. Implement only the numbered tasks below.**

## Current baseline

- Branch prepared by Astra: `codex/network-intent-assurance`.
- Runtime foundation: commit `d33310c`; 165 Rust tests passed at that commit.
- This handoff commit additionally records reviewed lab tooling and successful
  nonroot permission acceptance on OpenWrt 25.12.5 and 24.10.8.
- G01/G02/G09 were accepted by Astra and committed as `d527622`. The Svelte
  component harness exists; later UI tasks build isolated components for Astra
  integration. Review evidence is in `docs/grok/reports/astra-turn-001.md`.
- Compiler admission still blocks the current router examples. Generated
  assurance coverage, real services, native adapter execution, guarded native
  deployment, full UI integration, telemetry, and hardware acceptance remain
  incomplete. No physical lab device is available.
- `docs/implementation-plan.md` describes the ultimate objective. It is context,
  **not authorization to expand this handoff**. This file narrows Grok's work.
- Automatic Codex implementation has been paused for this handoff. Do not enable
  it, manage Codex automations, or start another coordinator.

## Current handoff: turn 002 — G03, G04 and G05 only

Follow [TURN-002.md](docs/grok/TURN-002.md). Continue on `codex/grok-easy-tasks`
from the clean checkout containing this handoff update. Record the actual HEAD.
The user will start this batch manually in Cursor; this document does not start
an agent or automation. G01/G02/G09 are complete and must not be repeated.

Only G03/G04/G05 are authorized by the turn-002 starter prompt. Use up to three
workers with disjoint ownership, run the specified checks, write task reports and
`docs/grok/reports/turn-002.md`, then leave all changes uncommitted for manual
Astra review. Do not start G06/G07/G08 or change the frozen contracts.

If one task blocks, finish unrelated authorized tasks, report the exact blocker,
and stop. Do not invent a replacement task, weaken tests, or change a contract.
Do not treat a process observation timeout as process exit. Poll the same known
handle or inspect its authoritative state before restarting anything.

## Task map and later turns

| Task | Deliverable | Prerequisite |
|---|---|---|
| [G01](docs/grok/tasks/G01-fixture-index.md) | Deterministic index and checker for existing VM fixtures | None |
| [G02](docs/grok/tasks/G02-ui-harness.md) | Svelte/TypeScript component harness and browser tests | None |
| [G03](docs/grok/tasks/G03-view-controls.md) | Controlled view/layer/edit controls | Astra accepts G02 |
| [G04](docs/grok/tasks/G04-snapshot-panel.md) | Read-only configuration/operational detail panel | Astra accepts G02 |
| [G05](docs/grok/tasks/G05-assurance-panel.md) | Read-only assurance results panel | Astra accepts G02 |
| [G06](docs/grok/tasks/G06-source-editor.md) | Controlled CodeMirror source component | Astra accepts G02 |
| [G07](docs/grok/tasks/G07-graph-renderer.md) | Read-only Svelte Flow/ELK renderer for supplied graphs | Astra accepts G02 |
| [G08](docs/grok/tasks/G08-changes-panel.md) | Read-only display of supplied change entries | Astra accepts G02 |
| [G09](docs/grok/tasks/G09-documentation.md) | Accurate README status and command guide | None |

Later recommended batches: G03/G04/G05, then G06/G07/G08. Run one batch only
when the user explicitly starts that turn and `REVIEW-GATES.md` records the
required Astra acceptance. Passing your own tests does not open a review gate.
Astra may request a repair-only turn; that replaces the next batch.

## Frozen boundaries

- `docs/grok/contracts/presentation.ts` is the presentation-only contract. Copy
  it unchanged to `frontend/src/contracts/presentation.ts` in G02. It is not a
  wire schema, trusted compiler result, signed evidence, or admission mechanism.
- Do not edit `src/NetDSL/`, any existing `runtime/` source or test, compiler
  scripts, Cargo files, device profiles, deployment/identity/authorization code,
  SQLite migrations, existing fixtures, or verification transcripts.
- Do not operate QEMU, Docker, SSH, physical devices, native builds, PKI,
  enrollment, controllers, probes, monitors, deployments, or external services.
- Do not generate fake authoritative health/admission decisions, reinterpret
  wire `u64` identities as JS numbers, or copy mock data into production state.
- No backend endpoints, persistence, localStorage, IndexedDB, service workers,
  analytics, telemetry exporters, credential handling, or network fetches in UI
  components. Development fixtures must be conspicuously labelled as such.
- Dependency installation for G02 is allowed using the normal package registry;
  lock the resolved versions. Do not buy services, use paid APIs or credentials,
  install global tools, or edit user Cursor/Codex settings.
- Render strings as text, never HTML. Redacted fields stay redacted. Preserve
  provided ordering, state labels, timestamps, unknowns, and error distinctions.
- Do not fix a failure in a forbidden file. Report the smallest reproduction and
  affected invariant for Astra. No test skips, weakened assertions, fabricated
  logs, broadened filesystem grants, or undocumented substitutions.

### Reserved for Astra

Compiler semantics and realization witnesses; complete assurance generation and
admission; authoritative graph projection/layer filtering; state comparison and
health/freshness calculation; semantic Undo/Redo and source-preserving visual
edits; controller/service integration; PKI/Cedar/signed evidence; durable queues
and database changes; native cross compilation and ABI; timed apply/rollback;
telemetry instrumentation; packaging and hardware acceptance. A component task
must not drift into any of these areas.

## Combined checks and handback

First batch: G01's Python checks; G02's `npm ci`, `npm run check`, `npm test`,
`npm run build`, and `npm run test:browser` from `frontend/`. Inspect G09 links.
Later UI batches run those frontend checks once after all worker checks pass.
Use `git diff --check` and inspect `git status --short` for out-of-scope files.
Do not rerun the entire Idris/Rust/VM suite for presentation-only changes.

Every report states task IDs, base HEAD, exact changed files, commands with exit
codes, actual outcomes, limitations and screenshots where requested. Distinguish
"implemented; awaiting review" from "Astra accepted." A blocker report is useful;
an unexecuted plan or a claim based only on type-checking is not completion.

A manual [Astra review prompt and checklist](docs/grok/ASTRA-REVIEW.md) is included
for the handback. G06 and G07 are the more involved bounded tasks; the earlier
components are simpler. Their scope is fixed, but tests and review still matter.

## Suggested Cursor prompt

> Read GROK_HANDOFF.md and docs/grok/TURN-002.md. Execute only G03, G04 and G05
> using the existing reviewed harness and frozen contracts. Delegate within the
> specified ownership, run all required checks, write the turn-002 reports, leave
> changes uncommitted, and stop for my manual Astra review. Do not start later tasks.
