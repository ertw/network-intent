# Current batch — turn 003 (G06 / G07 / G08)

Status: prepared for manual Grok 4.6 / Cursor launch. No tasks have been started.
Accepted implementation baseline: `2774655`, merged into local main. Begin from
the clean preparation commit containing this file; record its actual full HEAD.

## Read budget and authority

Coordinator reads this file, [review gates](REVIEW-GATES.md), the three packets
below, [presentation contract](contracts/presentation.ts) and
[report template](REPORT-TEMPLATE.md). Workers read this file, their own packet,
the same contract, and only relevant frontend source. Existing package/config files
are read-only context. Do not bulk-load historical reports, raw logs, old task
packets, progress archives or the full design specification. They are indexed in
[docs/README.md](../README.md) for targeted lookup only.

The full system uses Idris for semantics and Rust for assurance/runtime work.
This batch is presentation only. The Svelte app is a labelled development harness,
not a production application. Current examples are still admission-blocked;
native adapter/service and hardware acceptance are incomplete. These facts are
context, not tasks to fix.

## Startup and coordination

1. Inspect git status, branch and HEAD. Require a clean checkout containing
   ancestor `2774655` and this handoff. If dirty, stop without reset/stash/delete.
2. Work on `codex/grok-easy-tasks`. If starting on main, switch to that existing
   branch and fast-forward it from main with `git merge --ff-only main`. Stop if
   it has diverged. Do not reset branches or make commits. Record HEAD after this.
3. Use the committed Nix development shell, including in each worker terminal.
   Require the Nix baseline commit `bd9da11` as an ancestor of starting HEAD.
   From the repository root, enter `nix develop --no-write-lock-file`, then run
   frontend commands inside that shell. Noninteractive terminals may use:

   ```sh
   nix develop --no-write-lock-file -c sh -c 'cd frontend && npm ci'
   nix develop --no-write-lock-file -c sh -c 'cd frontend && npm run check'
   ```

   Use the same wrapper for tests, build and Playwright. Record actual Node/npm
   versions; the verified shell supplies Node 24.19.0 and npm 11.17.0. Do not
   prepend the old host runtime PATH over Nix. Run `npm ci` once before workers.
   No `--force`/`--legacy-peer-deps`, global installs, dependency changes, Nix
   configuration changes or lockfile regeneration. If Nix cannot start, report
   the failure instead of silently switching toolchains. Nix supplies Node/npm;
   frontend dependencies and Playwright browser availability still need checking.
   Do not run `nix flake check` for this presentation-only batch.
4. Verify both contract copies equal each other and their HEAD versions.
5. One coordinator may assign up to three workers, one per task. No nested
   coordinators. Workers may implement concurrently; **serialize all Playwright
   runs** because port 4173 and test-results are shared. Run global type checks
   only after files are stable. Do not reinstall dependencies during worker runs.
6. If a task blocks on design or a forbidden edit, report the smallest reproduction
   and finish unrelated authorized work. Do not invent replacement tasks.

## Ownership

| Packet | Component directory and browser test | Reports |
|---|---|---|
| [G06](tasks/G06-source-editor.md) | `frontend/src/features/source-editor/**`; `frontend/tests/source-editor.spec.ts` | `docs/grok/reports/G06.md`; `docs/grok/reports/G06/**` |
| [G07](tasks/G07-graph-renderer.md) | `frontend/src/features/graph-renderer/**`; `frontend/tests/graph-renderer.spec.ts` | `docs/grok/reports/G07.md`; `docs/grok/reports/G07/**` |
| [G08](tasks/G08-changes-panel.md) | `frontend/src/features/changes-panel/**`; `frontend/tests/changes-panel.spec.ts` | `docs/grok/reports/G08.md`; `docs/grok/reports/G08/**` |

Coordinator additionally owns `docs/grok/reports/turn-003.md`. Repair a worker's
files only after that worker stops editing. Helpers/unit tests stay in the owned
feature directory. No shared abstractions, imports from other unfinished features,
App.svelte/global CSS edits, dependencies/config changes, edits to prior tests,
reports, screenshots, task packets, review gates or either contract. Demo.svelte
is discovered automatically; no registration changes are needed.

## Common correctness rules

- Use the frozen typed props. Parent values remain authoritative even if a callback
  is ignored or rejected. Test both accepted requests and retained parent values.
- Demo controls/counters are synthetic fixture UI, outside the exported component.
  Keep data in memory and visibly labelled as synthetic; never copy device data.
- Render strings as escaped text; preserve order, duplicates, whitespace and exact
  identifiers. Empty, redacted, unknown and unavailable are distinct states.
- No backend, fetch, persistence, localStorage, IndexedDB, telemetry, credentials,
  compiler execution, authoritative model mapping, health calculation, admission,
  semantic Undo/Redo, deployment, QEMU, Docker, SSH or physical devices.
- G07 may import installed Svelte Flow CSS inside its own feature; no shared CSS
  edits. G06 may use the installed CodeMirror state/view/commands packages. G08
  needs no new dependencies. All required packages already exist in the lockfile.
- Use unique per-instance DOM IDs/radio names. Preserve normal copy/selection and
  modifier shortcuts; custom keyboard handlers must not swallow unrelated keys.

## Verification and handback

Each browser spec must fail on uncaught page errors and requests leaving the
local origin; the existing harness spec's listeners do not apply to other files.
Use real mounted components. Mocks are limited to the specified async race tests;
G07 must also use actual ELK and Svelte Flow in Chromium.

Capture screenshots at 1280x800 and 390x844 under each task's report directory.
New specs must make evidence capture explicit (for example `CAPTURE_GROK_EVIDENCE=1`);
default routine screenshots go under ignored frontend/test-results. Existing
G02–G05 specs now write only ignored outputs. Do not overwrite committed evidence.

After focused checks run sequentially, coordinator runs from frontend:

```sh
npm run check
npm test
npm run build
npm run test:browser
```

Record exact commands/exit codes, versions, actual behavior, screenshots and every
limitation/failure/resolution. Passing type checks alone is not browser evidence.
Check tracked and untracked scope, whitespace, original evidence and contract
hashes. No full Idris/Rust/VM suite for this UI-only batch.

**Stop after G06/G07/G08, leaving changes unstaged and uncommitted for manual Astra
review.** Do not commit, push, create PRs, update acceptance gates, mark the full
goal complete, start another task or enable automatic implementation.
