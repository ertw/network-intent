# Goal Mode Plan: Network Intent and Continuous Assurance

## 1. Goal and activation

**Current execution status (2026-09-17):** the full objective remains incomplete.
Implementation proceeds through manually launched, scoped Grok batches and manual
Astra review. The continuation heartbeat is paused. This specification does not
authorize a batch agent to start goals, automations, services or deployment.
See [current checkpoint](goal-progress.md) and [current batch](grok/CURRENT.md).

The original startup/activation checklist has been completed or superseded by
this manual workflow; its historical wording is available in Git at `2774655`.
The technical objective and acceptance scope below remain in force. Missing
hardware validation remains an explicit incomplete acceptance item.

## 2. Technical implementation contract

### Application and state model

Build a Svelte/TypeScript application with Svelte Flow, ELK.js, and CodeMirror. Idris remains authoritative for language semantics, validation, target compilation, and witness checking. Rust handles observation, planning, persistence, and execution.

Provide:

- **L1/L2/L3/L4/L7** views, with one primary layer, optional overlays, and an advanced combined overview. No L5/L6 entries.
- Independent **Intent / Device configuration / Operational / Compare** selection and optional assurance overlays.
- A simple View/Edit toggle, with human AAA deferred.
- Minimal viewing controls and selection-driven detail.
- Resizable **Source / Changes / Assurance** editing panes.
- Live valid-source previews, source-preserving visual edits, shared semantic Undo/Redo, and protection against stale worker results.
- In-memory unsaved browser drafts; explicitly accepted revisions and operational history stored in SQLite.

Apply the datastore, schema, comparison, and capability lessons from RFCs **8342, 9144, 7950, 6241, 8526, and 8525**. Keep configuration readback, configuration demonstrably in use, operational state, apply receipts, and independent observations distinct.

Introduce versioned interfaces for intent revisions, device profiles, ownership, snapshots, witnesses, assurance plans, deployment plans, signed evidence, and health assessments. Preserve source mappings, collection scope, completeness, origin, freshness, and order-sensitive configuration.

Do not worry about backwards compatibily since this has not yet been deployed. Fee free to make breaking changes.

### Witnesses and assurance

Accept semantic visual changes only when:

1. Idris validates the candidate.
2. Every affected target compiles.
3. Its realization witness passes independent checking.
4. The planner produces complete, concrete assurance coverage.
5. The plan checker validates dependencies, bindings, primitive support, and coverage.

Apply the same gate to promotion and deployment of DSL changes. Unsupported changes may remain drafts but cannot bypass admission.

Implement these trusted Rust primitives:

- TCP connect.
- UDP DNS.
- TCP DNS.
- HTTP(S).
- ICMP.
- OpenWrt UCI readback.
- OpenWrt netifd operational state.

Bind every probe to explicit endpoints, source locations, IP families, expectations, deadlines, and resource limits. Preserve protocol-specific failure meanings; a timeout cannot establish universal firewall isolation.

Start with supported interface, addressing, route, bridge, attachment, DNS, and explicitly bound service contracts. Block unsupported claims with actionable coverage explanations.

Generated plans continue as persistent monitors after deployment. Follow RFCs **9417/9418** for assurance dependencies, distributed observations, graph versions, and symptom history.

Defaults remain:

- netifd: 15 seconds.
- Active probes: 30 seconds.
- UCI readback: 60 seconds.
- Heartbeat: 10 seconds.
- Freshness: three intervals plus invocation timeout.
- Incident opening: three consecutive violations.
- Recovery: two successes.

Unknown or stale evidence never counts as success.

### Separate runtime agents

Implement distinct Rust controller, witness-agent, and apply-agent packages.

The witness and apply agents have separate binaries, services, identities, permissions, databases, and execution paths. Share protocol schemas and fixtures, not a common interchangeable agent runtime.

The OpenWrt witness uses native ubus integration and a read-only session to collect supported UCI, interface, device, and netifd wireless information. Distinguish committed configuration from session staging. Redact sensitive fields before export, persistence, or telemetry.

The apply agent requires explicit configuration adoption and performs:

**Prepare → Preflight → Stage → Durable recovery checkpoint → Provisional apply → Independent verification → Confirm or rollback.**

