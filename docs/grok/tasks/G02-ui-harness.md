# G02 — Svelte component development harness

**First-turn task; required review gate for G03–G08.** Build the harness and test
infrastructure only. Do not build the product, compiler bridge, or backend client.

## Own and dependencies

May create/edit `frontend/**`, except future component files belonging to G03–G08.
Also own `docs/grok/reports/G02.md` and screenshot directory
`docs/grok/reports/G02/`. Do not edit root package/Cargo/Make files.
Copy `docs/grok/contracts/presentation.ts` byte-for-byte to
`frontend/src/contracts/presentation.ts`; do not redesign it.

Use Svelte 5, TypeScript, Vite and its compatible Svelte plugin, svelte-check,
Vitest, Playwright, Svelte Flow (`@xyflow/svelte`), ELK (`elkjs`), and CodeMirror 6
(`@codemirror/state`, `@codemirror/view`, `@codemirror/commands`). Choose mutually
compatible stable registry releases; record exact resolved versions in the npm
lockfile. Read package engine requirements and report the tested Node/npm versions.
No SvelteKit, React, backend framework, routing library, design system, or paid API.
Normal project-local dependency installation and Playwright Chromium are allowed.
Do not force installation past incompatible peer dependencies.

## Exact structure and scripts

Create a normal Vite app with `frontend/package.json`, `package-lock.json`,
`index.html`, TS/Vite/Svelte configuration, `src/main.ts`, `src/App.svelte`,
`src/app.css`, the copied contract, and browser/unit test configuration.

Provide scripts: `dev`, `build`, `check`, `test` (non-watch Vitest), and
`test:browser` (non-watch Playwright). Playwright starts/stops the development
server on `127.0.0.1:4173` with strict port selection. Document commands in
`frontend/README.md`; ignore node_modules, dist, test-results and playwright-report
inside `frontend/.gitignore`. Do not ignore source, lockfiles or screenshots in
`docs/grok/reports/`.

## Harness behavior

- Show a persistent heading: **Component development fixtures — no live device
  data or admission decisions**. Do not imply this is the integrated product.
- Discover `src/features/*/Demo.svelte` using Vite's static import glob. A folder
  `view-controls` must be reachable at `/#/view-controls`, etc. Render the selected
  fixture and a simple accessible navigation list. Initially there may be none.
- Unknown fixture routes show a useful empty/error state, never a network fetch.
- Future workers add their own feature directory and Demo.svelte without changing
  App.svelte, dependency files, or a shared registry. Verify discovery with a
  temporary test-only/simple example fixture in `src/features/harness-example/`.
- All state is in memory. No server endpoints, compiler invocation, telemetry,
  localStorage, cookies, service worker, external fonts or remote assets.
- Supply readable base typography, focus rings and spacing at 1280x800 and
  390x844. Do not build navigation for imaginary product features.

## Steps

1. Initialize only `frontend/`; install and lock the allowed dependencies.
2. Copy the frozen contract and implement the minimal fixture loader/navigation.
3. Add a unit test for route-to-fixture resolution and a real Chromium smoke test.
4. Verify a second fixture is discovered without edits to shared app files.
5. Exercise unknown routes and keyboard navigation; capture both viewport sizes.
6. Run all commands below and write the report. List any design question instead
   of making a new domain model.

## Required verification

From `frontend/`: `npm ci`, `npm run check`, `npm test`, `npm run build`,
`npm run test:browser`. Browser tests must use the real rendered app and fail on
uncaught browser errors. Verify no application requests leave the local dev
origin. Confirm the copied contract matches its source bytes.

Acceptance: clean install works, component routes load, empty/unknown cases work,
keyboard focus is visible, both viewport screenshots are readable, and no
forbidden state/backend features exist. Screenshots go under
`docs/grok/reports/G02/`. Type-checking alone is not browser verification.
