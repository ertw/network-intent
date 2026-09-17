# G05 — Read-only assurance results panel

**Requires Astra acceptance of G02 and an explicit new user turn.**

Own only `frontend/src/features/assurance-panel/**`,
`frontend/tests/assurance-panel.spec.ts`, and `docs/grok/reports/G05*`.
Export `AssurancePanel.svelte` and `Demo.svelte` using `AssurancePanelProps`.
No shared-file edits or new dependencies.

## Exact behavior

1. Show the supplied plan ID and graph-version label without numeric conversion.
   Null values display "No plan" and "Unknown graph version" respectively.
2. Show one row per input row in the supplied order. Include label, primitive,
   outcome, freshness and completeness. Null outcome means "No observation".
3. Preserve distinct outcome labels: Success, Violation, Timeout, Refused,
   Unreachable, DNS NXDOMAIN, DNS server failure, TLS failure, Malformed response,
   Unsupported, Unavailable. Do not merge timeout with violation or unsupported.
4. Selecting a row calls `onSelect(id)`; selection is controlled by `selectedId`.
   Its detail shows source, endpoint (or "Device-local / no endpoint supplied"),
   supplied detail text, provenance, and dependency IDs as text in supplied order.
5. Stale/unknown freshness and partial/unavailable completeness stay visible next
   to a raw Success outcome. Use neutral styling for such a success; a positive
   accent is allowed only when the supplied freshness is fresh and completeness
   complete. This is display styling, not a computed health assessment.
6. Never display an aggregate "Healthy", "Admitted", "Verified deployment" or
   "Safe to apply" verdict. Do not count streaks, traverse the DAG, calculate
   freshness, interpret protocol messages, execute probes, or create incidents.
7. Empty rows show "No assurance observations". Unknown selected IDs produce no
   invented detail. All strings are escaped and accessible by keyboard.

## Demo / tests

Use a synthetic row for each of the eleven outcomes, plus a null outcome; include
fresh-success, stale-success, partial-success, unavailable, and unknown-freshness
cases. Include large-looking string IDs that must remain exact, long dependency
IDs, and HTML-like detail text. Browser tests assert every distinction, controlled
selection, prop updates, no input mutation, no aggregate verdict, and narrow-screen
readability. Capture wide and narrow screenshots.

Run `npm run check` and `npx playwright test tests/assurance-panel.spec.ts` from
frontend. If a prop cannot express a needed authoritative fact, report it; do not
extend the contract or invent that fact.