Use three-way changes, preserve unowned configuration, reject conflicts, and bind operations to exact revisions and device capabilities. Use timed UCI apply/confirm/rollback with a durable local journal and restart/boot recovery.

The default confirmation window is 180 seconds with two successful verification rounds. Require independent witness evidence, including management-path verification from a LAN witness. Multi-device changes require a safe sequential order with valid intermediate states.

Preserve existing secret values initially; unsupported credential changes remain blocked.

### Identity, reliability, and storage

Implement built-in PKI with separate identities, short-lived certificates, enrollment, renewal, revocation, and trust rotation. Use SPIFFE-compatible identity semantics without requiring SPIRE.

Use mutual TLS, signed assignments, and DSSE/in-toto evidence statements. Use Cedar for controller authorization and local scope enforcement in each runtime agent.

Implement durable inbox/outbox delivery:

- Immutable message IDs and payload digests.
- Acknowledge after durable storage.
- Deduplicate identical retries.
- Reject conflicting reuse of IDs.
- Enforce device fencing generations and plan epochs.
- Reconcile uncertain apply operations before retrying.

Provide at-least-once delivery with idempotent processing; do not claim exactly-once physical execution.

Use SQLite with foreign keys, local-filesystem WAL, serialized writes, migrations, bounded queues, and online backups. Retain detailed observations for 30 days and deployment/audit/incident evidence for one year, preserving referenced baselines.

Instrument the controller and both runtime agents with OpenTelemetry. Observe scheduling, probe execution, queues, certificates, database contention, delivery failures, recovery, and telemetry loss.

Use the researched lessons from Prometheus blackbox exporter, OpenTelemetry Collector, Batfish, HSA/VeriFlow, NetKAT, SPIFFE/SPIRE, in-toto, and Cedar. Do not substitute sampled probes for symbolic proofs or require those systems as additional deployed services unless specified above.

## 3. Codex coordinator and subagents

These are **Codex implementation subagents**, distinct from the Rust agents being built.

| Role | Model and effort | Responsibilities |
|---|---|---|
| **Coordinator** | `gpt-6-astra`, high; xhigh for difficult reviews | Architecture, contracts, Idris assurance boundaries, security decisions, delegation, integration, and final acceptance. |
| **Implementation subagent** | `gpt-5.6-terra`, medium | Bounded Rust modules, frontend components, adapters, and substantive tests under agreed interfaces. |
| **Lower-cost subagent** | `gpt-5.6-luna`, medium | Fixtures, documentation, straightforward UI work, schema plumbing, focused test additions, and narrow investigations. |
| **Escalated specialist** | `gpt-6-astra`, high | Specific unresolved correctness or security problems that exceed the cheaper worker’s scope. |

