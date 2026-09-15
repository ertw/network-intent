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

## Deliberately pending

The coordinator must add the guarded runtime: durable journal and boot
recovery, UCI stage/apply/confirm/rollback, independent witness verification,
the 180-second confirmation window, and device transport. Do not connect this
planner directly to a write-capable UCI client without those controls.

Validation: `cargo test -p intent-apply-agent` currently passes four planner
tests covering binding, drift, unowned preservation, scalar/list behavior,
deletion, secret rejection, duplicate identity rejection, and no-op plans.
