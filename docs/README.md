# Documentation map and retention policy

## Start here — narrow context

- Grok next batch: [grok/CURRENT.md](grok/CURRENT.md), its assigned packet(s), the
  frozen contract and report template. No recursive docs ingestion.
- Fresh Astra review: [grok/ASTRA-REVIEW.md](grok/ASTRA-REVIEW.md).
- Current project state: [goal-progress.md](goal-progress.md).
- Developer commands and evidence distinctions: [developer-guide.md](developer-guide.md).
- Full eventual scope (architecture work only): [implementation-plan.md](implementation-plan.md).

## References — load only for the task at hand

Language/backend work: [language](language.md), [backends](backends.md),
[diagnostics](diagnostics.md), [router settings](router-settings.md),
[secrets](secrets.md), schema JSON, parity and verification documents.
Runtime work: [runtime contracts](runtime-contracts.md),
[assurance standards](assurance-standards.md), [browser compiler](browser-compiler.md),
[OpenWrt lab](openwrt-lab.md).

## Retention review (2026-09-17)

The directory had 83 files and approximately 698 KB of text before this cleanup.
File count and image size do not themselves imply those files must enter an
agent's context. The main problem was stale active instructions and bulk reads.

| Material | Decision | Reason |
|---|---|---|
| Original implementation-handoff.md (3,198 lines / 60,713 bytes) | Compact to historical pointer | Language-1.0 examples and startup spikes are stale; original recoverable with `git show 2774655:docs/implementation-handoff.md` |
| Root Grok handoff, goal-progress, review gates | Replace accumulated chronology with current state | Removes contradictory “not implemented”, “uncommitted”, and old batch launch instructions |
| implementation-plan.md | Keep technical scope/acceptance; replace obsolete activation checklist | Preserves unique requirements without inviting duplicate automation |
| TURN-002.md and completed task packets | Keep at existing paths, historical use only | Reviews refer to exact task scope; current reading list excludes them |
| Grok/Astra reports and manifests | Keep immutable | Audit trail, failures, scope and hashes; moving/deleting breaks traceability |
| Captured screenshots and raw verification logs | Keep immutable, not default reading | Evidence cannot be recreated by rewriting a summary; browser specs must not overwrite committed images |
| progress-history | Keep, excluded from active reading | Historical checkpoints only, not current process/resume instructions |
| schemas 2.0 and 3.0 | Keep both | scripts/check_release.py tests the old-to-new release boundary |
| Language, backend, parity, security and runtime references | Keep | Distinct maintained contracts or evidence; not proven redundant |

No evidence or schema was deleted. Historical file paths remain stable for report
links and manifests. Routine browser captures for completed tasks now use ignored
frontend/test-results; new task specs require an explicit evidence-capture switch.

For future compaction, keep one current batch and one current checkpoint. Once a
batch is committed, update gates with its commit rather than appending another
contradictory launch plan. Archive information through Git-backed pointers only
when its complete original is committed and incoming references still resolve.
Do not erase task reports just to improve context size; control the reading list.

Preparation verification: installed CodeMirror CRLF roundtrip/edit probe and bundled
ELK layout probe passed; `npm run check` and all 26 existing Chromium tests passed.
Committed G02–G05 evidence remained byte-identical. The repository-local Markdown
link audit also found and repaired one pre-existing relative link in progress-history.
This validates the prerequisites, not completion of G06/G07/G08.
