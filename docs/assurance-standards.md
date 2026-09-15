# Assurance and device-integration standards notes

This is an implementation reference, not evidence that these features are already implemented. Sources were checked on 2026-09-14. RFC dates below are publication dates.

## Requirements derived from the RFCs

| Source | Relevant distinction or capability | Requirements for this project |
|---|---|---|
| [RFC 8342 (March 2018)](https://www.rfc-editor.org/rfc/rfc8342.html) | NMDA models separate conceptual datastores. `running`, `candidate`, and `startup` are configuration datastores; `intended` is validated configuration intended to be in effect; `operational` contains actually used configuration plus state. Values can lag, and operational data can contain remnant configuration during convergence. | Persist and label datastore/view provenance on every read. Keep intended, device configuration/readback, and operational observations distinct. Never treat a successful configuration write as proof of operational use. Model convergence, remnant data, freshness, and origin explicitly. |
| [RFC 9144 (December 2021)](https://www.rfc-editor.org/rfc/rfc9144.html) | Defines an NMDA `<compare>` RPC with `source`, `target`, optional subtree/XPath filter, `all`, and `report-origin`; differences are YANG Patch edits and can include `source-value`. Intended-versus-operational differences are expected during propagation and are diagnostically useful. | Implement Compare as a typed, bounded diff: source/target datastore IDs, filter, graph/revision IDs, patch edits (`operation`, target path, value, source value), and optional origin. Preserve list ordering and keys. Rate-limit expensive comparisons and authorize access; inaccessible subtrees must not be mistaken for equality. |
| [RFC 7950 (August 2016)](https://www.rfc-editor.org/rfc/rfc7950.html) | YANG 1.1 is a schema language for configuration, state, RPCs, actions, and notifications. Modules are versioned by revisions; constraints can be enforced by client or server; schema node order matters in YIN mappings. | Treat schema/module revision, namespace, feature/deviation set, type constraints, defaults, `config true/false`, keys, `must/when`, and list order as part of a device profile. Validate before compilation and record the exact schema identity used for each witness. Do not assume arbitrary JSON is a valid YANG instance. |
| [RFC 6241 (June 2011)](https://www.rfc-editor.org/rfc/rfc6241.html) | NETCONF is a secure, connection-oriented RPC protocol separating configuration and state and exposing capabilities. It defines candidate/running/startup operations and validation/commit semantics. Updated by RFC 8526. | Reuse the same state vocabulary even for ubus. Make target/source datastore, capability/profile, authenticated session, request ID, validation result, and server error part of receipts. Keep transport authorization separate from semantic admission. |
| [RFC 8526 (March 2019)](https://www.rfc-editor.org/rfc/rfc8526.html) | NMDA NETCONF adds `get-data`/`edit-data`, datastore identity references, operational support, and YANG Library discovery. A compliant server exposes a YANG Library content ID and clients can cache until it changes. | Device profiles must include supported datastore identities, YANG library content ID, module revisions, features, deviations, and capability evidence. Invalidate cached compilation/witness assumptions when content ID or profile changes. Use explicit datastore identity in read and compare requests. |
| [RFC 8525 (March 2019)](https://www.rfc-editor.org/rfc/rfc8525.html) | YANG Library describes module sets, schemas, datastore-to-schema mapping, and a server-generated `content-id`; a `yang-library-update` notification signals change. A datastore has exactly one schema reference. | Store `content_id`, schema name, module name/revision/namespace/location, submodules, features, deviations, and datastore mapping. Include a library snapshot/hash in evidence. Missing or stale library data makes capability coverage unknown, never successful. |
| [RFC 9417 (July 2023)](https://www.rfc-editor.org/rfc/rfc9417.html) | SAIN is a holistic service-assurance architecture: configuration alone does not establish service health. Agents build an expression/dependency graph, correlate symptoms to subservices, and support service-impact analysis across heterogeneous protocols/models. | Represent assurance as a versioned dependency graph. Bind every check to the subservice, target, scope, and evidence source; retain symptoms and impacted services. Separate configuration acceptance from health. A reachable IP endpoint is insufficient evidence for an application-level claim. |
| [RFC 9418 (July 2023)](https://www.rfc-editor.org/rfc/rfc9418.html) | Defines NMDA-conformant YANG assurance graphs: configurable subservices/dependencies and read-only health/status/symptoms. Dependencies are keyed references; circular dependencies must be rejected. Graph history is outside the model and needs separate storage. | Validate graph IDs, node types, dependency types, referenced IDs, acyclicity, agent bindings, health-score explanations, and current graph version. Store graph revisions and symptom history separately from the current graph. Unknown, incomplete, or agent-unbound subservices cannot satisfy coverage. |

### Evidence and comparison fields

At minimum, each observation and generated diff should carry: `device_id`, `profile_id`, `schema/library_content_id`, `datastore` (`intended`, committed/configuration readback, or `operational`), `scope`/filter, `source`, `collected_at`, `observed_at` if supplied by the device, `fresh_until`, completeness, redaction status, request/session ID, revision/hash, and error/timeout class. Each assurance result should additionally carry `graph_version`, `subservice_id`, dependency inputs, expected value/range, evidence IDs, health/status, symptoms, and evaluator version.

Do not collapse these states:

* **Intended:** validated target configuration the controller wants in effect.
* **Staged:** session-local edits not yet committed or applied.
* **Committed/configuration readback:** persisted device configuration as read through the device API.
* **Operational:** runtime state and configuration actually in use; it can lag or contain remnants.
* **Apply receipt:** what the device API accepted, including transaction/timer status.
* **Independent observation:** a separate witness result, with its own scope and freshness.

An absent node, filtered node, unauthorized node, timed-out query, and unsupported node need distinct outcomes. `unknown` or `stale` must never be converted to healthy/successful.

## OpenWrt native ubus/rpcd and UCI contract

Primary references: [OpenWrt ubus technical reference](https://openwrt.org/docs/techref/ubus), [rpcd developer documentation](https://openwrt.org/docs/guide-developer/rpcd), [UCI technical reference](https://openwrt.org/docs/techref/uci), and the [rpcd `uci.c` source](https://git.openwrt.org/project/rpcd/tree/uci.c). The wiki warns that the older ubus UCI table is incomplete/outdated; use the installed rpcd object signatures and source for exact target-version behavior.

* ubus is the native IPC/RPC bus. `rpcd` supplies built-in `session` and `uci` plugins; `netifd` exposes `network` objects. Use ubus methods through an authenticated, least-privilege session and record the object/method/signature actually used.
* A UCI session is not the committed configuration. With `ubus_rpc_session`, rpcd stores staged deltas under `/tmp/run/rpcd/uci-<session>` (the no-session default is `/tmp/.uci`). A read must declare whether it reads committed `/etc/config` state or session staging. A staged read must include the session ID and delta set; a committed read must not accidentally inherit another session’s staged changes.
* The rpcd UCI object includes read/write operations plus `commit`, `revert`, `apply`, `confirm`, `rollback`, and `reload_config`. `commit` persists package configuration; it is distinct from reloading services and from a timed apply transaction. `revert` discards staged changes subject to session permissions.
* `apply` snapshots configuration, applies it, and arms a rollback timer (the rpcd implementation default is 10 seconds unless a timeout is supplied). `confirm` cancels the pending rollback; `rollback` restores the snapshot. The implementation checks the initiating session and denies conflicting apply/rollback operations. Treat timeout, confirm, rollback, and session loss as explicit transaction states, and journal them durably outside the router before claiming recovery.
* OpenWrt’s LuCI flow may use a separate HTTP endpoint around ubus, and LuCI also maintains frontend-local staging. The apply-agent must call and verify the native rpcd/ubus contract directly rather than infer behavior from LuCI UI calls.
* For read-only observation, restrict ACLs to UCI `configs`/`get` (and any exact read methods required), device/interface/netifd state reads, and session inspection. Do not grant `set`, `add`, `delete`, `commit`, `apply`, `confirm`, or `rollback` to the witness identity. Redact secrets and sensitive options before persistence, export, or telemetry.
* The `network.interface` and `network.device` operational trees describe runtime state supplied by netifd; they are not proof of UCI persistence. Collect them as operational evidence with timestamps and completeness, independently of UCI configuration reads.

### Required transaction checks

1. Before a read or write, record the ubus endpoint, rpcd version/build, session identity, ACL decision, package scope, and whether the read is committed or staged.
2. Before apply, hash the exact staged delta and committed baseline; reject ownership conflicts and unexpected intervening deltas.
3. On apply, record timeout, snapshot/transaction ID if available, affected packages, and the exact acceptance response.
4. During the confirmation window, perform independent management-path and service/interface checks for at least two successful rounds; confirm only after those checks pass.
5. On timeout, restart, session disappearance, or failed verification, classify the result as rollback/uncertain until a fresh committed read plus independent operational observation establishes the outcome.

## Pinned VM image availability (verified, not downloaded)

Both requested x86/64 release indexes are live as of 2026-09-14 and publish image SHA-256 values plus detached signature locations:

* [OpenWrt 25.12.5 x86/64 index](https://downloads.openwrt.org/releases/25.12.5/targets/x86/64/), published files dated June 30, 2026. Examples: `generic-squashfs-combined.img.gz` SHA-256 `4b12d707ad34cdad0e53a6191e8c69eb6c0c09bfa6f9a0a05a65fc64a49096df`; `generic-ext4-combined.img.gz` SHA-256 `23e2538e8ab0eb52dfed1c65d608ecdb71ffd432dd54885da138ae67cd9e4461`.
* [OpenWrt 24.10.8 x86/64 index](https://downloads.openwrt.org/releases/24.10.8/targets/x86/64/), published files dated July 25, 2026. Examples: `generic-squashfs-combined.img.gz` SHA-256 `8e5baa948e3244a2a5d3fd1a70557c3fcd7ded7cf5e5d5fe12e90bc685326a41`; `generic-ext4-combined.img.gz` SHA-256 `23872c64fdb66d0765e0d17f71453a66e7a91e9ecb10606d5f738e6d166e14ae`.

Each index lists `sha256sums`, `sha256sums.asc`, and `sha256sums.sig`. Verify the image hash against `sha256sums`, then verify that checksum manifest with the OpenWrt release signing key before using a VM. Availability does not establish that a particular image boots in the project’s hypervisor or that ubus/rpcd package behavior is identical across builds; those remain lab acceptance checks. No image was downloaded or device altered for this research.

## Pitfalls and unresolved facts

* RFC 9417/9418 define an assurance architecture/model, not the project’s probe semantics, thresholds, persistence policy, or cryptographic evidence format; those remain local contracts.
* RFC 8342/9144 terminology should not be used to imply that UCI has native NMDA datastores. Map OpenWrt’s session staging, committed files, and netifd runtime into explicit project states.
* rpcd’s apply timer and behavior are implementation/version dependent. The source documents a default 10-second timer; the requested 180-second controller confirmation window must be enforced by the apply agent as a higher-level policy and tested against the target image.
* The OpenWrt wiki’s UCI/ubus reference is partly outdated and was intermittently protected by anti-bot pages during research. Pin exact rpcd/uci/netifd package revisions in VM fixtures and retain command/object introspection output as acceptance evidence.
* The release indexes prove artifact availability and provide checksum/signature paths; they do not prove signature verification, boot success, package presence, or hardware compatibility. Those facts remain unresolved until the lab performs them.
