# G06 — Controlled CodeMirror source component

**Requires Astra acceptance of G02 and an explicit new user turn.** This task is
an editor widget, not DSL interpretation, admission, preview, semantic history or
source-preserving visual editing.

Own only `frontend/src/features/source-editor/**`,
`frontend/tests/source-editor.spec.ts`, and `docs/grok/reports/G06*`.
Export `SourceEditor.svelte` and `Demo.svelte` using `SourceEditorProps` and the
CodeMirror 6 dependencies installed by G02. No extra dependency or shared edit.

## Exact behavior

- Create one CodeMirror EditorView on mount, update via transactions/compartments,
  and destroy it on unmount. Do not recreate it for every keystroke or prop change.
- Local typing/paste/deletion emits the complete next source via `onChange`.
  External source replacement updates the document without echoing onChange.
  Normal controlled-value feedback must not reset the cursor or scroll position.
- `editable=false` prevents typing, paste, cut, drop and programmatic *user edit*
  commands through the component. Selection, scrolling and copying remain usable.
  Switching back to edit permits changes. External prop replacement remains allowed
  in either mode because it comes from the parent.
- Emit selection as UTF-16 editor offsets. Accept valid external selections;
  clamp out-of-range endpoints to document length. Do not reinterpret those offsets
  as compiler byte offsets or source spans. Avoid callback loops on prop updates.
- Preserve source text: no formatting, trimming, tab conversion or appended newline.
  Test uniform LF and CRLF documents and documents with/without a final newline.
  Mixed line separators are outside this widget packet: detect them and display a
  read-only explanation rather than silently normalize the source. Report the
  limitation for Astra's full editor integration.
- Do not enable CodeMirror's local history extension. Intercept standard Mod-Z,
  Mod-Shift-Z and Ctrl-Y and emit `onHistoryRequest('undo'|'redo')` only in edit
  mode; the component must not change text itself for history. Shared semantic
  Undo/Redo is reserved for Astra. In view mode these shortcuts emit nothing.
- Use plain-text editing with line numbers, wrapping toggle unnecessary. No DSL
  parser, highlighting grammar, compiler worker, autocomplete or diagnostics.
- No file I/O, autosave, localStorage, IndexedDB or network calls.

## Demo and tests

The demo parent shows the current in-memory text and event counters, supports an
external reset, and switches view/edit. Label history requests as fixture events;
do not implement a history stack. Exercise Unicode (including surrogate pairs),
comments, quoted strings, tabs, LF/CRLF, final newline and mixed newline rejection.

Real browser tests cover typing/paste/cut prevention in view mode, normal changes
in edit mode, selection offsets, external updates without feedback, history
callbacks without text mutation, and repeated mount/unmount without duplicate
callbacks. If clipboard permissions are unavailable use the browser clipboard
fixture/dispatch with an actual cancelable paste event; disclose that test method.
Capture readable wide/narrow screenshots. Run `npm run check` and
`npx playwright test tests/source-editor.spec.ts` from frontend, plus focused unit
checks for newline/selection helpers if used. Stop for Astra on any contract gap.
