# Manual Astra review after a Grok turn

Copy this prompt into an Astra task using the same checkout:

> Review Grok's current uncommitted batch against GROK_HANDOFF.md, its task packets
> and reports. Inspect the actual diff, including new files, and verify the relevant
> checks yourself. Check scope, frozen contract equality, preserved state/error
> distinctions, browser behavior and evidence. Do not start unrelated implementation.
> Fix small issues within the authorized batch; report design gaps or substantive
> rework. Update docs/grok/REVIEW-GATES.md with the exact reviewed state and results.
> Tell me whether the batch is ready to commit and which next batch can be authorized.
> Leave changes uncommitted unless I explicitly request a commit.

Review should establish:

- Only the authorized task and report files changed. New files must be inspected;
  ordinary `git diff` alone omits untracked files.
- The frozen presentation contract has not changed or become backend authority.
- Dependencies, scripts and browser tests run from a clean install. Real browser
  behavior supports UI claims; mocks do not substitute for requested integration.
- Unknown, stale, unavailable, redacted, ordered and read-only states retain the
  distinctions in each packet. No synthetic data is presented as live evidence.
- Test failures, unavailable checks and screenshots are reported accurately.
- Any acceptance is tied to a specific commit or reproducible diff identity.
  Passing tests alone does not accept a task or authorize the following turn.

Suggested later batches remain G03/G04/G05 and G06/G07/G08. Source editor and graph
renderer are the more involved bounded tasks; their history, newline and async
layout behavior deserve focused review. Domain decisions remain reserved for Astra.
