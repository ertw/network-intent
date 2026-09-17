# Turn 002 — controlled views, snapshots and assurance display

Execute **G03 / G04 / G05 only** when the user supplies this batch's Cursor prompt.
G01/G02/G09 were accepted and committed as `d527622`. G06/G07/G08 remain excluded.
The frozen contracts and all restrictions in [GROK_HANDOFF.md](../../GROK_HANDOFF.md)
still apply. This is isolated component work, not integration into the product.

## Coordinator startup

1. Read this file, [review gates](REVIEW-GATES.md), all three task packets and
   [the prior review](reports/astra-turn-001.md). Inspect `git status --short`.
   Start only from a clean checkout on `codex/grok-easy-tasks` containing this
   handoff update and ancestor `d527622`. Record the actual full HEAD in reports.
   Do not recreate/reset the branch, stash changes, or rerun the first batch.
2. Use a supported Node runtime: `^20.19.0 || ^22.12.0 || >=24.0.0`, npm >=10.
   Record `node --version` and `npm --version`. The host default Node 23 is not
   supported. On this host, the existing bundled Node 24 can be selected for
   this shell with:

   ```sh
   export PATH="/Users/erikwilliamson/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PATH"
   ```

   This path is host-specific; use another already installed compatible Node
   if it is absent. Do not install global tools or change user shell settings.
3. Run `npm ci` once from `frontend/` before starting workers. Do not use
   `--legacy-peer-deps`, `--force` or modify package/lock files. If installation
   or runtime selection blocks, report it; do not redesign the dependency tree.
4. Compare the two presentation contract files byte-for-byte. They must match
   both each other and HEAD. Do not extend the props or invent domain authority.

## Delegation and ownership

One coordinator may assign one task to each of up to three workers. Give each
worker its complete task packet and these batch rules. No nested coordinators.

| Worker | Feature and browser test | Reports |
|---|---|---|
| [G03](tasks/G03-view-controls.md) | `frontend/src/features/view-controls/**`; `frontend/tests/view-controls.spec.ts` | `docs/grok/reports/G03.md`; `docs/grok/reports/G03/**` |
| [G04](tasks/G04-snapshot-panel.md) | `frontend/src/features/snapshot-panel/**`; `frontend/tests/snapshot-panel.spec.ts` | `docs/grok/reports/G04.md`; `docs/grok/reports/G04/**` |
| [G05](tasks/G05-assurance-panel.md) | `frontend/src/features/assurance-panel/**`; `frontend/tests/assurance-panel.spec.ts` | `docs/grok/reports/G05.md`; `docs/grok/reports/G05/**` |

The coordinator additionally owns `docs/grok/reports/turn-002.md`. It may reconcile
in-scope component fixes after the corresponding worker has stopped editing.
No changes to App.svelte, global CSS, contracts, package/lock files, configs,
canary features, existing tests or previous reports/screenshots. Demo.svelte
files are discovered by the existing glob; no shared registration is needed.

Workers may implement concurrently. **Browser runs are serialized**: Playwright
owns fixed port 4173 and its shared test-results directory. Workers report ready
for focused verification; the coordinator schedules each command in turn. Do
not kill an unrelated process or change the port if it is occupied. Do not run
`npm ci` while workers or tests are using node_modules. Run whole-project type
checks after all workers' files are stable to avoid transient cross-worker errors.

## Exact component boundaries

- G03 emits controlled view/layer/mode requests. It performs no graph filtering,
  authorization or semantic editing.
- G04 displays supplied snapshots with datastore/provenance/order/redaction
  distinctions. It performs no observation, freshness calculation or decoding.
- G05 displays supplied observations and their raw outcomes. It performs no
  aggregate health/admission decision, probe scheduling or dependency traversal.
- Fixture selectors/reset buttons/event counters belong to Demo.svelte. The
  exported components expose only the frozen props and packet behavior.
- Keep fixtures synthetic, labelled and in memory. No real device data, network
  clients, persistence, telemetry, compiler calls or services.
- Any contract gap goes into the task report; do not fill it with an invented
  authoritative value. Complete unrelated authorized work if a task blocks.

## Verification and handback

Run focused browser commands from the packets sequentially, then from `frontend/`:

```sh
npm run check
npm test
npm run build
npm run test:browser
```

`check` includes test TypeScript. The full browser suite must pass with all new
fixtures present. Existing harness screenshots now go to ignored
`frontend/test-results/harness-screenshots/`, preserving the committed G02 evidence.
New task screenshots go only into their respective G03/G04/G05 report directories.
Inspect actual wide/narrow screenshots, keyboard interactions, escaped text and
empty/error/unknown/partial/stale cases required by each packet. Each new browser
test file must fail on uncaught browser errors and requests leaving the local
origin; existing harness checks do not automatically apply to another spec file.

Use [REPORT-TEMPLATE.md](REPORT-TEMPLATE.md). Record exact commands, working
directory, exit codes, Node/npm versions, changed files, screenshots and limitations.
The coordinator lists worker assignments and combined checks in turn-002.md.
Include any unsuccessful commands and their resolutions; do not fabricate passes.

Before stopping, inspect tracked and untracked files, check whitespace, confirm
contracts still match HEAD, and confirm only the ownership table plus turn-002.md
changed. Do not rerun Idris/Rust/VM suites for this presentation-only batch.
**Leave changes uncommitted and unstaged. Stop for the user's manual Astra review.**
Do not update review gates, start later tasks, push, create PRs, operate devices,
or resume the automatic goal runner.
