# Turn 002 — coordinator report (G03 / G04 / G05)

- Base HEAD: `9468988b5d5565c8320d012602f14aca8829cf3e`
- Branch: `codex/grok-easy-tasks`
- Task and packet: G03 / G04 / G05 only (`docs/grok/TURN-002.md`)
- Outcome: Implemented awaiting Astra review
- Not done: no commit, stage, push, PR, G06/G07/G08, REVIEW-GATES / GROK_HANDOFF / TURN-002 packet edits, or Astra acceptance

Workers (disjoint ownership):

| Task | Assignment | Feature and browser test |
|---|---|---|
| G03 | view-controls worker | `frontend/src/features/view-controls/**`; `frontend/tests/view-controls.spec.ts` |
| G04 | snapshot-panel worker | `frontend/src/features/snapshot-panel/**`; `frontend/tests/snapshot-panel.spec.ts` |
| G05 | assurance-panel worker | `frontend/src/features/assurance-panel/**`; `frontend/tests/assurance-panel.spec.ts` |

Coordinator additionally owns this file and in-scope G04 keyboard-scroll repair after the first focused Playwright failure.

## Owned files changed

Union of ownership (all untracked, unstaged):

### G03

- `frontend/src/features/view-controls/ViewControls.svelte`
- `frontend/src/features/view-controls/Demo.svelte`
- `frontend/src/features/view-controls/view-selection.ts`
- `frontend/src/features/view-controls/view-selection.test.ts`
- `frontend/tests/view-controls.spec.ts`
- `docs/grok/reports/G03.md`
- `docs/grok/reports/G03/desktop-1280x800-default.png`
- `docs/grok/reports/G03/desktop-1280x800-advanced.png`
- `docs/grok/reports/G03/desktop-1280x800-keyboard-focus.png`
- `docs/grok/reports/G03/mobile-390x844-default.png`

### G04

- `frontend/src/features/snapshot-panel/SnapshotPanel.svelte`
- `frontend/src/features/snapshot-panel/Demo.svelte`
- `frontend/src/features/snapshot-panel/labels.ts`
- `frontend/src/features/snapshot-panel/labels.test.ts`
- `frontend/src/features/snapshot-panel/cases.ts`
- `frontend/src/features/snapshot-panel/cases.test.ts`
- `frontend/src/features/snapshot-panel/scrollable-value.ts`
- `frontend/src/features/snapshot-panel/scrollable-value.test.ts`
- `frontend/tests/snapshot-panel.spec.ts`
- `docs/grok/reports/G04.md`
- `docs/grok/reports/G04/desktop-1280x800-committed.png`
- `docs/grok/reports/G04/desktop-1280x800-redacted-unknown.png`
- `docs/grok/reports/G04/mobile-390x844-long-text.png`

### G05

- `frontend/src/features/assurance-panel/AssurancePanel.svelte`
- `frontend/src/features/assurance-panel/Demo.svelte`
- `frontend/src/features/assurance-panel/display.ts`
- `frontend/src/features/assurance-panel/display.test.ts`
- `frontend/src/features/assurance-panel/fixtures.ts`
- `frontend/src/features/assurance-panel/fixtures.test.ts`
- `frontend/tests/assurance-panel.spec.ts`
- `docs/grok/reports/G05.md`
- `docs/grok/reports/G05/desktop-1280x800-all-outcomes.png`
- `docs/grok/reports/G05/desktop-1280x800-stale-success-detail.png`
- `docs/grok/reports/G05/mobile-390x844-selection.png`

### Coordinator

- `docs/grok/reports/turn-002.md`
- G04 `scrollable-value.ts` / `scrollable-value.test.ts` and `SnapshotPanel.svelte` keyboard-scroll wiring (in-scope fix only)

No other paths in `git status --short`. Contracts, App.svelte, global CSS, package/lock, configs, canary fixtures, harness specs, G01/G02/G09 reports, NetDSL, runtime, compiler, and Cargo were not modified.

## Behavior delivered

- **G03:** Controlled L1/L2/L3/L4/L7 primary radios, collapsed Advanced overlays (primary excluded, canonical order), combined overview retain/restore, independent state/assurance/editMode, no input mutation, View/Edit is a mode request only.
- **G04:** Read-only snapshot display with four distinct datastore labels, verbatim timestamps (**Unknown** if null), uncoerced scalars, ordered lists with duplicates, empty scalar vs empty list vs redacted vs unknown, escaped HTML-like text, scrollable long values.
- **G05:** Read-only assurance rows in supplied order, eleven distinct outcomes plus **No observation**, controlled selection, stale/partial/unknown Success without a health verdict, exact large string IDs, escaped HTML-like detail.

## Coordinator verification

Working directory `/Users/erikwilliamson/Documents/network-intent` unless noted `frontend/`.
PATH: `/Users/erikwilliamson/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin`.
Node `v24.19.0`, npm `10.9.2`. Port 4173 was free before each Playwright run. `reuseExistingServer` stayed false. Playwright commands were serialized (never overlapping). `npm ci` was not rerun.

