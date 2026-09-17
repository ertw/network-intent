# G08 — Read-only supplied changes panel

**Turn 003; read [CURRENT.md](../CURRENT.md).** Prerequisites are accepted in
`d527622` and `2774655`; start only on the user’s manual Cursor launch. Display an
existing list; do not calculate textual/semantic/three-way differences.

Own only `frontend/src/features/changes-panel/**`,
`frontend/tests/changes-panel.spec.ts`, and `docs/grok/reports/G08*`.
Export `ChangesPanel.svelte` and `Demo.svelte` with `ChangesPanelProps`.
Use only dependencies installed by G02. No cross-feature or shared-file edits.

## Exact behavior

- Render entries in supplied order with labels Added / Removed / Modified /
  Blocked. Show their provided explanations. Do not infer the kind from values.
- Selection calls `onSelect(id)` and is controlled by `selectedId`. Display Before
  and After field groups for the selected entry; retain each field/list order and
  duplicate list values. Missing sides are visibly empty, not "unknown".
- Scalars, ordered lists, redacted fields and unknown fields use the same display
  rules in the frozen contract: no coercion, secret reveal or hidden raw values.
- Blocked is an input display status, not a decision made by this component. There
  is no approve, accept, apply, adopt, merge, conflict resolution or undo button.
- Empty input shows "No changes supplied". Invalid selected ID has no fabricated
  detail. Long values remain inspectable; text is escaped; controls are keyboard
  accessible; before/after stack vertically at narrow widths.

## Steps / acceptance

Create synthetic cases for all four kinds, an empty list, empty scalar/list,
redacted and unknown fields, duplicate ordered-list entries, long text and an
explanation containing HTML-like characters. Test prop updates, selection callbacks,
ordering, empty states, literal text and lack of mutation controls in the browser.
Do not implement a diff algorithm to produce the fixture cases.

Run `npm run check` and `npx playwright test tests/changes-panel.spec.ts` from
frontend. Capture wide/narrow screenshots and report. Stop for Astra before adding
anything that decides whether a change is safe, supported, admitted or deployable.

## Resolved display and review rules

This is the simplest remaining task and needs no new algorithm or dependency.
Use local rendering code; do not import/edit SnapshotPanel or extract shared UI.

- Show a visible Empty value label for empty scalars while leaving the value text
  itself empty. Show Empty list separately. Empty Before/After arrays mean No
  fields supplied; an unknown field displays Unknown plus its supplied reason.
- Preserve visible whitespace using pre-wrap or equivalent. Ordered list entries,
  including empty entries and duplicates, remain separate. Null explanation means
  no explanation supplied; it is not proof that a blocked change is safe.
- Parent-retained selectedId must leave selection unchanged after onSelect. Null
  or an absent selected ID means no details selected, not an invented default row.
- Test accepted/ignored selection, external replacement/removal of a selected
  entry, exact whitespace, long values, empty distinctions and absence of mutation
  controls. If scroll handlers are needed, leave modified navigation keys alone.
