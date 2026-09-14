# Design and trusted boundary

```text
source -> tokens/CST -> statements (raw AST) -> resolve -> Network V3
       -> validate + DAG certificates -> StableNetwork
       -> capability analysis -> UCI/IOS AST -> generic renderer
                                  |-> Markdown/Mermaid/JSON
```

The parser retains the source, tokens (including comments), source spans, raw
statement fields and block structure. Resolution gives references distinct
`Ref VLAN`, `Ref DeviceKind`, `Ref HostKind`, `Ref ServiceKind`, `Ref RouteKind`
types. `PortRef` includes its owning device. Host `eth0` is an explicit language-3.0
convention; no other implicit host interfaces resolve. References become numeric
keys into immutable inventories, not unresolved names. Certification checks key
bounds even for callers constructing a `Network` programmatically.

`IPv4`, `VlanId`, and `IPv4Prefix` carry erased checked bounds. Prefix size is
proved equal to the power of two implied by its width, and network host bits
must be zero. Arithmetic uses integers followed by checked construction.
IPv4 allocation semantics do not purport to model IPv6. `Router/Address.idr` defines independently checked 128-bit IPv6 addresses and
canonical prefixes. IPv6 interface addresses may retain host bits; delegated
addresses/prefixes are not fabricated during compilation.

`StableNetwork` contains the immutable `Network V3`, service/route ordering
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
Explicit bridge/bond attachment graphs must be acyclic; certification reruns
the DAG checker and enforces single-master membership. Logical interfaces may
share a master, but cannot bind to enslaved members.

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
configuration or literal credentials can cross secret-bearing DSL fields.
Nonsecret strings are not scanned for secrets. `SecretOption` is a dedicated
UCI AST constructor carrying an opaque `SecretRef WiFiCredential` and security
mode. Rendering emits deterministic placeholders, a versioned binding manifest,
and explicit readiness metadata. No secret materialization takes place. See
[the secrets methodology](docs/secrets.md).

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
use bounded polynomial algorithms over small inventories; deterministic graph traversal is O(V(V+E)).
No near-linear performance claim or incremental semantic engine is made.


## Explicit router model

Language 3.0 is the only accepted source version. There is one current semantic
model, with VLAN shorthand and explicit routing as distinct capabilities, not
version-specific paths. A router cannot mix their port-ownership models.

`Router/Model.idr` separates physical links, bridges, bonds, logical interfaces,
firewall zones, radios, and APs. Device-local `RouterRef kind` values cannot be
interchanged across entity kinds. A logical interface references an attachment;
a zone references interfaces; an AP references a radio and bridged interface.
No resolved topology reference remains an unresolved string.

`Router/Options.idr` uses finite enums and typed optional record fields. Parsers
reject unknown keys and duplicate scalar settings. `Router/CheckOptions.idr`
and `Router/Validate.idr` repeat scalar and cross-entity checks at public
certification, including callers constructing models directly. There is no raw
UCI extension or arbitrary option bag. `Router/Compile.idr` maps the certified
model to the same UCI AST and renderer used by VLAN shorthand.

The backup parity oracle is intentionally outside production code. It parses
sanitized UCI fixtures and independently compares every option and rule order
with CLI output. It neither imports UCI into the semantic model nor implements
any source-language semantics in Python. The compiler does not read the backup.


## Gateway and satellite model (3.0)

Device configuration and routing ownership are independent. Explicit networking
can belong to a non-enforcing `device`; `router` remains the sole shorthand
policy owner. `WiFiInterface` models AP and station roles. `WirelessLink` holds
`WiFiRef` endpoints, each with a typed device reference and a `RouterRef WiFiEntity`.
Cross-device checks are in `NetDSL/Wireless.idr` and run at public certification,
including for programmatically constructed models.

Finite connected-component expansion over declared wireless interface links
allows segment-level address and active DHCP conflict checks across multiple
hops. No certificate claims actual association or the completeness of physical
connectivity. Unattached interfaces have a distinct `Unattached` constructor;
they never alias loopback or invent a physical device. Inactive DHCP pools retain
checked endpoints but contribute zero active leases to allocation and capacity.
