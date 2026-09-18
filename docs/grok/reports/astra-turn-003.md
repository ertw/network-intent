# Astra review — turn 003

**Accepted after in-scope repairs. G06 / G07 / G08 are ready to commit as the
reviewed presentation-only batch.** No remaining blocking finding or frozen-contract
gap was identified. All work remains unstaged and uncommitted. No next batch,
automation, compiler integration or runtime acceptance is authorized by this review.

Review date: 2026-09-17. Repository and command working directory:
`/Users/erikwilliamson/Documents/network-intent`.
Base HEAD: `a4baeaa8f8818eddef6e913f0e52bef6df4aba90`.
Branch: `codex/grok-easy-tasks`; local `main` is the same HEAD.
Both `2774655` and Nix baseline `bd9da11` are ancestors. Grok reported the actual
preparation HEAD correctly. Its historical claim that the checkout was clean when
it began cannot be reconstructed from the current worktree; the present diff and
history are consistent with it.

## Scope and contract checks

Read the review brief, CURRENT, G06/G07/G08 packets, presentation contract,
review gates, all four current Grok reports, all new implementation/test source,
and relevant frontend/Nix configuration and installed Flow event handling.
Inspected tracked, staged and untracked inventory, including PNG evidence.
At review entry the tracked and staged diffs were empty; 45 untracked files were
all within G06/G07/G08 ownership plus the turn-003 coordinator report. No unrelated
baseline drift was found. The reviewer added only in-scope code/tests/evidence,
this record, the manifest and the explicitly authorized gate update.

Both contract copies compare equal and have no difference from HEAD:
`3a4e4db2e959c6e1b74adc63b7384d9191af56aa80aee10da4931b4c70c0bb85` (SHA-256).
Package/lockfiles, flake/lockfiles, App.svelte, shared styles, previous tests,
previous reports/evidence and compiler/runtime files remain unchanged. Grok's
current reports and original screenshots are preserved as historical claims.
No dependencies, persistence, network integration, semantic history, projection,
diff calculation or admission authority were introduced.

## Findings repaired

1. **G07 duplicate IDs crashed the mounted renderer (blocking, fixed).** Selecting
   `duplicate-nodes` or `duplicate-edges` raised Svelte `each_key_duplicate` before
   the input diagnostic appeared. Validation alone did not protect the accessible
   lists, which were still keyed by invalid IDs. Lists now use positional keys,
   preserving every supplied row and the visible invalid-input diagnostic. Browser
   regressions cover both duplicates, overlapping IDs and recovery to valid input.
2. **G07 canvas selection overrode a retained parent value (blocking, fixed).**
   Select n2 in the list, enable Ignore onSelect, then click canvas n1: previously
   n1 remained highlighted even though selectedId was still n2. Flow mutates its
   bound arrays independently of the callback. A local reconciliation effect now
   restores node and edge selection to selectedId. Regression failed before repair
   and passes afterward; ignored edge deselection is also covered.
3. **G07 canvas keyboard selection omitted the parent callback (fixed).** Installed
   Flow keyboard handlers changed internal selection without calling node/edge click
   handlers. A scoped capture handler now routes unmodified Enter/Space/Escape on
   canvas nodes/edges through onSelect and suppresses the competing local action.
   Other keys and modified shortcuts are left alone. Browser checks cover accepted
   node/edge keyboard selection and ignored Escape, alongside pointer selection.
4. **G07 generation lifetime did not match the packet literally (fixed).** A new
   LayoutSession had been constructed on every prop replacement. Disposal prevented
   stale commits, but reset its generation counter. One session now lives for the
   component lifetime, invalidates each replacement and disposes on unmount.
   Mounted browser tests exercise reverse success order, stale rejection, current
   failure, loading with no old canvas, empty/invalid replacement, and unmount.
5. **Verification gaps (strengthened).** Original browser assertions did not cover
   the defects above. Source tests now compare exact textContent (not normalized
   whitespace), test LF/CRLF with/without final newline through typing/paste/delete,
   Unicode/tabs, retained/nonfinite external selection, view-mode parent replacement,
   editor DOM identity and repeated mount callbacks. Changes tests verify exact
   spaces, separate duplicate/empty entries and keyboard access to long values.
   Each new spec now checks exact local origin, including scheme. Explicit
   `CAPTURE_ASTRA_EVIDENCE=1` writes separate evidence without replacing Grok images.

