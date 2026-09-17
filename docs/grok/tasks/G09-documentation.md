# G09 — Accurate developer status and commands

**First-turn task.** Documentation only, based on existing files and recorded
verification. Do not run VM/device/build experiments to obtain new claims.

Own only `README.md`, new `docs/developer-guide.md`, and
`docs/grok/reports/G09.md`. Do not edit goal-progress, implementation-plan,
review gates, other handoffs, runtime contracts, existing evidence or code.

## Read first

`GROK_HANDOFF.md`; `Makefile`; Cargo workspace/package manifests;
`docs/browser-compiler.md`; `docs/runtime-contracts.md`; `docs/openwrt-lab.md`;
`runtime/witness-agent/NATIVE-UBUS.md`; `runtime/apply-agent/README.md`;
`docs/verification-runs/runtime-integration-2026-09-16.log`.

## Exact edits

1. Preserve valid compiler instructions and language examples in README.
2. Update its obsolete blanket statements that live observation/deployment are
   outside the whole project: distinguish the existing compiler release from the
   in-progress broader suite. Do not claim services/deployment are ready.
3. Add a concise status section linking the full objective, current progress,
   developer guide, OpenWrt lab, and bounded Grok handoff. Mention that the current
   router examples remain admission-blocked and hardware acceptance is incomplete.
4. In the guide, document existing commands `make build`, `make test`,
   `make test-browser`, and `make test-runtime` / `cargo test --workspace`, their
   required local toolchains, and what each verifies. Preserve Rust minimum 1.89
   and Idris 2 0.8.0 as recorded; do not invent OS support or installation recipes.
5. Separate source/compiler checking, runtime host tests, captured VM response
   fixtures, stock-ubus permission tests, native adapter execution, and physical
   acceptance. State exactly which are verified and which are pending.
6. Document that the upcoming `frontend/` is a fixture component harness awaiting
   Astra integration. Because G02 runs in parallel, link its README once it exists
   and label the work in progress if it does not. Do not claim its tests passed
   based only on this task packet. The coordinator reconciles the final wording.
7. Explain contribution flow: bounded Grok batch, uncommitted diff + actual check
   results, user-triggered Astra review. Do not resume the automatic goal runner.

## Verification / acceptance

Check every new repository-relative Markdown link against the filesystem. Compare
command names with current Make/package files; label unavailable future commands
as pending rather than executable. Search edited docs for unqualified "complete",
"safe", "verified", "exactly once" and "production ready" and justify/remove any
unsupported claim. Do not change historical evidence counts. Record which files
support each new verification claim in the report. `git diff --check` must pass.

No broad rewrite of the language specification, new runtime architecture, external
research, roadmap expansion, or invented screenshots. Report contradictions for
Astra rather than reconciling them by changing code or the ultimate objective.
