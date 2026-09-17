# G03 — Controlled layer, state and edit controls

**Turn 002 task. G02 was accepted and committed in `d527622`.**
Read [TURN-002.md](../TURN-002.md) for the current batch, toolchain and test coordination.
Start only when the user supplies the turn-002 prompt.

Own only `frontend/src/features/view-controls/**`,
`frontend/tests/view-controls.spec.ts`, and `docs/grok/reports/G03*`.
Use `ViewControlsProps` from the frozen contract. Export `ViewControls.svelte`
and `Demo.svelte`; no shared files or new dependencies.

## Exact behavior

1. Render primary layer choices L1/L2/L3/L4/L7, in that order. No L5/L6.
2. An expandable Advanced area exposes independent overlay checkboxes and the
   combined overview switch. Overlays exclude the current primary layer and are
   emitted in canonical layer order, without duplicates.
3. Selecting a different primary layer preserves other selections and removes
   that new primary from overlays. It must not add the previous primary as an
   overlay automatically.
4. Combined overview disables primary/overlay editing while retaining their
   values; disabling it restores the retained choices.
5. State selection is independently Intent / Device configuration / Operational /
   Compare. Assurance overlay and View/Edit are independent switches.
6. All changes call `onChange` with a new complete value. Do not mutate the input,
   retain hidden domain state, perform filtering, or fetch anything. Rendering new
   props updates the controls without causing a callback loop.
7. View/Edit only requests a mode change. It does not authorize any compiler,
   graph, source, adoption or deployment operation.

Use labelled native inputs/buttons with visible focus and accurate checked/pressed
state. Keep advanced controls collapsed initially. The component must fit a
390-pixel viewport without horizontal scrolling.

## Demo and tests

The demo owns the controlled value and displays emitted events as development
fixture data. Tests operate actual controls with pointer and keyboard and assert:
all five layer labels; no L5/L6; overlay exclusion/order; combined retain/restore;
independence of state/mode/assurance selections; no input mutation; external prop
update without spurious callbacks; narrow-screen readability.

Run `npm run check`, focused unit tests if added, and
`npx playwright test tests/view-controls.spec.ts` from frontend. Capture wide and
narrow screenshots. Report and stop; no graph filtering or product integration.

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
- Capture screenshots only under `docs/grok/reports/G03/`, at 1280x800 and
  390x844. Never overwrite G02 or another task's evidence. Coordinate all browser
  commands through the coordinator so only one owns port 4173 at a time.
- All original harness tests must still pass. The coordinator runs the full
  check/unit/build/browser suite after workers finish; passing only a focused
  test is not acceptance. Stop with actual results for Astra review.