Production component repairs are confined to GraphRenderer.svelte. G06 and G08
component implementations required no repair. Their controlled prop paths were
also inspected: source replacement is internally annotated, view edits are filtered
at transaction level, and ChangesPanel derives detail from the current supplied
array and selectedId, so removal cannot retain fabricated details.

## Executed checks

All frontend commands below ran from the repository root through
`nix develop --no-write-lock-file -c sh -c 'cd frontend && COMMAND'`.
No host Node PATH override, global install, peer override or lockfile update was used.
Commands connected to Chromium were serialized on port 4173 with one worker and
`reuseExistingServer: false`.

Actual versions: Node **v24.19.0**, npm **11.17.0**, Playwright **1.63.0**,
Chromium **153.0.8010.12**, Svelte **5.57.0**, Svelte Flow **1.6.6**,
ELK **0.12.0**, Vitest **4.1.11**, Vite **8.3.0**.
Nix reported `nix (Determinate Nix 3.22.4) 2.35.2`.
Node/npm were printed inside the Nix shell; Chromium version came from a real
`chromium.launch()`/`browser.version()`/`browser.close()` probe (exit 0).

| Command / stage | Exit | Observed result |
|---|---:|---|
| Initial Nix node/npm + npm ci, sandboxed | 1 | Nix fetcher lock under ~/.cache was denied; no install/test pass claimed |
| Same command with approved cache/network access | 0 | Node/npm versions above; npm ci added 111 packages, audited 112; postinstall Chromium installation check succeeded; audit reported 0 vulnerabilities |
| Entry-state `npm run check && npm test && npm run build && npm run test:browser` | 0 | 0 Svelte errors/warnings, test TypeScript passed; 69 unit tests / 17 files; production build; 40 Chromium tests |
| `npx playwright test tests/graph-renderer.spec.ts -g "invalid duplicate\|invalid overlapping\|canvas requests"` before repair | 1 | 3 failed, 1 passed: duplicate nodes, duplicate edges, ignored canvas selection reproduced |
| `npm run check && npx playwright test tests/graph-renderer.spec.ts tests/source-editor.spec.ts tests/changes-panel.spec.ts` after initial repairs | 0 | 21 Chromium tests; type checks passed |
| `npm run check && npx playwright test tests/graph-renderer.spec.ts` after mounted race/gesture coverage | 0 | 10 graph Chromium tests; type checks passed |
| `npm run check && npm test && npm run build && CAPTURE_ASTRA_EVIDENCE=1 npm run test:browser` | 0 | 69 unit tests / 17 files; build; 49 Chromium tests; nine fresh screenshots |
| Final test-only edits: `npm run check && npm run test:browser` | 0 | 0 Svelte errors/warnings and TypeScript success; 49/49 Chromium tests, no skips |
| `git diff --check`, staged-diff check, untracked text whitespace/conflict-marker scan | 0 | Clean; nothing staged |
| Contract cmp, HEAD diff and ancestry checks | 0 | Frozen contracts unchanged/equal; expected base and ancestors |

Each command in the successful `&&` chains completed with exit 0. The final edits
only tightened origin guards and a retained-selection assertion; production source
was unchanged after the combined unit/build run. Browser tests fail on uncaught
page errors and nonlocal requests. No such failures occurred in final runs.

Real ELK and Svelte Flow ran in all ordinary graph browser cases. Only the explicitly
named mounted race test replaced the local promise-based layout adapter. The suite
verified finite node positions, rendered chain/cycle/parallel edges, actual pan/zoom,
node drag prevention, disabled connection handles, deletion prevention, controlled
selection and unique instance DOM IDs. A visual edge count is not an assertion of
network semantics or an independent routing algorithm.

A convenience contact-sheet command failed because host Python lacked Pillow;
no package was installed. All nine PNGs were inspected directly with the image
viewer instead. This failed convenience command is not test evidence.

## Screenshot review

Fresh images are under [G06/astra](G06/astra/), [G07/astra](G07/astra/) and
[G08/astra](G08/astra/), with the original three filenames per task. All nine fresh
files were byte-for-byte identical to the original nine images inspected visually.
The manifest hashes both sets. Screenshot names describe the browser viewport;
`fullPage: true` produces taller images:

