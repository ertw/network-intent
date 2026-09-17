# G07 — Read-only supplied-graph renderer with ELK

**Requires Astra acceptance of G02 and an explicit new user turn.** Only render
already-projected nodes/edges. The compiler-to-graph mapping and semantic layer
filtering are reserved for Astra.

Own only `frontend/src/features/graph-renderer/**`,
`frontend/tests/graph-renderer.spec.ts`, and `docs/grok/reports/G07*`.
Export `GraphRenderer.svelte` and `Demo.svelte`, using `GraphRendererProps`,
Svelte Flow and ELK already installed by G02. No shared edit or new dependency.

## Exact behavior

1. Preserve every supplied node/edge ID and label. Layer values are display tags,
   not instructions to infer extra vertices, links or dependencies. Use fixed
   node dimensions 200x80 for deterministic initial layout.
2. Layout a cloned input with ELK's layered algorithm, direction RIGHT,
   inter-layer spacing 80 and node spacing 40. Sort the *layout copy* by IDs for
   stable input order; do not mutate caller arrays or change their semantic data.
3. Map ELK positions onto Svelte Flow nodes and render supplied edges. Pan, zoom,
   fit-to-view and selection work. Nodes are not draggable or connectable; edges
   cannot be added/deleted; Delete/Backspace cannot mutate the graph.
4. Node/edge selection calls `onSelect(id)`; background selection calls null.
   The selected ID prop is authoritative. Labels/subtitles are escaped text.
5. Every layout request has a monotonically increasing local generation. If B
   finishes before an older A, only B may update the display. Unmount invalidates
   outstanding requests. Failed layout of current input displays an error and
   does not present the old graph as current. Do not alter global compiler workers.
6. Reject duplicate node IDs, duplicate edge IDs, dangling endpoints, or overlapping
   node/edge IDs with a visible input diagnostic. Never silently discard data.
7. Empty graph displays "No graph to display". Provide a keyboard-accessible list
   of supplied node/edge labels for selection, alongside the canvas; do not rely
   on pointer-only canvas interaction. Fit at 390x844 without page overflow.

## Tests and demo

Synthetic fixtures: empty, single node, disconnected nodes, simple directed chain,
cycle, parallel edges, all five layer tags, invalid IDs/endpoints, long/HTML-like
labels. These are renderer examples, not network topology assertions.

Use a controllable layout adapter in unit tests: start A then B, resolve B then A
and assert only B is displayed; reject B and assert an error; unmount before resolve
and assert no update. At least one real-browser test must run actual ELK and Svelte
Flow, assert all expected labels/edges and finite positions, and exercise selection,
pan/zoom and mutation prevention. Do not satisfy every test using an ELK mock.

Run `npm run check`, the focused layout unit tests, and
`npx playwright test tests/graph-renderer.spec.ts`. Capture wide/narrow screenshots.
Do not add editing gestures, DSL conversion, admission, health aggregation,
persistence, or invented L5/L6 views. Report unresolved renderer behavior to Astra.
