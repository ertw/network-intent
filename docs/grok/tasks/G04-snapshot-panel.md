# G04 — Read-only snapshot detail panel

**Turn 002 task. G02 was accepted and committed in `d527622`.**
Read [TURN-002.md](../TURN-002.md) for the current batch, toolchain and test coordination.
Start only when the user supplies the turn-002 prompt.

Own only `frontend/src/features/snapshot-panel/**`,
`frontend/tests/snapshot-panel.spec.ts`, and `docs/grok/reports/G04*`.
Export `SnapshotPanel.svelte` and `Demo.svelte`, using `SnapshotPanelProps`.
No shared-file edits, new dependencies, decoder, wire-type conversion or backend.

## Exact behavior

- Show title and snapshot ID (or "No snapshot"). Display these datastore labels
  distinctly: Committed configuration; Session staging; Configuration in use;
  Operational state. Never shorten them to the same "Applied" label.
- Display source, collected/received ISO timestamps exactly as supplied, freshness,
  completeness, missing items, and unavailability reason. Null timestamps display
  "Unknown". Do not compare timestamps with the clock or calculate freshness.
- Render groups and fields in supplied order. Scalars stay strings ("0", "false",
  empty strings, leading zeros and whitespace are not coerced). Show an empty
  string explicitly as an empty value, without changing it.
- Ordered lists show every entry in order, including duplicates. An empty list is
  displayed as "Empty list". Redacted fields show "Redacted" and have no value,
  reveal button, tooltip value or copied hidden text. Unknown fields show their
  supplied reason and cannot look like empty or redacted values.
- With no groups, display "No configuration details" or "No operational details"
  according to the datastore. If completeness is partial/unavailable, retain that
  warning even for an empty collection; absence is not proof of no configuration.
- Text must be escaped. Use a scrollable code/value area for long strings, not
  ellipsis that conceals the only available value. No edit, save, apply or retry.

## Steps and acceptance

Create synthetic demo cases covering all four datastores, complete/partial/
unavailable and fresh/stale/unknown provenance, scalar/list/redacted/unknown
fields, empty groups, long text and HTML-like text. The fixture labels explicitly
say they are synthetic. Do not copy credentials from real logs.

Browser tests verify labels and preserved order/duplicates; scalar empty string
versus empty list versus unknown versus redacted; absence of script/HTML execution;
external prop updates; no mutation controls; keyboard scrolling and readable
390x844 layout. Capture wide and narrow screenshots.

Run `npm run check` and `npx playwright test tests/snapshot-panel.spec.ts` from
frontend. Do not add state comparison, secrets heuristics, or health calculations.

## Harness integration rules

- Implement Svelte 5 typed props using the existing frozen contract; do not edit
  either contract copy. A Demo.svelte export is discovered automatically.
- Demo controls may select synthetic cases, replace props and show event counters.
  Keep those controls outside the exported component's production-facing UI.
- Test through the real browser route and preserve explicit synthetic labels.
  Tests must capture uncaught page errors and requests leaving the local origin;
  copy the small checking pattern into the owned test file if needed. Do not
  change shared test infrastructure or import another worker's unfinished files.
- Put helpers/unit tests inside the owned feature directory. Do not create shared
  field/provenance components in this batch; local duplication can be reviewed
  for extraction during later integration.
- Capture screenshots only under `docs/grok/reports/G04/`, at 1280x800 and
  390x844. Never overwrite G02 or another task's evidence. Coordinate all browser
  commands through the coordinator so only one owns port 4173 at a time.
- All original harness tests must still pass. The coordinator runs the full
  check/unit/build/browser suite after workers finish; passing only a focused
  test is not acceptance. Stop with actual results for Astra review.

Preserve visible whitespace in scalar/list values (for example `white-space:
pre-wrap`); HTML whitespace collapsing must not turn two spaces into one.
