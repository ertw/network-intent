# Apply agent planning boundary

This package is intentionally separate from the witness agent. It currently
contains only explicit adoption and pure three-way UCI planning; its binary
does not contact a device or execute UCI, a shell, or HTTP.

## Implemented API

- `adopt(device, revision, baseline, owned_fields)` records an immutable
  baseline digest and exact owned section/field paths.
- `plan(ownership, binding, baseline, current, desired)` checks exact device,
  profile/fencing, the adopted baseline revision, baseline digest, ambiguous
  identities, drift, and unsupported secret materialization. `PlanBinding`
  also carries the desired revision, which the resulting plan binds for a
  future guarded apply.
- Output starts from `current`, retaining unowned sections, fields, and list
  order. Desired unowned field changes and all unowned section creation,
  deletion, type changes, or reordering are blocked. Owned scalar/list
  deletion is explicit through a missing desired owned field; field ownership
  does not authorize section creation/deletion/reordering. Drift never
  produces a partially-applicable plan.

## Guarded deployment journal

`journal::DeploymentJournal` persists `Prepare → Preflight → Stage → Durable
checkpoint → Provisional apply → Independent verification → Confirm or
rollback` in local SQLite using WAL, foreign keys, FULL synchronous commits,
and serialized transitions. It stores owned baseline field/value digests and
secret-store references, never secret values. The checkpoint is committed
before the apply-only backend is called, then an intent marker is committed
before that irreversible call.

The small `ApplyBackend` trait exposes preflight, staging, durable checkpoint,
provisional apply, confirmation, rollback, and read-only reconciliation.
Backends must make their writes idempotent by deployment ID. Before the first
live effect, the journal passes a persisted `ApplyWindow`; the adapter must arm
its device timer using the remaining interval and refuse an expired interval.
Pre-apply abort/recovery calls the separate idempotent `discard_staging`
operation, which must never write live configuration. A failed or
interrupted apply is never re-applied; recovery reconciles it and can restore
the durable checkpoint. A lost confirmation reply remains fenced until
read-only reconciliation determines whether it was confirmed. The default
confirmation window is 180 seconds. Confirmation accepts only
`intent_identity::confirmation::VerifiedDeploymentConfirmation`, which has
already checked two ordered independent evidence rounds including a LAN
management-path probe. The journal rechecks deployment/checkpoint, device,
revision, plan, epoch, graph version, and deadline locally.

There is still no native UCI transport or production write path. A future
adapter must implement `ApplyBackend`; it cannot bypass the journal.

The journal advances caller-supplied `now_ms` by monotonic elapsed time after
the operation lock is acquired. Before confirmation, it rechecks the sealed
authorization and supplies the backend its exclusive expiry; the backend must
also refuse expiry reached while it waits or before its physical confirmation.

Validation: `cargo test -p intent-apply-agent` currently passes four planner
tests covering binding, drift, unowned preservation, scalar/list behavior,
deletion, secret rejection, duplicate identity rejection, and no-op plans.
