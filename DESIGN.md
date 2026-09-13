# Design and trusted boundary

```text
source -> tokens/CST -> statements (raw AST) -> resolve -> Network V1
       -> validate + DAG certificates -> StableNetwork
       -> capability analysis -> UCI/IOS AST -> generic renderer
                                  |-> Markdown/Mermaid/JSON
```

The parser retains the source, tokens (including comments), source spans, raw
statement fields and block structure. Resolution gives references distinct
`Ref VLAN`, `Ref DeviceKind`, `Ref HostKind`, `Ref ServiceKind`, `Ref RouteKind`
types. `PortRef` includes its owning device. Host `eth0` is an explicit v1
convention; no other implicit host interfaces resolve. References become numeric
keys into immutable inventories, not unresolved names. Certification checks key
bounds even for callers constructing a `Network` programmatically.

`IPv4`, `VlanId`, and `IPv4Prefix` carry erased checked bounds. Prefix size is
proved equal to the power of two implied by its width, and network host bits
must be zero. Arithmetic uses integers followed by checked construction.
IPv4 allocation semantics do not purport to model IPv6. A separate address-family
tag reserves the architectural distinction; IPv6 source syntax is rejected.

`StableNetwork` contains the immutable `Network V1`, service/route ordering
certificates tied to its exact vertices and edges, and erased `So` evidence that
the complete implemented validation pass returns no diagnostics. The public
`certify` entry point repeats all mandatory model checks, independently of the
parser. Downstream compilation accepts this stable type only.

The graph algorithm uses deterministic Kahn traversal and validates coverage,
vertex uniqueness and every edge's rank. Cycles return a closed path in the
direction of source edges. Invalid graph inventories are distinct from cycles.
This certifies the implemented predicates. It is not a separately mechanized
theorem that those predicates fully capture every networking requirement.
Physical links are checked for endpoint consistency and degree, but can cycle.

Trusted production modules use `%default total`; neither `believe_me` nor
`assert_total` occurs in project source. Explicit finite fuel bounds parser work.
Parser source size, token count and nesting are bounded. No project FFI is used.
Only CLI/test I/O is marked `covering`. The Idris compiler, standard library,
Chez backend/runtime and code generator are part of the trusted computing base;
no transitive standard-library proof audit is claimed.

Target compilation creates structured UCI sections/fields or IOS command trees.
`SafeText` rejects line and control characters; UCI rendering quotes values and
escapes apostrophes. Target identifiers originate from checked labels or numeric
semantic IDs. Renderers implement only quoting, indentation and serialization.
Source maps retain originating spans and expansion chains. No raw vendor
configuration or literal secrets can cross the DSL boundary.

`MigrationState debts` contains a certified candidate and a previous stable
model. Introducing debt changes its type; discharge requires evidence matching
the debt class. Candidate replacement recertifies structural and security/model
properties, and `finish` only accepts an empty debt list. Its evidence inputs are
explicit assertions at a future integration boundary, not proofs of DNS health.
The current model has no operational availability checker. The spike demonstrates
typestate and preserves that epistemic limit rather than fabricating observations.

AAA's pure spike distinguishes accepted, rejected and unavailable results,
matches exact permission sets, rejects absent required accounting, and requires
independent recovery. `SecretRef kind` construction accepts an opaque URI only.
No actual AAA commands are emitted. Applied and operational state have distinct
types with timestamps/source labels and explicit `Unknown` evidence.

Algorithmic limits suit home/small-lab networks: conflict and topology checks
are quadratic in small inventories; deterministic graph traversal is O(V(V+E)).
No near-linear performance claim or incremental semantic engine is made.