| Command (include working directory) | Exit code | What it establishes |
|---|---|---|
| repo root: `git rev-parse HEAD` | 0 | HEAD is exactly `9468988b5d5565c8320d012602f14aca8829cf3e` |
| PATH-selected `node --version` / `npm --version` | 0 | `v24.19.0` / `10.9.2` |
| `cmp docs/grok/contracts/presentation.ts frontend/src/contracts/presentation.ts` | 0 | Contract copies match each other |
| `git show HEAD:… \| cmp -` both presentation.ts files | 0 | Both copies still match HEAD |
| `frontend/`: `npx playwright test tests/view-controls.spec.ts` | 0 | G03 focused 7/7 |
| `frontend/`: `npx playwright test tests/snapshot-panel.spec.ts` (first) | 1 | G04: 5 passed, keyboard-scroll failed |
| `frontend/`: `npx playwright test tests/snapshot-panel.spec.ts` (rerun) | 0 | G04 focused 6/6 after in-scope fix |
| `frontend/`: `npx playwright test tests/assurance-panel.spec.ts` | 0 | G05 focused 4/4 |
| `frontend/`: `npm run check` | 0 | svelte-check 0 errors/warnings; test TypeScript |
| `frontend/`: `npm test` | 0 | Vitest 7 files / 43 tests |
| `frontend/`: `npm run build` | 0 | Vite production build |
| `frontend/`: `npm run test:browser` | 0 | Playwright 24/24 including original harness |
| repo root: `git diff --check` | 0 | No tracked whitespace errors |
| Untracked text-file trailing-whitespace / conflict-marker scan | 0 | Clean |
| `git status --short` | 0 | Ownership table + this report only; nothing staged |

Idris, Rust, QEMU/Docker/SSH, and VM suites were **not** rerun.

### Focused Playwright

1. `cwd=/Users/erikwilliamson/Documents/network-intent/frontend` `npx playwright test tests/view-controls.spec.ts` → exit 0, 7 passed (5.9s).
2. Same cwd `npx playwright test tests/snapshot-panel.spec.ts` → exit 1, 5 passed / 1 failed (`scrolls long values from the keyboard and fits 390x844`: `scrollTop` expected > 0, received 0).
3. In-scope fix in G04 owned files: `applyScrollKey` on read-only `<pre>` value boxes plus `min-height: 0` so `max-height` clips.
4. Same cwd `npx playwright test tests/snapshot-panel.spec.ts` → exit 0, 6 passed (2.4s).
5. Same cwd `npx playwright test tests/assurance-panel.spec.ts` → exit 0, 4 passed (1.8s).

### Unsuccessful commands and resolutions

| Command | Exit | Resolution |
|---|---|---|
| `frontend/`: `npx playwright test tests/snapshot-panel.spec.ts` | 1 | Chromium does not move `scrollTop` on End/PageDown for a focused overflow box. Coordinator added `frontend/src/features/snapshot-panel/scrollable-value.ts` and wired `onkeydown` in `SnapshotPanel.svelte`. Spec unchanged. Rerun exit 0. |

No fabricated passes. No test skips. No weakened assertions.

## Screenshot inventory (vision inspection)

All files exist on disk. Playwright `fullPage: true` makes PNG height exceed the named viewport; width matches 1280 or 390.

### G03

- [G03/desktop-1280x800-default.png](G03/desktop-1280x800-default.png) — L1–L4/L7; Advanced collapsed; readable labels
- [G03/desktop-1280x800-advanced.png](G03/desktop-1280x800-advanced.png) — overlays exclude current primary; combined switch visible
- [G03/desktop-1280x800-keyboard-focus.png](G03/desktop-1280x800-keyboard-focus.png) — ≥3px focus ring on primary L3
- [G03/mobile-390x844-default.png](G03/mobile-390x844-default.png) — 390px wrap; no clipped layer/state labels

### G04

- [G04/desktop-1280x800-committed.png](G04/desktop-1280x800-committed.png) — **Committed configuration**; `0`/`false`/`007`; duplicate list entries
- [G04/desktop-1280x800-redacted-unknown.png](G04/desktop-1280x800-redacted-unknown.png) — empty scalar vs **Empty list** vs **Redacted** vs **Unknown**
- [G04/mobile-390x844-long-text.png](G04/mobile-390x844-long-text.png) — HTML-like tags as text; Partial warning; focused scrollable banner; **Session staging**

### G05

- [G05/desktop-1280x800-all-outcomes.png](G05/desktop-1280x800-all-outcomes.png) — plan id `9007199254740993` exact; eleven outcomes + **No observation**; HTML-like label as text
- [G05/desktop-1280x800-stale-success-detail.png](G05/desktop-1280x800-stale-success-detail.png) — stale Success; distinct Observation vs Provenance sources
- [G05/mobile-390x844-selection.png](G05/mobile-390x844-selection.png) — 390px wrap; keyboard focus ring; long dependency id wraps

Existing G02 evidence under `docs/grok/reports/G02/` was not overwritten. Harness screenshots went to ignored `frontend/test-results/harness-screenshots/`.

## Limitations / Astra decisions

Keep worker notes in G03/G04/G05 reports. Coordinator additions:

1. **G04 keyboard scrolling required a component handler.** Native overflow-box keys were not enough in Chromium.
2. **Placeholder chrome** for null reason / empty missing (`None supplied`, `No missing items listed`, G05 `none`) is not in the contract.
3. **G03 overlay canonicalization on every emit**, including unrelated field changes.
4. **G05 primitive tokens** are shown as contract enums, not invented display names.
5. **No Astra acceptance is claimed.** Stopped for the user’s manual Astra review.

## Scope check

- Contracts match each other and HEAD (`cmp`).
- No forbidden files changed.
- Changes are unstaged and uncommitted (`git status` shows `??` only; `git diff --cached` empty).
- Later tasks were not started. Devices/QEMU/Docker/SSH were not used.
- Stop here for manual Astra review.
