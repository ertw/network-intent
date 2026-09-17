# G03 — Controlled layer, state and edit controls

**Requires Astra acceptance of G02 and an explicit new user turn.**

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
