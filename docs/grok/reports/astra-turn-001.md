# Astra review — turn 001

Completed 2026-09-17 on `codex/grok-easy-tasks`.

**G01, G02 and G09 accepted after small in-scope fixes. Ready to commit.**
No files were staged or committed by this review. G03/G04/G05 are eligible for
an explicit new user authorization after this reviewed baseline is committed.
No later implementation has started; the automatic goal runner remains paused.

## Exact reviewed state

Base HEAD: `788b486863db9af36e3e932bfed899396c708ac6`.

[Manifest](astra-turn-001-manifest.json) records all 39 implementation, documentation,
Grok report and screenshot files in the reviewed batch, with exact byte hashes,
lengths and executable bits. Manifest SHA-256:

`83703c70aff1d2ae9ef87b12b0cf6c11fb5d0fb67e4fac35eaba76a951de6e60`

The manifest excludes itself, this review and REVIEW-GATES.md to avoid circular
hashes. No other changes are excluded. Acceptance applies to these bytes at this
base, not arbitrary future edits. To reproduce the identity from the repo root:

```python
from pathlib import Path
import hashlib, json, subprocess
p = Path('docs/grok/reports/astra-turn-001-manifest.json')
assert hashlib.sha256(p.read_bytes()).hexdigest() == '83703c70aff1d2ae9ef87b12b0cf6c11fb5d0fb67e4fac35eaba76a951de6e60'
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

## Findings fixed

1. **G02 extension tests blocked future tasks.** The glob test required exactly
   two fixtures, the unknown-route test used future G03's `view-controls` route,
   and keyboard navigation assumed a fixed first link. Tests now require the
   canaries without excluding new fixtures, use a dedicated missing route, and
   Tab to the canary through the discovered navigation. Unit and browser checks
   also passed with a temporary third fixture sorting before both canaries;
   that temporary directory was removed.
2. **G02 engine declaration included unsupported Node versions.** Package and
   lockfile root engines now match the locked toolchain intersection:
   `^20.19.0 || ^22.12.0 || >=24.0.0`. README records the distinction between
   Grok's Node 23 run and this supported Node 24 review.
3. **G02 tests were excluded from TypeScript checks.** Added tsconfig.tests.json
   with DOM types and included it in `npm run check`. Both unit and Playwright
   test source are checked; no assertions were removed to silence errors.
4. **G02 overflow was hidden globally.** Replaced horizontal clipping with text
   wrapping. Added a real-browser long/HTML-like unknown-route test and a failed
   local module load/recovery test. Literal text, error visibility and narrow
   layout pass.
5. **G01 symlink checks missed ancestor directories.** An alternate `lab` or
   `lab/openwrt` symlink could redirect writes despite checking fixtures itself.
   These ancestors are now rejected before indexing/writing. Added tests for
   both ancestors, valid-JSON byte changes and unknown/nested directories.

G09 needed no implementation changes. Its commands match Makefile and its status
claims preserve the compiler, host-test, captured-fixture, stock-ubus permission,
native adapter and physical acceptance distinctions.

## Independently executed checks

Frontend working directory: `frontend/`. Node **24.19.0**, npm **10.9.2**.
Node was selected by prepending the bundled Node bin directory to PATH for these
commands; no global installation or user settings were changed:
`/Users/erikwilliamson/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin`.

| Check | Result |
|---|---|
| `npm ci --cache /tmp/network-intent-npm-review-cache` | Exit 0; clean install, no engine warnings, no legacy-peer-deps flag; Chromium installed |
| `npm ls --all` | Exit 0; no invalid peers; missing optional integrations are not required |
| `npm run check` | Exit 0; Svelte 0 errors/warnings; config and test TypeScript checks pass |
| `npm test` | Exit 0; 9 tests |
| `npm run build` | Exit 0; production bundle built |
| `npm run test:browser` | Exit 0; 7 real Chromium tests, final two-fixture checkout |
| Unit/browser tests with temporary third fixture | Exit 0; 9 / 7 passed; temporary feature removed |
| `python3 -m unittest discover -s tests -p 'test_fixture_index.py'` | Exit 0; 15 tests |
| Index CLI `--write` then `--check` | Both exit 0; original fixtures unchanged |
| Frozen presentation contract comparison | Byte-identical to unchanged source contract |
| All 11 original fixture files against HEAD and index | Exact bytes, hashes and lengths match |
| Lockfile source/integrity inspection | npm registry URLs; integrity hashes present |
| README, guide and task report local links | Resolve on filesystem |
| Tracked diff and untracked text scan | No whitespace errors or conflict markers; ownership in scope |

An initial Node 23 sandboxed `npm ci` failed with npm's “Exit handler never called”
and inability to write its log directory. It is not counted as a pass. The clean
supported-Node install above subsequently succeeded with a writable temporary
cache and normal approved network access. One final browser rerun was initially
rejected because automatic approval review had hit its usage limit. After the
user asked to continue and usage became available, it succeeded. No checks remain
blocked by that interruption.

## Browser evidence and scope

Final regenerated screenshots include
[mobile fixture](G02/mobile-390x844-harness-example.png),
[desktop keyboard focus](G02/desktop-1280x800-keyboard-focus.png), and
[mobile unknown route](G02/mobile-390x844-unknown.png). These three were visually
inspected: readable labels/text, visible focus, no clipping. The final run also
regenerated desktop home/fixture/unknown screenshots. The older
`mobile-390x844-unknown-view-controls.png` remains Grok's historical first-batch
image; that route is no longer used as an unknown-route test.

The browser tests execute the actual rendered Svelte app, fail on uncaught page
errors and record any requests leaving the local origin. Only the intentional
module-failure case intercepts a local module request. This does not validate
future state/assurance/editor components; those are not implemented yet. No
unknown/stale/redacted domain values were converted into authoritative decisions.
No compiler, runtime, identity, device, existing fixture or transcript changed.

## Remaining limitations / next batch

- Native C/Rust adapter execution, deployment and physical hardware acceptance
  remain outside this batch and incomplete. No Idris/Rust/VM suite was rerun.
- Grok disclosed initial lock generation with `--legacy-peer-deps`. The retained
  lock installs normally and has valid peers; no dependency version change is
  needed for this batch. That flag is not an approved future install workflow.
- The old apply-agent README test count remains a known out-of-scope documentation
  issue; G09 correctly cites the historical workspace log instead.
- Keep the two labelled harness canaries for extension regression tests.
- Next eligible batch: **G03 / G04 / G05 only**, after committing this baseline
  and receiving explicit user authorization. G06/G07/G08 remain for a later turn.
