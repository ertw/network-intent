# G07 — Read-only supplied-graph renderer with ELK

**Turn 003; read [CURRENT.md](../CURRENT.md).** Prerequisites are accepted in
`d527622` and `2774655`; start only on the user’s manual Cursor launch. Only render
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

## Resolved implementation boundaries (Astra validation)

Installed Svelte Flow 1.6.6 exposes nodesDraggable, nodesConnectable,
elementsSelectable, selectionOnDrag and deleteKey controls. The installed bundled
ELK 0.12.0 completed a local layout probe. Read installed types to use the exact API;
do not upgrade packages or add a renderer framework.

- Use local bundled ELK (no remote worker URL), with its promise-based layout.
  A local injectable layout function enables race tests without contract changes.
- Clone all objects handed to libraries. Parent arrays, objects and selectedId
  remain authoritative even when a selection callback is ignored. Never propagate
  library position/selection mutations into the supplied graph or semantic source.
- While new input is laying out, show loading and remove the previous canvas
  rather than presenting it as current. Invalidate pending work on every input
  replacement, including empty/invalid input, and on unmount. Only the latest
  generation may show a graph or an error.
- Configure explicit handles/ports so supplied edges actually render; parallel
  edges and cycles must not disappear. Validate finite positions after ELK.
- Keep fixed 200x80 canvas nodes. Full labels/subtitles and edge labels must remain
  inspectable in the accessible selection list if they exceed the canvas box.
- Test ignored parent selection, A/B reverse completion, older rejected promises,
  replacement by empty/invalid data, no mutation via Delete/Backspace/drag/connect,
  multiple instances, and unmount. Run real ELK+Flow in at least one browser case.
