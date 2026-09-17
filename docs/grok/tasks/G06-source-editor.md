# G06 — Controlled CodeMirror source component

**Turn 003; read [CURRENT.md](../CURRENT.md).** Prerequisites are accepted in
`d527622` and `2774655`; start only on the user’s manual Cursor launch. This task is
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

## Resolved implementation boundaries (Astra validation)

The installed CodeMirror 6 supports `EditorState.lineSeparator`, `sliceDoc`,
readOnly and change/transaction filters. Astra verified CRLF roundtrip and an edit
using the installed package. No dependency or compiler work is required.

- Configure the exact uniform line separator and serialize via state.sliceDoc();
  do not use doc.toString() for CRLF output. A document without line breaks defaults
  to LF. On external replacement, update the separator configuration if necessary.
- Selection offsets refer to CodeMirror's UTF-16 document positions: each internal
  line boundary occupies one position even when source output uses CRLF. Test
  this distinction explicitly; do not change the frozen contract to source offsets.
- Mixed LF/CRLF or any standalone CR is unsupported in this packet. Show the
  original raw text in a read-only fallback with an explanation; do not normalize
  it or emit source/history edits. Resume normal editing on a supported external
  replacement. At most one live EditorView; dispose any inactive view correctly.
- Both the DOM editable setting and transaction-level checks must prevent user
  edits in view mode. Mark parent-driven replacement transactions internally so
  they remain allowed without echoing onChange. Do not expose the EditorView as
  a public bypass; test typing, cut/paste/drop and a user-edit command path.
- A user edit is a request. After callbacks and the parent's update, reconcile the
  document to the latest source prop even when the parent retains the old source.
  Accepted normal feedback preserves cursor/scroll; rejected changes roll back
  without a second source callback. Include an ignored-callback fixture/test.
- `selection=null` means no external selection override. Finite external offsets
  are truncated to integers and clamped to the internal document length. Non-finite
  selections are ignored. Never emit selection callbacks just because props change.
- A history shortcut emits exactly one request and prevents browser/CodeMirror
  history mutation. Do not install local history or implement a demo history stack.
  Document any platform-specific clipboard test technique truthfully.