| Task | Desktop images | Narrow image |
|---|---|---|
| G06 | 1280×1002 edit, 1280×982 view | 390×2399 edit/two instances |
| G07 | 1280×1199 chain, 1280×1456 layers | 390×2749 long/HTML-like labels/two instances |
| G08 | 1280×976 kinds, 1280×905 details | 390×2132 stacked Before/After |

The wide views are readable; editor line numbers/text, real graph edges, selected
change details and empty-state distinctions are visible. Narrow views wrap without
horizontal page overflow. G07 canvas text shrinks under fit-view; full labels remain
readable in the accessible list. G08 Before/After stack vertically. Harness navigation
and synthetic controls make narrow full-page evidence tall; the initial 844px view
does not show the entire widget. This is a development harness, not product layout
acceptance. HTML-like strings remain literal text.

## Remaining limitations and evidence boundaries

- No blocking in-batch defect remains. Mixed LF/CRLF and standalone CR intentionally
  use the specified read-only fallback; general editor integration remains deferred.
- Clipboard coverage uses cancelable browser ClipboardEvent/DataTransfer and drop
  fixtures, not the OS clipboard permission path. Browser coverage is macOS Chromium,
  not Firefox/WebKit or every operating-system shortcut convention.
- Svelte Flow still logs its known shortcut warning for `deleteKey={null}`. Tests
  demonstrate that deletion is prevented. No dependency or shared-library change
  was made. npm also warned about fsevents install-script approval; npm ci still
  exited 0 and the required tooling ran successfully.
- Production build retains an approximately 1.61 MB minified ELK-containing chunk
  and Vite's >500 kB warning. Bundle redesign is outside this batch.
- The changed-array/retained-ID removal path in ChangesPanel was inspected in source;
  browser cases cover array replacement/empty input, unknown ID, and ignored requests,
  but not a separate fixture that removes only the selected row while retaining its ID.
- Idris/Rust suites, full nix flake check, VM/device/deployment and native adapter
  acceptance were not run and are not claimed. No automation was resumed.

## Exact state and reproduction

[astra-turn-003-manifest.json](astra-turn-003-manifest.json) records SHA-256,
Git-style mode (including executable bits), type and path for every tracked and
nonignored untracked file in the reviewed checkout. Unchanged repository files are
baseline identity records, not claims of a new full-system review. Only the manifest,
this review record and REVIEW-GATES.md are excluded to avoid self-reference.
Git-ignored install/build/test outputs are not part of the proposed commit inventory.

From the repository root, verify the state before using this gate:

```sh
python3 - <<'PY'
import hashlib, json, os, stat, subprocess
from pathlib import Path
m = json.loads(Path('docs/grok/reports/astra-turn-003-manifest.json').read_text())
assert subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip() == m['base_head']
assert subprocess.check_output(['git', 'diff', '--cached', '--name-only']) == b''
paths = set(filter(None, subprocess.check_output(
    ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z']
).decode().split('\0'))) - set(m['excluded'])
assert paths == {r['path'] for r in m['files']}
for r in m['files']:
    p = Path(r['path'])
    link = p.is_symlink()
    data = os.readlink(p).encode() if link else p.read_bytes()
    mode = '120000' if link else ('100755' if p.stat().st_mode & 0o111 else '100644')
    assert hashlib.sha256(data).hexdigest() == r['sha256'], r['path']
    assert mode == r['mode'], r['path']
print('Reviewed inventory, hashes and executable modes match')
PY
nix develop --no-write-lock-file -c sh -c 'cd frontend && npm ci'
nix develop --no-write-lock-file -c sh -c 'cd frontend && npm run check && npm test && npm run build && npm run test:browser'
```

Routine screenshots go to ignored frontend/test-results. To deliberately recapture
review evidence, use `CAPTURE_ASTRA_EVIDENCE=1` on the browser command; it changes
only the separate Astra evidence paths and may require a refreshed manifest if
rendering differs. Do not use CAPTURE_GROK_EVIDENCE for routine verification.

The reviewed batch is ready for a user-controlled commit. No staging, commit, push,
PR creation, additional implementation batch or automation was performed.

Manifest SHA-256: `669a8fe2a3b807198adf5517f8d39ecafffca6e6b959096cfbe10fed5719a0de`.
