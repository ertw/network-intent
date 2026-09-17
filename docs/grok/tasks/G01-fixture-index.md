# G01 — Deterministic index of existing OpenWrt fixtures

**First-turn task.** This is file inventory and integrity checking, not fixture
regeneration, observation, validation of hardware, or assurance evaluation.

## Read and own

Read `lab/openwrt/fixtures/`, `docs/openwrt-lab.md`, and the existing
`runtime/witness-agent/tests/openwrt_fixtures.rs` for context only.

May create/edit only:

- `scripts/index_openwrt_fixtures.py`
- `tests/test_fixture_index.py`
- `lab/openwrt/fixtures/index.json`
- `docs/grok/reports/G01.md`

Do not modify any existing JSON or raw log. Use only the Python standard library.

## Exact deliverable

Implement a CLI that defaults to checking a committed index:

```sh
python3 scripts/index_openwrt_fixtures.py --write
python3 scripts/index_openwrt_fixtures.py --check
python3 -m unittest discover -s tests -p 'test_fixture_index.py'
```

`--write` may write only `lab/openwrt/fixtures/index.json`. `--check` writes
nothing, exits 0 for an exact match, and exits nonzero with the mismatched path
for missing/extra/changed/invalid files. Resolve the repository from the script
path so commands also work from another cwd. No capture commands or subprocesses.

Index format is fixed: top-level `version: 1`, `releases: [...]`; each release
has `release`, `files`, `unavailable_objects`; each file has its repository-relative
`path`, lowercase hex `sha256` of its exact bytes, and `bytes` byte count.
Releases and file paths are sorted lexically; unavailable objects retain their
order from `capabilities.json`. Enumerate the two known release directories only.
Include every existing `.json` within them, including capabilities. Exclude the
index itself. Serialize UTF-8, indent 2, final newline, without dates or host paths.

Validate `capabilities.json` as an object with `release` equal to its directory
name, a string `collection`, and a duplicate-free string array
`unavailable_objects`. For these fixtures only `network.wireless` may be unavailable;
its listing requires `netifd-wireless.json` to be absent, and otherwise that file
must exist. Require capabilities plus uci-network, uci-changes, netifd-interfaces
and netifd-devices JSON files in each release. Validate the index structure and
compare it with the freshly generated canonical bytes in check mode.

Parse each JSON strictly: reject duplicate object keys and non-finite numeric
constants. Reject symlinks, unexpected nested directories, or unknown release
directories rather than following them. Preserve the distinction between a missing
wireless object on 25.12.5 and a valid empty wireless object on 24.10.8. Do not
synthesize missing responses or turn this index into a health assertion.

## Steps

1. Inventory the committed fixture paths and inspect capabilities.
2. Write small pure index-building/checking functions that accept a root path;
   tests use temporary directories, never the real fixtures for mutation tests.
3. Implement the CLI with explicit diagnostics and mutually exclusive flags.
4. Write the index once from the unchanged real files, then run `--check`.
5. Run the tests, inspect the diff, and report exact commands and outcomes.

## Required tests / acceptance

- Two consecutive generations produce identical bytes, with no clock/path noise.
- A modified byte, missing file, extra JSON, wrong release label and malformed
  capabilities produce a check failure with the relevant path.
- Duplicate JSON keys, NaN and symlink input are rejected.
- The two wireless capability cases remain distinct.
- Running `--check` does not modify any fixture or index bytes.
- Only owned files changed; all original fixture hashes remain unchanged.

Stop for Astra if the current fixture data contradicts its capability metadata.
Do not repair or recapture it under this task.
