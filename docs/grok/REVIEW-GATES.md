# Astra review gates

Only Astra, when explicitly asked by the user, may update this file. Grok reads
it but must not edit it. The global goal remains incomplete regardless of these
component approvals.

| Task | Status | Reviewed commit/diff | Notes |
|---|---|---|---|
| G01 | Not submitted | — | First-turn task |
| G02 | Not submitted | — | Required before G03–G08 |
| G03 | Not authorized this turn | — | Await G02 review |
| G04 | Not authorized this turn | — | Await G02 review |
| G05 | Not authorized this turn | — | Await G02 review |
| G06 | Not authorized this turn | — | Await G02 review |
| G07 | Not authorized this turn | — | Await G02 review |
| G08 | Not authorized this turn | — | Await G02 review |
| G09 | Not submitted | — | First-turn task |

On each review, Astra checks the changed code and actual tests, the ownership
boundary, mock/authoritative separation, and any material untested behavior.
Possible outcomes: Accepted; Changes requested; Blocked on design. Record the
reviewed HEAD or exact diff identity and authorize the next batch explicitly.
No approval of these tasks enables deployment or hardware profiles.
