# Original DSL design — historical reference

The original 3,198-line implementation handoff described the initial DSL design,
including language-1.0 examples, early spikes and proposed later milestones.
It is not the current execution plan or a description of all implemented behavior.

Use these maintained documents:

- [Language specification](language.md): current language syntax and semantics.
- [Backend contracts](backends.md): implemented target realization boundaries.
- [Full suite specification](implementation-plan.md): eventual technical scope.
- [Current checkpoint](goal-progress.md): completed work and remaining limits.
- [Current delegated batch](grok/CURRENT.md): permitted next tasks.

The complete original document is retained unchanged in Git at `2774655`:

```sh
git show 2774655:docs/implementation-handoff.md
```

Retrieve it only when investigating historical rationale. Its migration/AAA ideas
and unimplemented milestones must not be treated as current task authorization.
The core principles remain: Idris owns semantics; vendor realization is downstream;
capability failures stay explicit; intended/configured/observed state remain
separate; model proofs are not runtime observations; secrets stay outside pure
compiler artifacts.
