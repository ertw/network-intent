# Component development harness

Svelte 5 + Vite UI harness for isolated component fixtures. This is not the
integrated Network Intent product. Fixtures are in-memory development views and
must not be treated as live device data or admission decisions.

## Toolchain

Use Node `^20.19.0 || ^22.12.0 || >=24.0.0` and npm `>=10`, matching the
intersection of the locked packages' engines. Grok ran the initial checks on
Node 23.11.0, which is outside that supported range and produced engine warnings.
Astra's independent results are recorded in `docs/grok/reports/astra-turn-001.md`.

Use `npm ci` from the lockfile. Grok reported an npm 10.9.2 arborist crash during
initial dependency resolution and used `--legacy-peer-deps` once to generate
this lockfile. Normal clean installation and peer validation must pass without
that flag; it is not part of the supported workflow.

## Commands

Run these from `frontend/`:

| Script | Command | Purpose |
|---|---|---|
| Install | `npm ci` | Clean install from the lockfile; also installs Playwright Chromium |
| Dev | `npm run dev` | Vite on `127.0.0.1:4173` with `strictPort` |
| Typecheck | `npm run check` | `svelte-check` plus `tsc` for configs, unit tests and browser tests |
| Unit tests | `npm test` | Non-watch Vitest (`vitest run`) |
| Production build | `npm run build` | Vite production bundle |
| Browser tests | `npm run test:browser` | Non-watch Playwright (starts/stops the dev server) |

Playwright launches `npm run dev`, waits for `http://127.0.0.1:4173`, and stops
the server when tests finish. The port is fixed (`strictPort`); another process
bound to 4173 will fail the run.

## Adding a fixture

Create `src/features/<name>/Demo.svelte`. Vite's static glob picks it up and
the route `/#/<name>` works without editing `App.svelte`, `package.json`, or a
name registry. Product features such as `view-controls` are reachable at
`/#/view-controls` once their `Demo.svelte` exists.

Keep fixture strings as text. Do not fetch, persist, or talk to a compiler from
UI components.

Routine G02–G05 browser screenshots are written to ignored `test-results/`
subdirectories. Their committed report screenshots are historical review evidence
and are not overwritten by later test runs. New task specs should expose an
explicit evidence-capture option; see `docs/grok/CURRENT.md`.
