# Browser compiler

`src/NetDSL/Compiler.idr` is the single pure compiler boundary. It accepts a
filename and the exact source string, parses, elaborates, and compiles every
declared device with the standard 4094 tagged-VLAN limit. It returns either
structured diagnostics or the certified model and target realizations.

`evaluateSource` renders deterministic JSON with five fields:
`ok`, `model`, `targets`, `witnesses`, and `diagnostics`. The witnesses preserve
explicit coverage blockers; compiler success is not revision admission.
Browser code calls the Idris
JavaScript build's synchronous `globalThis.netcEvaluate(source)`. The filename
is deliberately fixed to `browser.net` there, so diagnostics remain stable
without exposing filesystem access. This registration is the only JavaScript
foreign call; parsing and compiler decisions are not duplicated in JavaScript.

Build the browser artifact with `idris2 --build-dir build/browser --build
browser.ipkg`; this keeps JavaScript output separate from native artifacts. The
generated artifact is intended for a browser-like global environment and does
not use filesystem or process APIs. `scripts/check_browser_parity.py` checks
its JSON against a temporary native program importing the same compiler module.
The wrapper, source-link, build outputs, and executable all live under a
temporary directory, and each compiler or evaluator subprocess has a timeout.