This allocation follows the documented flagship, balanced, and cost-sensitive model tiers. Actual subscription allowance consumption must be read from the account tools. [OpenAI model guidance](https://developers.openai.com/api/docs/models).

### Delegation rules

- Use at most **three concurrent subagents plus the coordinator**, matching current capacity.
- Default substantial implementation to Terra and narrow mechanical work to Luna.
- Escalate after a concrete failed attempt or an identified reasoning risk; avoid repeated cheap attempts that increase total work.
- Give each subagent a bounded deliverable, file ownership, fixed interfaces, required tests, and explicit completion criteria.
- Supply a concise task packet instead of the complete conversation when selecting an alternate model.
- Avoid simultaneous edits to shared schemas or overlapping files. The coordinator owns cross-module contracts.
- Have subagents report changed files, validation results, limitations, and follow-up requirements.
- Review every contribution. The coordinator personally reviews witness admission, authorization, signature verification, idempotency, and rollback logic.
- Reuse existing subagents where useful and avoid duplicate investigations or repeated full test suites.

### Work allocation

After the coordinator establishes contracts, delegate independent work across:

1. Rust witness primitives and ubus observation.
2. Svelte/ELK/CodeMirror visualization and editing.
3. Controller persistence, protocol fixtures, and integration testing.

Introduce the apply-agent workstream after the witness/evidence interfaces are stable. Reserve a worker slot for independent failure testing during deployment integration.

## 4. Checkpoints and automatic resumption

### Durable progress

Update `docs/goal-progress.md` after each completed subtask or milestone and before long-running verification.

Record:

- Goal objective and accepted specification revision.
- Completed, active, and pending work.
- Current branch and relevant repository state.
- Subagent assignments and results.
- Exact test commands and outcomes.
- Unresolved issues and required external inputs.
- The next concrete action.
- Usage-limit status and the next known eligible resume time.

Do not depend on surviving subagent sessions or conversation memory. After interruption, inspect the repository and checkpoint before redispatching work.

Check usage at milestone boundaries. As allowance becomes low, checkpoint more frequently and avoid launching work that cannot be handed off cleanly. Do not mark the goal complete because a usage window is ending.

### Resume heartbeat

When execution starts, create one active, same-task heartbeat named:

**Resume Network Intent Assurance**

Schedule it **every 15 minutes**. Before creating it, check for an existing matching heartbeat for this task and update that entry instead of duplicating it.

Use this saved prompt:

> Follow up on this task’s Network Intent implementation goal. Read the goal, docs/implementation-plan.md, docs/goal-progress.md, current repository state, and live Codex usage limits. Respect explicit user pauses, Plan Mode, and unresolved approval requirements. If the goal is complete, disable this heartbeat. If work is already progressing, do not duplicate assignments or start another coordinator. If work stopped because of usage limits, resume when every applicable exhausted limit has reset and allowance is available. Continue from the recorded next action using GPT-6 Astra as coordinator, GPT-5.6 Terra for substantial bounded implementation, and GPT-5.6 Luna for suitable lower-cost subtasks. Recreate interrupted subagents only after checking their existing work. If allowance remains exhausted, record the next eligible reset and wait for a later scheduled check. Do not purchase credits or redeem reset credits. Stay quiet while nothing actionable has changed; notify on meaningful resumed progress, completion, failure, or required user action. Preserve checkpoints before further interruption.

Check **both five-hour and weekly limits**, plus any other reported blocking window. The current snapshot showed 56% five-hour allowance remaining and 12% weekly allowance remaining; future runs must obtain fresh values.

Resumption is best-effort on the first eligible scheduled run, rather than an exact reset-time guarantee. The local execution host and app must remain available; scheduling does not bypass exhausted account limits. [Scheduled-task requirements](https://learn.chatgpt.com/docs/automations?surface=app).

## 5. Milestones and completion criteria

Execute in this order:

1. **Goal setup and contracts:** persist the specification/checkpoint, activate goal and heartbeat, define state/protocol types, and implement witness/coverage checking.
2. **Observation:** controller/SQLite, enrollment, Rust witness agent, all seven primitives, and real ubus reads.
3. **Continuous assurance:** generated plans, scheduling, signed idempotent delivery, health evaluation, history, and OpenTelemetry.
4. **Deployment:** separate apply agent, adoption, conflict handling, independent confirmation, rollback, and crash/reboot recovery.
5. **Visualizer integration:** practical layers, state comparison, coverage-driven editing, DSL synchronization, and deployment review.
6. **Acceptance and hardening:** full integration, failure injection, documentation, packaging, and supported-device validation.

Required acceptance includes:

- Existing `make test` checks and native/browser compiler parity.
- Witness forgery, uncovered claims, stale profiles, and missing bindings rejected.
- All primitives tested for success, failure, ambiguity, malformed responses, and timeouts.
- Correct UCI staging/readback, ordering, ownership, and redaction.
- Enforced witness/apply permission and identity separation.
- Signature, enrollment, certificate, replay, duplicate, fencing, and out-of-order delivery tests.
- Crash injection at every deployment boundary and verification of recovery.
- Monitoring across browser closure, process restarts, and controller outages.
- View-mode mutation prevention, source round trips, stale-response handling, and Undo/Redo.
- Real OpenWrt VM integration using pinned 25.12.5 and 24.10.8 lab images, followed by hardware validation before enabling corresponding hardware profiles.
- No false assurance from stale, incomplete, unsupported, or unavailable evidence.

After all required checks pass, the coordinator marks the goal complete, disables the resume heartbeat, and reports implementation results, verification evidence, and remaining documented limitations.


## Execution activation record

Execution authorized by the attached user request on 2026-09-14 America/Los_Angeles. The Plan Mode activation note above is historical; this task is now executing in Default mode. This specification supersedes earlier MVP non-goals where they conflict. Specification revision: assurance-suite-v1. No token budget was requested.
