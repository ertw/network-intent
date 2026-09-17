# Astra review — turn 002

Completed 2026-09-17 on `codex/grok-easy-tasks`.

**G03, G04 and G05 accepted after small in-scope fixes. Ready to commit.**
Changes remain unstaged and uncommitted. No later task was started. The next
eligible batch is G06/G07/G08, after committing this reviewed baseline and explicit
user authorization; it still needs a current batch handoff before Cursor launch.

## Exact reviewed state

Base HEAD: `9468988b5d5565c8320d012602f14aca8829cf3e`.
The [manifest](astra-turn-002-manifest.json) records the exact bytes and executable
bits of all 35 implementation, test, Grok report and screenshot files.
Manifest SHA-256:

`d6caf650b707c809f9cd33ecd49047a5c1aebab054e97ec281331af5dda5febe`

Only this review, the manifest itself and REVIEW-GATES.md are excluded to avoid
circular hashes. Acceptance is tied to this content, not future modifications.
To reproduce the identity from the repository root before committing:

```python
from pathlib import Path
import hashlib, json, subprocess
p = Path('docs/grok/reports/astra-turn-002-manifest.json')
assert hashlib.sha256(p.read_bytes()).hexdigest() == 'd6caf650b707c809f9cd33ecd49047a5c1aebab054e97ec281331af5dda5febe'
m = json.loads(p.read_text())
assert subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip() == m['base_head']
changed = set()
for args in [['git', 'diff', 'HEAD', '--name-only', '-z'],
             ['git', 'ls-files', '--others', '--exclude-standard', '-z']]:
    changed.update(filter(None, subprocess.check_output(args).decode().split('\0')))
assert changed - set(m['excluded_review_metadata']) == set(m['files'])
for name, entry in m['files'].items():
    f = Path(name)
    assert f.is_file() and not f.is_symlink(), name
    raw = f.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == entry['sha256'], name
    assert len(raw) == entry['bytes'], name
    assert bool(f.stat().st_mode & 0o111) == entry['executable'], name
```

## Findings and fixes

### G03: native input state could override the parent visually

The original component used `checked={value...}` with an onChange callback. If
the parent retained its existing value, native radio/checkbox activation still
changed the DOM; an unchanged prop did not cause Svelte to reset it. The added
real-browser regression failed: Intent became unchecked even though the parent
still supplied Intent.

The component now emits the request, awaits Svelte's update and synchronizes
native checked states from the latest supplied value. No optimistic domain state
is retained. Radio group names use an instance-specific Svelte ID to avoid
cross-instance native grouping. A fixture-only hold-selection switch allows
verification without changing the frozen component contract. The regression now
passes for state radios, primary radios, assurance, overlays and combined overview.
Normal accepted requests, keyboard navigation and external updates also pass.

### G04: empty scalar had no visible explanation

An empty scalar previously rendered a blank pre element with only an aria-label.
It now has a visible **Empty value** label. The value element's text remains
exactly empty; placeholders are not inserted into the supplied data. The browser
check requires both the visible label and the unchanged empty text.

### G04: custom scrolling consumed modified navigation keys

The scroll handler now leaves already-handled and Shift/Ctrl/Meta/Alt-modified
keys alone. A browser DOM-event regression verifies that these events are not
cancelled and the helper does not change scrollTop. Ordinary End/PageDown still
scroll the real focused overflow box. This test checks handler interception;
it does not claim cross-platform native text-selection behavior.

G05 required no code changes. Raw outcomes, freshness/completeness, source labels,
large string IDs and unknown selections are preserved without aggregate verdicts.

## Independent verification

Frontend commands ran from `frontend/` with Node **24.19.0** and npm **10.9.2**,
using the existing bundled Node directory prepended to PATH:
`/Users/erikwilliamson/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin`.
No global installation or user settings changed.

| Command/check | Result |
|---|---|
| `npm ci --cache /tmp/network-intent-npm-review-cache` | Exit 0; clean install, no legacy peer flag |
| `npm run check` | Exit 0; Svelte 0 errors/warnings, config/test TypeScript passes |
| `npm test` | Exit 0; 7 files, 43 unit tests |
| Focused new parent-retained-value regression before fix | Exit 1; Intent radio incorrectly unchecked |
| `npm run test:browser` after fixes | Exit 0; 26 tests (G03 8, G04 7, G05 4, original harness 7) |
| `npm run build` | Exit 0; production bundle builds |
| Both contract copies vs each other and HEAD | Byte-identical |
| `git diff --exit-code` before review metadata | Exit 0; existing tracked files untouched |
| Untracked ownership, whitespace and conflict-marker scan | Only authorized task/report files; clean |
| Grok report local links | All resolve |

Browser tests use actual Svelte components in Chromium, assert no uncaught page
errors and detect requests leaving the local origin. No mocks replace component
behavior. Unit tests were not repeated after the final G03 component-only fix;
the final component change was covered by type checking, the full browser suite
and production build. No tests were skipped or weakened to make a failure pass.
No Idris/Rust/VM/device work was performed for this presentation-only review.

## Visual evidence

The final full browser run regenerated G03/G04/G05 screenshots in their owned
report directories. Existing G02 evidence stayed untouched. Visually inspected:

- [G03 narrow controls](G03/mobile-390x844-default.png): all five primary choices,
  independent controls, synthetic labels and no clipped layout.
- [G04 value distinctions](G04/desktop-1280x800-redacted-unknown.png): visible empty
  scalar, empty list, redacted and unknown remain different.
- [G04 narrow long values](G04/mobile-390x844-long-text.png): literal HTML-like text,
  partial warning, scrollable value and focus ring.
- [G05 stale-success detail](G05/desktop-1280x800-stale-success-detail.png): raw
  Success alongside Stale, neutral styling, distinct observation/provenance sources.
- [G05 narrow selection](G05/mobile-390x844-selection.png): ordered outcomes and
  wrapped long identifiers/details; page is intentionally long.

Screenshots are full-page captures at 1280- or 390-pixel viewport widths, so their
image height can exceed 800/844. They show development fixtures, not live state.

## Decisions and remaining limitations

- G03 canonicalizes overlays on emitted requests, including unrelated field edits.
  Accepted as enforcement of the packet's canonical, duplicate-free output rule.
  Supplied props are not mutated and rendering alone emits no normalization.
- Null reason/empty-list display placeholders are UI labels, not inferred health.
  In G05, `none` under Missing/Reason/Depends on means no item supplied by these
  props. It does not establish the absence of an operational problem.
- Primitive enum tokens are acceptable in this isolated display; no domain mapping
  or contract extension is needed for this batch.
- The static snapshot/assurance heading IDs and long stacked details should be
  revisited if future integration mounts several instances together. This batch
  verifies the scoped standalone panels, not a completed comparison/product UI.
- Before the next batch, route routine G03/G04/G05 screenshot output to ignored
  test output (as already done for G02), or add an explicit capture option. Their
  current specs write task report images, which must become immutable evidence
  once this batch is committed. This is next-handoff preparation, not a reason
  to start G06/G07/G08 now.
- Compiler semantics, authoritative projection/health, runtime/services, native
  adapter execution, deployment and hardware acceptance remain reserved for Astra.
