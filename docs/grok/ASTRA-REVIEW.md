# Fresh Astra reviewer — turn 003

This file is a self-contained review brief. No conversation history is required.
The user invokes it after Grok completes G06/G07/G08 in this checkout.

## Assignment and baseline

Review the current unstaged/uncommitted Grok batch against
[CURRENT.md](CURRENT.md), [G06](tasks/G06-source-editor.md),
[G07](tasks/G07-graph-renderer.md), [G08](tasks/G08-changes-panel.md), and
[the frozen contract](contracts/presentation.ts). Read only these plus
[review gates](REVIEW-GATES.md), relevant source/configs, and current reports
`reports/G06.md`, G07.md, G08.md and turn-003.md. Missing reports are findings;
implementation and executed checks matter more than the narrative.

Repository: Network Intent, an Idris-authoritative compiler with Rust runtime
foundations and a Svelte 5 component harness. This batch builds isolated source,
graph and changes widgets. It does not implement semantic editing/history,
compiler integration, graph projection, state diffing or admission. Native adapter,
deployment and physical acceptance remain incomplete. Do not resume the paused
goal runner, create agents for broader work or activate services/devices.

Accepted work through `2774655` is in main: G01/G02/G09 in `d527622`, G03/G04/G05
in `2774655`. Prior review recorded 43 unit and 26 Chromium tests plus build/type
checks. A subsequent preparation commit updates docs and routes old screenshots
to ignored outputs. Grok must report that actual starting HEAD, not merely
`2774655`. Verify it against git history and branch state. Expected work branch:
`codex/grok-easy-tasks`. Do not reset/stash/discard changes or commit anything.

## Review procedure

1. Inspect `git status`, staged diff, tracked diff and every new file. `git diff`
   alone misses untracked files. Check task ownership against CURRENT.md, and
   identify baseline drift/unrelated edits before attributing them to Grok.
2. Verify both presentation.ts copies are unchanged from starting HEAD and equal
   each other. Inspect source and tests, not just reports. No new dependency,
   authority, persistence, external request, shared infrastructure or old evidence
   modification is allowed by these task packets.
3. Use supported Node `^20.19.0 || ^22.12.0 || >=24.0.0`, npm >=10. On the original
   host the existing Node 24 bin is:
   `/Users/erikwilliamson/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin`.
   Prepend it to the command PATH if needed; do not change user settings or install
   global tools. Run clean `npm ci` from frontend, without forced peers.
4. Serialize all browser runs on port 4173. Run `npm run check`, `npm test`,
   `npm run build`, and `npm run test:browser` yourself. Inspect relevant real
   Chromium screenshots at 1280x800 and 390x844. Preserve old committed screenshots.
   New task evidence capture uses the explicit option documented by each spec.
   Record exact commands, working directories, exit codes and actual tool versions.
5. Add focused regressions for defects you find. Fix small issues inside the
   authorized task paths; report substantive redesign, missing contract information
   or forbidden edits instead of expanding scope. Do not skip/weaken assertions.
   Rerun only checks affected by repairs, plus the final combined suite as needed.
6. Write `reports/astra-turn-003.md` and a file-hash manifest of the exact reviewed
   content (include new files and executable bits). Exclude only the manifest,
   review record and review gates to avoid self-reference. Record base HEAD and
   reproducible verification instructions. Preserve Grok reports as their own
   historical claims; clarify differences in the Astra report.
7. Update REVIEW-GATES.md: Accepted / Changes requested / Blocked on design, tied
   to the exact reviewed state. State whether the batch is ready to commit.
   **Leave all changes unstaged/uncommitted.** No automatic next batch exists.

## Critical checks

- **G06:** exact LF/CRLF roundtrip and edits; internal CodeMirror versus serialized
  offsets; tabs/Unicode/trailing newline; mixed separators/standalone CR fallback;
  no echoed external updates; accepted and rejected parent changes; all user-edit
  paths blocked in view mode while parent replacement still works; null/external
  selection behavior; exactly one history request with no local mutation; cleanup.
  DOM contenteditable=false alone is insufficient to block programmatic edit paths.
- **G07:** actual ELK + Svelte Flow rendered edges, finite positions, cycles,
  parallel/disconnected cases; no caller mutation; ignored parent selection;
  keyboard/canvas selection, pan/zoom; all mutation gestures disabled; latest
  input wins across reverse resolution, failure, empty/invalid replacement and
  unmount; no old graph shown as current; full labels inspectable, unique IDs.
- **G08:** supplied order and kinds, controlled selection including removal and
  ignored requests, empty scalar/list/side versus unknown/redacted, exact visible
  whitespace and duplicates, escaped text, inspectable long values, no decisions
  or mutation controls. It must not calculate its own diff.
- All components stay synthetic/labelled in demos. No semantic Undo/Redo, graph
  mapping, health calculation or admission decision may be invented.

## Final response to the user

Lead with accepted/changes requested and commit readiness. Summarize defects
fixed or remaining, actual check results, evidence limits, and links to the review
and gates. Do not claim full system completion. If tool approval fails, explain
what remains unverified; a tool failure is not a test pass.
