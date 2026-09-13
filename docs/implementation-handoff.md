# Network Intent DSL — Implementation Handoff Specification

## 1. Project goal

Build a vendor-neutral, strongly validated network-intent DSL with:

- an ergonomic external syntax intended to be pleasant for humans to read and edit;
- an Idris 2 semantic core;
- rich domain-specific diagnostics;
- a proof-carrying normalized network model;
- multiple target backends, initially OpenWrt/UCI and Cisco IOS;
- generated documentation and topology views;
- explicit modeling of AAA;
- first-class migrations and temporarily degraded states;
- automatic language/schema version enforcement;
- clean separation between desired, intended, applied, and observed state;
- optional browser/editor integration using the same semantic implementation.

The DSL should express **network intent**, not vendor commands.

Example target experience:

```text
network-language 1.0

network home {
  domain home.arpa

  vlan trusted 10 {
    subnet 10.0.10.0/24
    gateway +1
    dhcp +100 .. +199
  }

  vlan servers 20 {
    subnet 10.0.20.0/24
    gateway +1

    host nas            +10
    host hypervisor     +11
    host home-assistant +20
  }

  vlan iot 30 {
    subnet 10.0.30.0/24
    gateway +1
    dhcp +100 .. +240
  }

  router gateway {
    driver openwrt

    port lan1 { access trusted }
    port lan2 { access servers }
    port lan4 { trunk trusted servers iot }
  }

  switch core {
    driver cisco-ios

    port Gi1/0/1 {
      trunk trusted servers iot
      connect gateway.lan4
    }

    port Gi1/0/2 {
      access servers
      connect nas.eth0
    }
  }

  policy {
    trusted -> internet allow
    trusted -> servers  allow

    iot -> internet allow
    iot -> gateway allow dns, ntp
    iot -> trusted deny
    iot -> servers deny
  }
}
```

This file should simultaneously function as:

1. human-readable documentation;
2. a machine-readable source of truth;
3. input to a strongly validating compiler;
4. input to target-specific configuration generators;
5. input to documentation and visualization generators.

The system must **never silently weaken intent** merely because a target device is incapable of implementing it.

---

# 2. Guiding principles

## 2.1 Intent is distinct from implementation

Generic DSL constructs should describe semantic concepts:

```text
port Gi1/0/2 {
  access servers
}
```

not implementation:

```text
switchport mode access
switchport access vlan 20
```

Vendor commands belong below the semantic boundary.

Similarly:

```text
iot -> gateway allow dns
```

should not directly encode nftables, UCI firewall rules, Cisco ACL syntax, or any other particular realization.

---

## 2.2 Strong normal state; explicit abnormal state

A normal network configuration must satisfy all mandatory steady-state invariants.

Do not weaken `ValidNetwork` merely because migrations sometimes require broken intermediate states.

Instead model:

```text
StableNetwork
```

and:

```text
MigrationState outstandingObligations
```

as different semantic states.

A temporarily missing DNS service during a migration is not an invalid `StableNetwork`; it is a valid **transitional network carrying a named DNS availability obligation**.

---

## 2.3 Make invalid states unrepresentable when practical

Prefer structural types over after-the-fact validation.

If every port has exactly one owning device, encode:

```idris
record Port where
  owner : DeviceRef
```

instead of constructing an arbitrary `Owns` graph and later proving each port has exactly one owner.

Use graph proofs only for genuinely graph-shaped properties such as:

- reachability;
- acyclicity;
- connectivity;
- dependency termination;
- spanning relationships;
- routing dependencies.

---

## 2.4 Proofs do not replace observation

The system must distinguish at least four levels of assurance:

### Structural

The model is well-formed.

Examples:

- references resolve;
- VLAN IDs are valid;
- addresses are syntactically and semantically valid;
- no impossible graph edge exists.

### Semantic

The desired network satisfies our own declared invariants.

Examples:

- DHCP pool does not overlap static assignments;
- required DNS paths exist in our model;
- route dependency graph terminates;
- AAA recovery requirements are satisfied.

### Realization

According to our model of a vendor's capabilities, a generated target configuration implements the desired semantics.

This is conditional on our backend and capability model being correct.

### Operational

The real network has been observed to exhibit the required behavior.

Examples:

- an interface is actually up;
- DNS actually answers;
- RADIUS actually authenticates;
- a route is actually installed;
- the target actually accepted and applied the intended configuration.

Do not describe realization or operational correctness as a theorem merely because the internal model was proven valid.

RFC 8342 makes a closely related distinction between intended configuration, applied configuration, and operational state.

---

# 3. High-level architecture

The desired pipeline is:

```text
                    DSL source
                        │
                        ▼
                 concrete syntax
                        │
                        ▼
                     Raw AST
                        │
                 name resolution
                        ▼
                  Resolved Model
                        │
            semantic elaboration
                        │
                        ▼
             Certified Intent IR
                        │
          ┌─────────────┼───────────────┐
          ▼             ▼               ▼
       OpenWrt        Cisco           Docs
       compiler       compiler        compiler
          │             │
          ▼             ▼
       UCI AST         IOS AST
          │             │
          ▼             ▼
       renderer       renderer
          │             │
          └──────┬──────┘
                 ▼
          Intended Config
                 │
                 ▼
          deployment layer
                 │
                 ▼
        Accepted/Applied state
                 │
                 ▼
        observation adapters
                 │
                 ▼
        Operational snapshot
                 │
                 ▼
            drift analysis
```

The compiler and deployment engine should be separable components.

It must be possible to:

```console
netc check home.net
netc compile home.net --target gateway
netc compile home.net --target core
netc docs home.net
```

without giving the tool access to any real device.

---

# 4. Idris 2 role

Idris is the semantic implementation language, not necessarily the syntax users directly edit.

The semantic core should contain:

- domain types;
- smart constructors;
- typed references;
- graph structures;
- invariant definitions;
- decision procedures;
- proof certificates;
- elaboration;
- migration semantics;
- target capability checking;
- target-independent compiler logic.

Idris 2's core is based on quantitative type theory, including erased (`0`) quantities, which is useful for proof fields that need not survive at runtime.

Important caveat: Idris's own documentation currently warns against relying unquestioningly on its totality checker for proofs and exposes mechanisms such as `assert_total`. Therefore, Idris and its compiler remain part of the trusted computing base.

Project policy should therefore be:

```text
%default total
```

in trusted semantic modules where feasible, and:

- no `believe_me` in the trusted core;
- no `assert_total` in the trusted core unless explicitly reviewed and documented;
- isolate FFI;
- isolate parser I/O and deployment I/O from the proof-heavy core;
- clearly mark anything that expands the trusted computing base.

Do not market this system as machine-verified vendor networking merely because the implementation uses Idris.

---

# 5. Parsing architecture

## 5.1 Authoritative parser

The authoritative parser must be part of the compiler implementation and produce an Idris AST.

It should:

- retain precise source spans;
- emit domain-friendly parse errors;
- support comments;
- distinguish identifiers, strings, numeric literals, addresses, prefixes, ranges, references, and operators;
- preserve enough syntax information for formatting or source rewriting.

The parser should never directly produce the final certified IR.

Pipeline:

```text
text
 ↓
tokens/CST
 ↓
Raw AST
 ↓
resolve
 ↓
elaborate
 ↓
Certified IR
```

---

## 5.2 Browser/editor parser

Tree-sitter may be useful for:

- incremental parsing;
- syntax highlighting;
- folding;
- rough symbol extraction.

Tree-sitter must **not** become the source of truth for language semantics.

Editor parsing and authoritative parsing may temporarily disagree while text is incomplete.

Maintain a conformance corpus so that complete valid inputs accepted by one parser are accepted by the other.

Prefer:

```text
Tree-sitter:
    responsive editor syntax

Idris compiler:
    authoritative parse + semantics
```

rather than attempting to reproduce Idris semantic reasoning in Tree-sitter.

---

## 5.3 Source provenance

Every AST and derived semantic object that can produce a diagnostic should retain provenance.

Conceptually:

```idris
record Located a where
  constructor At
  span  : SourceSpan
  value : a
```

For generated objects, provenance may be a chain:

```text
generated firewall rule
← expansion of service dns
← policy iot -> gateway
← home.net:82:5
```

This is critical for good diagnostics after several compilation passes.

---

# 6. Language design

## 6.1 No arbitrary embedded programming language in v1

Do not make the DSL Turing-complete.

We want:

- easy static analysis;
- deterministic expansion;
- predictable diagnostics;
- reliable migration/version analysis;
- editor tooling.

Initially support declarative constructs such as:

- named sets;
- port ranges;
- groups;
- simple templates if necessary;
- selectors;
- relative address allocation.

Do not add arbitrary user functions/macros until concrete use cases require them.

---

## 6.2 Identifiers and namespaces

Decide explicitly whether names share a namespace.

Recommended model:

```text
vlan servers
host servers
```

may coexist if references are contextually typed, but diagnostics must make ambiguity obvious.

Typed references should resolve according to expected kind:

```text
access servers
```

requires a `VlanRef`.

```text
connect servers.eth0
```

requires a device/host interface reference.

Internally, never carry unresolved strings beyond name resolution.

---

## 6.3 Address syntax

Parse IP addresses and CIDR prefixes directly into semantic structures.

Do not retain `"10.0.20.0/24"` as the primary representation.

Suggested types include:

```text
IPv4Address
IPv6Address
IPAddress
IPv4Prefix
IPv6Prefix
Prefix
MACAddress
VlanId
TCPPort
UDPPort
ASN
MTU
```

IPv4 octets and prefix lengths should have bounded representations.

For example, conceptually:

```idris
record IPv4 where
  constructor MkIPv4
  a : Fin 256
  b : Fin 256
  c : Fin 256
  d : Fin 256
```

A canonical network prefix should additionally establish that host bits are zero.

---

## 6.4 Relative addresses

Support ergonomic address-relative notation:

```text
vlan servers 20 {
  subnet 10.0.20.0/24

  gateway +1
  host nas +10
}
```

`+10` means an offset from the prefix's network address, not “last octet 10.”

This must work correctly for prefixes other than `/24`.

Errors should explicitly distinguish:

- offset outside prefix;
- network address;
- broadcast address where applicable;
- reserved address according to policy;
- collision.

---

## 6.5 IPv4 and IPv6 must not be falsely unified

Do not design an abstraction based purely on IPv4 assumptions.

IPv6 introduces:

- multiple addresses per interface;
- link-local addressing;
- router advertisements;
- SLAAC;
- DHCPv6;
- privacy addresses;
- prefix delegation;
- NDP;
- multicast-heavy control traffic.

Generic `Address`, `Assignment`, and `Prefix` abstractions may exist, but address-family-specific semantics should remain explicit.

Do not assume:

```text
gateway +1
DHCP +100..+199
```

is a universal allocation mechanism.

---

# 7. Core semantic model

## 7.1 Prefer structured records over a completely generic graph

Conceptually the network is a graph, but do not implement the entire semantic IR as:

```text
List Node
List Edge
```

unless necessary.

Prefer structures such as:

```text
Network
 ├─ Devices
 │   └─ Interfaces
 ├─ VLANs
 │   └─ Prefixes
 ├─ Hosts
 │   └─ Interfaces
 ├─ Services
 ├─ Routes
 ├─ Policies
 ├─ AAA
 └─ VPNs
```

Then derive graph views from those structures.

This allows simple invariants such as “a port has exactly one owner” to be structurally true.

---

## 7.2 Typed graph relations

Where generic graph relations are appropriate, edge kinds must constrain endpoints.

Conceptually:

```idris
data Relation : NodeKind -> NodeKind -> Type where
  Owns       : Relation Device Port
  Link       : Relation Port Port
  Carries    : Relation Port VLAN
  HasPrefix  : Relation VLAN Prefix
  AttachedTo : Relation Host VLAN
  Runs       : Relation Host Service
  RoutesTo   : Relation Prefix Prefix
```

A nonsensical relation such as:

```text
Host --Owns--> VLAN
```

must be unconstructable.

---

## 7.3 Graph views

Different relationships have different valid topology.

Do not apply a single “acyclic graph” rule globally.

Expected examples:

```text
physical links:
    cycles may be valid

ownership:
    forest

containment:
    forest/DAG

recursive route dependencies:
    DAG

service dependencies:
    often DAG but possibly policy-dependent

security trust:
    possibly cyclic, but certain cycles may be prohibited

communication policy:
    arbitrary directed graph
```

Provide view functions such as:

```idris
physicalGraph    : Network -> Graph PortRef
ownershipGraph   : Network -> Graph EntityRef
dependencyGraph  : Network -> Graph ServiceRef
routeGraph       : Network -> Graph RouteRef
trustGraph       : Network -> Graph EntityRef
```

---

# 8. Proof strategy

## 8.1 Prefer certificate-producing algorithms

Do not force the elaborator to derive huge graph theorems directly.

Write ordinary algorithms that return either:

- a useful counterexample; or
- a certificate from which the desired property follows.

Examples:

### Acyclicity

Run topological sort.

Success produces a topological ordering.

Certificate:

```text
every edge goes from lower rank to higher rank
```

This proves no directed cycle can exist.

Failure produces the actual cycle for diagnostics.

### Connectivity

Produce a spanning tree or forest.

Certificate establishes:

- every required node is covered;
- every tree edge exists in the source graph.

### Prefix overlap

Sort ranges and detect an overlapping pair.

Success gives a uniqueness/non-overlap certificate.

Failure gives the conflicting prefixes and their declarations.

---

## 8.2 Certified elaboration boundary

Aim for an API conceptually like:

```idris
elaborate :
  RawNetwork ->
  Either
    (List Diagnostic)
    (g : Network ** ValidNetwork g)
```

After this boundary, downstream pure compilers may assume steady-state invariants.

For migrations use a different result type rather than weakening `ValidNetwork`.

---

# 9. Addressing and IPAM invariants

The initial validation system should cover:

- VLAN ID range;
- prefix syntax;
- canonical network addresses;
- valid host membership;
- duplicate static IPs;
- DHCP pool membership;
- static/DHCP overlap;
- gateway membership;
- overlapping prefixes unless explicitly permitted by VRF/context;
- prefix ownership;
- address-family compatibility;
- reserved address rules;
- multiple assignments where explicitly supported;
- route next-hop membership/reachability according to configured semantics.

Usable VLAN IDs should normally be 1–4094. VLAN 1 may be semantically special on many targets and therefore should be a policy/backend concern rather than globally forbidden.

---

# 10. Layer-2 topology

Model at least:

```text
physical device
physical port
logical interface
bridge
LAG / port-channel
VLAN
access membership
tagged membership
native/untagged membership
physical connection
```

Later extensions may include:

```text
MLAG
stacking
bonding
wireless radios/SSIDs
tunnels
subinterfaces
```

Do not assume one logical interface equals one physical connector.

A trunk carrying multiple VLANs is one physical relation with multiple logical memberships.

---

# 11. Interface identity

Do not equate a vendor interface label with permanent semantic identity.

Separate where useful:

```text
logical role:
    nas-uplink

physical realization:
    core / Gi1/0/2
```

This makes hardware replacement easier.

A hardware migration should be able to remap a logical role without forcing all policy/documentation references to change.

Do not over-engineer stable identities in the compiler-only MVP. If deployment state later requires stable identities across renames, introduce an explicit identity mechanism or migration declaration rather than a hidden magical database.

---

# 12. Routing

Support static routing first.

Model:

```text
destination
next hop
outgoing interface/VRF
metric/preference
source table/context
```

Do not treat “reachable” as one boolean.

Distinguish at least conceptually:

```text
RouteExists
PacketPermitted
Layer3Reachable
TransportReachable
AuthenticatedServiceReachable
Healthy
```

Recursive routing must terminate.

Represent route dependency as a graph and require an acyclic/resolvable certificate for steady state.

Dynamic routing should be added later and must distinguish:

```text
configured topology
possible topology
currently selected forwarding topology
```

OSPF/BGP behavior cannot be reduced to the static configured graph.

---

# 13. Policy/firewall semantics

The simple DSL:

```text
iot -> internet allow
```

needs a precise documented semantic expansion.

The core policy model eventually needs to account for:

- address family;
- direction;
- stateful versus stateless behavior;
- connection initiation;
- existing connections;
- transport/service;
- ICMP/control traffic;
- multicast/broadcast;
- local/control-plane traffic versus forwarding;
- policy evaluation relative to NAT;
- implicit/default action.

Avoid making users spell this out for ordinary cases.

Instead define a well-documented default semantic profile and allow explicit refinement.

Example:

```text
iot -> gateway allow dns, ntp
```

might mean:

> permit new connections initiated from the IoT security domain toward DNS/NTP services hosted by the gateway, plus return traffic as defined by the stateful policy model.

That meaning belongs to the language specification.

---

# 14. NAT

NAT must be a first-class transformation and not hidden inside firewall policy.

Represent semantic transformations such as:

```text
internal endpoint
  ──SNAT──► external endpoint

external endpoint
  ──DNAT──► internal service
```

Eventually cover:

- source NAT;
- destination NAT;
- one-to-one NAT;
- port forwarding;
- hairpin NAT;
- upstream NAT assumptions;
- IPv6 translation only where deliberately supported.

Policy semantics must make clear whether a matcher refers to pre- or post-translation identity.

---

# 15. Services and service discovery

Declare services semantically:

```text
service dns {
  udp 53
  tcp 53
}

service home-assistant {
  tcp 8123
}
```

Allow well-known built-ins.

Do not make firewall definitions a pile of numeric ports.

Home-network-specific discovery must eventually cover:

- mDNS;
- SSDP;
- multicast reflection;
- IGMP/MLD behavior;
- selected service discovery between VLANs.

A policy such as:

> trusted clients may discover selected IoT services without permitting arbitrary IoT → trusted connections

must be expressible.

---

# 16. MTU and encapsulation

MTU is a path property.

Model enough information to reason about:

- physical MTU;
- VLAN overhead where relevant;
- PPPoE;
- WireGuard;
- IPsec;
- tunnels;
- nested encapsulation.

Eventually provide:

```text
effectiveMTU path
```

and warn when expected payload requirements are incompatible.

MSS clamping is an implementation mechanism, not the underlying semantic invariant.

---

# 17. VPNs

Treat a VPN as a logical network/security boundary, not merely a vendor-specific interface.

Example:

```text
vpn remote {
  driver wireguard
  subnet 10.0.90.0/24

  peer laptop +2
  peer phone  +3

  route trusted
  route servers
}
```

Policy:

```text
remote -> trusted allow
remote -> servers allow
remote -> iot deny
```

should not care that the underlying implementation happens to be WireGuard.

Provider-specific details remain in the implementation block.

---

# 18. AAA model

AAA must be its own semantic subsystem.

Do not model AAA primarily as:

```text
RADIUS
TACACS
local
```

Model:

```text
Access Surface
  ├─ Authentication contract
  ├─ Authorization contract
  └─ Accounting contract
```

Example surfaces:

```text
management-shell
console
device-api
vpn
wifi-8021x
wired-8021x
```

TACACS+ itself separates authentication, authorization, and accounting and is particularly associated with device administration, fine-grained operation authorization, and auditing.

---

## 18.1 Authentication

Authentication results must distinguish:

```idris
Accepted
Rejected
Unavailable
```

Never treat an explicit rejection as equivalent to service unavailability.

Fallback behavior must be defined per result.

DSL example:

```text
authenticate {
  first infra-aaa

  on unavailable {
    local break-glass
  }

  on reject {
    deny
  }
}
```

---

## 18.2 Authorization

Define vendor-neutral capabilities rather than universal privilege levels.

Example:

```text
role observer {
  permit state.read
  permit logs.read
  permit diagnostics.ping
}

role operator extends observer {
  permit interface.bounce
}

role administrator {
  permit config.read
  permit config.write
  permit device.reboot
}
```

Map identities/groups:

```text
group "net-observers" -> observer
group "net-operators" -> operator
group "net-admins"     -> administrator
```

A backend that only supports “admin” and “viewer” must fail compilation if it cannot faithfully realize `operator`.

Never silently map a role to something more privileged.

---

## 18.3 Accounting

Model events semantically:

```text
authentication-attempt
authentication-result
session-start
session-stop
authorization-decision
command-execution
configuration-change
privilege-change
```

Allow policies such as:

```text
command-execution required
session accounting best-effort
```

Targets advertise which event classes they can produce.

---

## 18.4 Break-glass access

Break-glass access must be explicit and prominent.

Example:

```text
break-glass emergency-admin {
  authenticate local-key

  surfaces {
    console
    management-ssh
  }

  authorize administrator

  constraints {
    remote-aaa-unavailable
  }
}
```

Possible invariants:

- every critical device has a recovery path;
- at least one recovery path is independent of the infrastructure the device is needed to repair;
- break-glass identity is not supplied by the remote AAA dependency it is intended to survive;
- break-glass actions are audited where technically possible.

---

## 18.5 AAA dependency graph

Model dependencies such as:

```text
switch
  → TACACS
  → DNS
  → management network
  → switch
```

This enables detection of survivability cycles.

A dependency cycle is not automatically invalid, but the system should detect when **all** administrative paths depend on a failing component.

---

# 19. Secrets

Secrets must not be ordinary model strings.

Use typed opaque references:

```text
SecretRef TACACSSharedKey
SecretRef WireGuardPrivateKey
SecretRef PasswordHash
```

DSL example:

```text
secret secret://network/tacacs
```

Pure compilation should work with secret references only.

Actual secret materialization belongs in a privileged deployment phase.

This is particularly important when Nix is used for packaging/building: plaintext secret values must not accidentally enter immutable Nix store artifacts.

---

# 20. Target capability model

Do not model device capabilities as simple booleans.

Capabilities may depend on:

- vendor;
- hardware model;
- firmware release;
- installed license;
- interface type;
- feature interactions;
- resource budgets.

Model quantitative limits where applicable:

```text
maximum VLANs
maximum ACL entries
TCAM capacity
maximum routes
VRFs
hardware queues
PoE budget
```

Some capabilities are relational:

```text
A supported
B supported
A + B together unsupported
```

The capability system must be extensible enough to represent this.

---

# 21. Backend realization contract

A target compiler should conceptually have:

```text
CertifiedIntent
+
TargetCapabilities
    ↓
Realization analysis
    ↓
Either Diagnostic TargetAST
```

A backend may only generate configuration if its capability model can faithfully realize required semantics.

If not:

```text
error[aaa.authorization-not-realizable]

Device "garage-switch" cannot realize role "operator".

Required:
    state.read
    interface.clear-counters

Must deny:
    config.write
    device.reboot

Target exposes only:
    viewer
    administrator

Refusing to grant broader privileges.
```

---

# 22. Backend architecture

Maintain the three-stage separation:

```text
semantic IR
    ↓
target compiler
    ↓
target AST
    ↓
renderer
    ↓
text
```

The renderer should be intentionally boring.

It should know syntax, quoting, indentation, and serialization—not networking semantics.

---

## 22.1 OpenWrt

Recommended target AST:

```text
UCI package
  sections
    section type
    optional name
    options
    lists
```

Separate target compilers may generate:

```text
/etc/config/network
/etc/config/dhcp
/etc/config/firewall
```

The generic UCI renderer should not know what a VLAN is.

---

## 22.2 Cisco IOS

Recommended target AST:

```text
command block
  command
  child commands
```

Example:

```text
vlan 20
 name servers
!
interface GigabitEthernet1/0/2
 description NAS
 switchport mode access
 switchport access vlan 20
```

The target compiler decides *what* commands are required.

The renderer decides only *how* they are printed.

---

# 23. Escaping and configuration injection

Never construct vendor commands through unchecked arbitrary interpolation.

Names/descriptions may contain characters significant to target syntax.

Target ASTs must:

- distinguish values from commands;
- escape/quote correctly;
- reject impossible representations;
- prevent a description/string from injecting another command.

This is a security property.

Add adversarial tests for quoting and injection from the beginning.

---

# 24. Vendor escape hatches

Provide a hierarchy:

```text
generic semantic DSL
    ↓
structured vendor-specific extension
    ↓
raw vendor configuration
```

Example:

```text
switch core {
  driver cisco-ios

  cisco {
    spanning-tree mode rapid-pvst
  }
}
```

Raw configuration should be a last resort.

Important: raw configuration can interfere with supposedly proven semantics.

Therefore raw blocks should **taint realization assurance** unless the compiler knows they are confined to a non-interfering namespace.

Example result:

```text
Semantic validity: proved
Cisco realization: conditional

Reason:
  target contains opaque raw configuration
```

Never claim full realization compatibility when opaque commands may override generated behavior.

---

# 25. Ownership semantics

The system must know what configuration it owns.

Absence from the DSL can mean:

```text
delete it
```

or:

```text
leave it alone
```

These are dramatically different.

Support ownership modes such as:

```text
authoritative
managed subset
observed
unmanaged
adopted
```

An authoritative VLAN set might mean:

> VLANs not represented by this source of truth should not exist.

A managed interface might mean:

> Only properties owned by this DSL may be changed.

This matters for deployment, drift, and deletion planning.

---

# 26. Migration model

Migration must be a first-class semantic concept.

Keep:

```text
ValidNetwork
```

strict.

Introduce:

```text
MigrationState outstanding
```

where `outstanding` explicitly represents temporary debt.

Example:

```text
MigrationState [
  DNSAvailable servers
]
```

means everything required has been established except the declared DNS availability invariant.

---

## 26.1 Types of migration debt

Distinguish:

```text
KnownViolation
DeferredProof
RequiresObservation
```

Examples:

```text
KnownViolation:
    DNS unavailable on new prefix

DeferredProof:
    client migration not yet established

RequiresObservation:
    old subnet currently has no active clients
```

Do not turn unknown facts into fake proofs.

---

## 26.2 Relaxable versus non-relaxable invariants

Not every invariant can be waived.

Classify invariants approximately as:

```text
Structural
Safety
Security
Availability
Redundancy
Liveness
```

Some availability/redundancy invariants may be relaxable.

Certain structural or security invariants may be permanently non-relaxable.

Encode relaxability as an explicit relation/type, not a `--force` flag.

---

## 26.3 Migration transitions

Model transitions:

```idris
Transition :
  MigrationState before ->
  MigrationState after ->
  Type
```

A migration plan is a path through state space.

Each transition may have:

```text
preconditions
intended action
expected postconditions
new obligations
resolved obligations
verification checks
recovery plan
```

---

## 26.4 Migration DSL

Example:

```text
migration resubnet-servers {
  from current
  to desired

  phase add-new-prefix {
    servers add-subnet 10.50.20.0/24

    tolerate {
      dns servers unavailable
        reason "DNS listener moves in the next phase"
    }
  }

  phase move-dns {
    dns servers listen-on 10.50.20.1

    resolves {
      dns servers unavailable
    }
  }

  phase move-clients {
    dhcp servers use 10.50.20.0/24

    verify {
      dns servers reachable
      gateway servers reachable
    }
  }

  phase retire-old {
    require {
      old-subnet servers unused
    }

    servers remove-subnet 10.0.20.0/24
  }

  finish {
    require stable
  }
}
```

Use words such as:

```text
tolerate
requires
resolves
verify
```

rather than `ignore`.

Temporary degradation must be visible and accountable.

---

# 27. Temporal semantics

Some migration properties depend on time:

```text
wait for DNS TTL
wait for DHCP lease expiration
credential overlap
certificate lifetime
key rotation overlap
route convergence
```

The migration subsystem eventually needs concepts such as:

```text
before
after
for-at-least
until
within
deadline
```

Do not assume a configuration change instantaneously changes every distributed subsystem.

DNS and DHCP in particular require convergence-aware modeling.

---

# 28. Rollback and recovery

Do not assume rollback is the inverse of a configuration operation.

Changing DHCP or DNS can create external state:

- leases;
- caches;
- live TCP sessions;
- route convergence;
- accounting records.

Therefore model:

```text
RecoveryPlan
```

rather than mathematical inverse transition.

Detect points of no return:

```text
warning[migration.point-of-no-return]

Phase "remove-old-prefix" has no verified recovery path.

Consider a verification gate before proceeding.
```

---

# 29. Deployment semantics

Deployment is a distributed transaction with heterogeneous target semantics.

Possible device capabilities include:

```text
atomic candidate
confirmed commit
transactional replace
incremental command application
file replace + reload
reboot required
```

NETCONF, for example, defines candidate configuration and confirmed-commit mechanisms on supporting devices, but these cannot be assumed universally.

Migration planning must account for how each target actually applies changes.

Intermediate device states during command application may differ from the abstract pre/post states of a migration phase.

---

# 30. Desired, intended, applied, and observed state

Maintain separate representations.

Recommended terminology:

```text
DesiredIntent
    human/DSL semantics

IntendedConfig
    target-specific configuration generated by compiler

AppliedConfig
    configuration actually in use according to target

OperationalSnapshot
    current runtime state and observations
```

Include timestamps and evidence sources in observed data.

Do not infer “down” from “not reported.”

Represent unknown explicitly.

---

# 31. Drift

Eventually provide:

```console
netc observe
netc diff
```

Classify differences:

```text
desired mismatch
target default
device-generated
learned state
unmanaged configuration
remnant configuration
unsupported observation
unknown
```

RFC 8342 explicitly recognizes intended, applied, learned, system/default, remnant, and operational state distinctions; use that conceptual vocabulary where useful rather than inventing a simplistic desired/actual binary.

---

# 32. Dynamic protocols

Do not attempt to prove current BGP/OSPF/STP forwarding behavior from static configuration alone.

Model separately:

```text
configured constraints
possible protocol outcomes
currently observed selection
```

For an STP topology, physical cycles may be correct while the selected forwarding tree is acyclic.

For dynamic routing, the compiler may prove:

```text
configuration is internally coherent
```

but observed route selection belongs to operational state.

---

# 33. Schema/language versioning

Every source file must declare a language version:

```text
network-language 1.0
```

Old source must be interpreted under old semantics.

Never silently reinterpret an old file according to new rules.

---

## 33.1 Version-indexed models

Conceptually:

```idris
Network : SchemaVersion -> Type
```

so:

```text
Network V1
Network V2
```

are distinct types.

Parser:

```idris
parse :
  (v : SchemaVersion) ->
  String ->
  Either Diagnostic (Network v)
```

This makes accidental cross-version interpretation much harder.

---

## 33.2 Elm-style automated version classification

Generate a canonical public-language/schema manifest from the compiler implementation.

On release:

```text
old manifest
    ↓
structural diff
    ↓
minimum required version bump
```

Examples:

```text
add optional field with old-preserving default:
    MINOR

add required field:
    MAJOR

remove construct:
    MAJOR

narrow accepted VLAN range:
    MAJOR
```

A release tool should reject a version bump smaller than the detected minimum.

Prefer tooling that determines the next version rather than asking a developer to choose arbitrarily.

---

## 33.3 Semantic changes

Schema comparison cannot detect:

```text
same syntax
same structural type
different meaning
```

Therefore semantic changes require explicit declarations.

Automatic structural classification establishes a **minimum** bump.

Manual semantic declarations may raise that requirement but never lower it.

---

## 33.4 Typed migrations

Model upgrades:

```idris
upgradeV1toV2 :
  Network V1 ->
  Either MigrationError (Network V2)
```

If every V1 source has an unambiguous preserving representation:

```idris
upgradeV1toV2 :
  Network V1 ->
  Network V2
```

Where practical, define semantic preservation in the compiler's own abstract meaning model.

Do not pretend this proves vendor-level behavioral equivalence.

---

# 34. DSL compatibility versus backend compatibility

Version independently:

```text
DSL language
semantic IR/internal API
backend API
persistent/export format
```

A Cisco backend API change should not require every user configuration to bump DSL language version.

---

# 35. Serialization and intermediate formats

Do not make JSON the internal semantic IR.

Inside Idris, use Idris values.

Do not serialize proof-carrying IR unless there is a compelling reason.

Prefer re-elaborating source to reconstruct proofs.

If external tools require a semantic export, define a **separate versioned interchange format**.

Possible formats:

- JSON for maximum interoperability;
- CBOR for compact machine exchange;
- EDN/S-expression-like formats if symbol/set/tag richness is valuable.

The interchange representation is not the authoritative semantic type system.

It must carry its own schema version.

---

# 36. Diagnostics

Diagnostics are a first-class API.

Suggested structure:

```idris
record Diagnostic where
  code        : DiagnosticCode
  severity    : Severity
  primarySpan : SourceSpan
  title       : String
  detail      : String
  related     : List RelatedDiagnostic
  fixes       : List TextEdit
```

Stable error codes matter.

Examples:

```text
address.static-dhcp-overlap
topology.missing-vlan
routing.dependency-cycle
aaa.no-independent-recovery-path
aaa.authorization-not-realizable
migration.point-of-no-return
backend.capability-mismatch
```

---

## 36.1 Errors should report witnesses/counterexamples

Bad:

```text
Network is not acyclic.
```

Good:

```text
Recursive route dependency detected:

    route corp
      → route vpn
      → route default
      → route corp
```

Bad:

```text
Prefix conflict.
```

Good:

```text
10.0.20.0/24 overlaps 10.0.20.128/25

Declared at:
    home.net:22
    home.net:48
```

Decision algorithms should be designed to return useful failure witnesses.

---

# 37. Editor/browser architecture

The DSL should eventually support a rich browser/editor.

Recommended architecture:

```text
Browser
  │
  ├─ CodeMirror/Monaco
  │
  ├─ incremental syntax parser
  │
  └─ semantic worker
         │
         ▼
  Idris core compiled to JavaScript
```

Idris 2 has JavaScript and Node code generators, making reuse of semantic code in a browser technically viable.

The initial browser implementation should revalidate the whole small home-network model on each debounced edit rather than prematurely building incremental dependent elaboration.

Optimize only after measurement.

---

## 37.1 Editor features

Eventually support:

```text
syntax diagnostics
semantic diagnostics
completion
go-to-definition
find references
hover
resolved-address preview
capability warnings
quick fixes
migration obligation view
topology graph view
generated target config preview
desired/applied/observed comparisons
```

Do not expose enormous raw dependent types to ordinary users.

Provide a stable semantic-information API tailored to networking.

---

## 37.2 Editing Idris versus editing the DSL

Idris itself exposes a machine-readable IDE protocol suitable for editor integrations.

However, users are editing the external network DSL, not Idris source.

Therefore build a DSL language service on top of the semantic engine rather than trying to forward the Idris IDE protocol directly to users.

---

# 38. Documentation generation

Generate at least:

```text
VLAN/address table
host inventory
service inventory
device/port table
routing table
policy matrix
AAA policy summary
migration state/debt
topology graph
dependency graph
```

Generated documentation must indicate epistemic state:

```text
Desired
Intended
Applied
Observed
Unknown
Drifted
```

Documentation generated solely from desired state must not masquerade as observed truth.

---

# 39. Testing strategy

Testing needs several layers.

## Parser tests

- valid syntax;
- invalid syntax;
- error recovery;
- source spans;
- comments;
- Unicode if supported;
- large/nested constructs;
- adversarial input.

## Parser conformance

If Tree-sitter or another editor parser exists, maintain a corpus shared with the authoritative parser.

## Domain tests

Test:

- IPv4;
- IPv6;
- CIDR;
- VLAN;
- port ranges;
- address ranges;
- prefix arithmetic;
- address offset calculations.

## Property tests

Examples:

```text
render(parse(x)) preserves meaning

no valid IPv4 constructor yields octet >255

relative address allocation stays within prefix or fails

accepted topological certificate contains no backward edge
```

## Negative semantic tests

Every important diagnostic should have a fixture.

Example:

```text
fixtures/errors/static-dhcp-overlap.net
fixtures/errors/route-cycle.net
fixtures/errors/aaa-no-recovery.net
```

Check diagnostic code, relevant spans, and key structured fields rather than only entire prose strings.

## Backend golden tests

Input semantic model → exact UCI/IOS output.

Keep golden output deterministic.

## Adversarial renderer tests

Especially descriptions/names containing target syntax metacharacters.

## Migration tests

Test:

- obligation introduced;
- obligation discharged;
- non-relaxable invariant rejected;
- final state cannot be accepted with debt;
- failed verification;
- unavailable recovery.

## Versioning tests

Keep fixture configurations from every supported language release.

Every new compiler release should verify that they continue to parse under their original semantics.

## Backend integration tests

Where practical:

- OpenWrt VM/QEMU;
- test containers/services;
- vendor simulator/emulator;
- real lab device tests.

Treat these as bridging tests across the vendor-semantic trust boundary.

## Cross-backend tests

Where two targets claim to implement the same abstract capability, test that they produce equivalent abstract behavior according to our own target semantic model.

---

# 40. Fuzzing and robustness

The DSL parser is an attack surface if configurations may ever come from untrusted sources.

Fuzz:

- lexer/parser;
- CIDR parsing;
- malformed Unicode;
- deeply nested syntax;
- huge lists;
- target rendering.

Prevent pathological input from producing unreasonable CPU or memory consumption.

Graph validation algorithms should have clear complexity characteristics.

For expected home-network sizes, correctness is more important than micro-optimization.

---

# 41. Idris elaboration/performance concerns

Do not encode the entire network as enormous compile-time type-level literal structures merely because Idris permits it.

That can make elaboration slow and error output unusable.

Prefer:

```text
runtime/value graph
+
proof/certificate values
```

rather than making every vertex and edge a deeply nested type parameter.

The important property is:

```text
(g : Network ** ValidNetwork g)
```

not “every byte of `g` appears as a gigantic type-level expression everywhere.”

Proofs that are irrelevant at runtime should be erasable.

---

# 42. Compiler purity boundary

Keep the semantic compiler mostly pure.

Recommended division:

```text
Pure:
  parse result transformations
  name resolution
  validation
  proofs
  normalization
  target compilation
  rendering
  documentation

Effectful:
  read files
  resolve external secrets
  query devices
  deploy configuration
  collect telemetry
```

This improves testability and makes semantic reasoning considerably easier.

---

# 43. Repository structure

A possible initial layout:

```text
network-dsl/
├── flake.nix
├── network-dsl.ipkg
├── README.md
├── DESIGN.md
│
├── src/
│   └── NetDSL/
│       ├── Syntax/
│       │   ├── Token.idr
│       │   ├── AST.idr
│       │   └── Span.idr
│       │
│       ├── Parser/
│       │   └── Parser.idr
│       │
│       ├── Diagnostic/
│       │   ├── Types.idr
│       │   └── Render.idr
│       │
│       ├── Domain/
│       │   ├── Address.idr
│       │   ├── Prefix.idr
│       │   ├── VLAN.idr
│       │   ├── Service.idr
│       │   ├── Device.idr
│       │   ├── Interface.idr
│       │   ├── Routing.idr
│       │   ├── Policy.idr
│       │   └── AAA.idr
│       │
│       ├── Graph/
│       │   ├── Core.idr
│       │   ├── Path.idr
│       │   ├── DAG.idr
│       │   ├── Connectivity.idr
│       │   └── Views.idr
│       │
│       ├── Resolve/
│       │   └── Names.idr
│       │
│       ├── Elaborate/
│       │   ├── Network.idr
│       │   ├── Addressing.idr
│       │   ├── Routing.idr
│       │   ├── Policy.idr
│       │   └── AAA.idr
│       │
│       ├── Invariant/
│       │   ├── Core.idr
│       │   ├── Stable.idr
│       │   └── Relaxable.idr
│       │
│       ├── Migration/
│       │   ├── State.idr
│       │   ├── Transition.idr
│       │   ├── Plan.idr
│       │   └── Verify.idr
│       │
│       ├── Backend/
│       │   ├── Capability.idr
│       │   ├── Common.idr
│       │   ├── OpenWrt/
│       │   │   ├── Compile.idr
│       │   │   ├── UCI.idr
│       │   │   └── Render.idr
│       │   └── CiscoIOS/
│       │       ├── Compile.idr
│       │       ├── AST.idr
│       │       └── Render.idr
│       │
│       ├── Docs/
│       │   ├── Markdown.idr
│       │   └── Mermaid.idr
│       │
│       ├── Version/
│       │   ├── Schema.idr
│       │   ├── Manifest.idr
│       │   └── Migrate.idr
│       │
│       └── Export/
│           └── Semantic.idr
│
├── app/
│   └── Main.idr
│
├── grammar/
│   └── tree-sitter-netdsl/
│
├── editor/
│   └── web/
│
├── examples/
│   ├── home.net
│   ├── aaa.net
│   └── migration.net
│
└── tests/
    ├── parser/
    ├── semantic/
    ├── diagnostics/
    ├── backend/
    ├── migration/
    └── compatibility/
```

Idris packages support ordinary modular package organization and versioned dependencies via `.ipkg`.

---

# 44. Nix role

Nix should initially be the reproducible build/development environment rather than the language in which network intent is expressed.

Use the flake for:

```text
Idris compiler
dependencies
tree-sitter tooling
browser tooling
test dependencies
formatter
CLI package
generated artifacts
```

Potential interface:

```console
nix develop
nix build .#netc
nix flake check
```

The network DSL itself remains independent of Nix semantics.

This also keeps future non-Nix users possible.

---

# 45. CLI design

Target CLI commands:

```console
netc check home.net
netc fmt home.net
netc explain <diagnostic-code>

netc compile home.net --target gateway
netc compile home.net --target core
netc compile home.net --all

netc docs home.net
netc graph home.net

netc plan migration.net
netc migrate home.net --to-language 2.0

netc schema
netc schema diff 1.3 1.4
netc release-check

netc observe ...
netc diff ...
```

`observe` and actual deployment may be later milestones.

---

# 46. CLI output guarantees

CLI commands intended for scripting should support structured output.

Example:

```console
netc check home.net --format json
```

Human output and machine output should be separate APIs.

Stable diagnostic codes matter more than exact prose.

---

# 47. Backend correctness boundary

The internal statement:

```text
CiscoConfig implements Policy
```

cannot generally be formally proven against real Cisco firmware because we do not possess a complete formal semantics for Cisco IOS.

Therefore backend confidence comes from:

```text
our semantic model
+ capability declarations
+ golden tests
+ integration tests
+ observed post-deployment behavior
```

Document this boundary explicitly.

A backend's claim is:

> Given the documented assumptions about target behavior and capabilities, this generated configuration realizes the semantic policy.

Not:

> This theorem proves the hardware will behave correctly.

---

# 48. Backend capability metadata is itself an assumption

Even if Idris proves:

```text
RequiredCapabilities ⊆ DeclaredCapabilities
```

it only proves something about the declared capability value.

It does not establish that the physical device actually possesses those capabilities.

Capability discovery should eventually be backed by:

```text
static vendor database
device model
software version
live capability query
```

and record the evidence source.

---

# 49. Hard edge cases that must remain visible in the design

Do not lose track of the following as implementation proceeds:

### NAT identity

Addresses before and after translation are different semantic identities.

### Stateful firewalls

Policy changes do not necessarily destroy existing flows.

### IPv6

Do not impose IPv4 allocation assumptions.

### Multicast/service discovery

Not reducible to normal unicast reachability.

### Dynamic routing/STP

Configured state is not selected operational state.

### Hardware limits

Feature existence is not sufficient; resource capacity matters.

### Vendor feature interaction

Two individually supported features may not work together.

### Physical reality

A configured-up interface may still be operationally down.

### Time

DNS, DHCP, credentials, certificates, and migration convergence are temporal.

### External systems

AAA, DNS, NTP, PKI, DHCP, and secret systems have their own failure modes.

### Partial deployment

Multi-device changes are not atomic by default.

### Rollback

Reversing configuration does not necessarily reverse external state.

### Raw vendor configuration

Opaque extensions can invalidate realization guarantees.

### Configuration ownership

Deletion semantics are impossible without knowing what the compiler owns.

### Proven versus observed

Never collapse them into one state called `valid`.

---

# 50. Recommended MVP scope

Do not implement everything above immediately.

The first useful version should deliberately be smaller.

## MVP language

Support:

```text
network
domain
vlan
IPv4 subnet
gateway
DHCP range
host/static assignment
device
port
access
trunk
physical connect
service
simple stateful allow/deny policy
```

Address IPv6 architecture correctly in the type design, but IPv6 surface features may follow later.

## MVP semantic invariants

Implement:

```text
valid VLAN IDs
valid IPv4/CIDR
canonical prefixes
address membership
duplicate addresses
DHCP/static overlap
prefix overlap
resolved references
port/VLAN consistency
physical link consistency
basic routing
basic policy references
graph cycle infrastructure
```

## MVP outputs

Generate:

```text
OpenWrt UCI network configuration
OpenWrt DHCP configuration
basic OpenWrt firewall configuration
Cisco IOS VLAN/interface configuration
Markdown address/VLAN/device docs
Mermaid topology
```

## MVP tooling

Implement:

```text
netc check
netc fmt
netc compile
netc docs
netc graph
```

---

# 51. Second milestone

Add:

```text
IPv6
VPN/WireGuard
NAT
multicast/service discovery
MTU reasoning
richer routing
target capabilities
configuration ownership
```

---

# 52. Third milestone

Add full AAA semantics:

```text
identity providers
auth result distinctions
fallback
roles/capabilities
accounting obligations
break-glass
AAA dependency graph
secret references
target realization analysis
```

AAA should be implemented only after the generic capability/diagnostic framework is mature, because AAA will stress it heavily.

---

# 53. Fourth milestone

Add migration semantics:

```text
MigrationState
obligations
known violations
deferred proofs
observational gates
relaxability
phases
recovery
temporal constraints
```

The migration system depends on a mature inventory of steady-state invariants, so it should not be the first subsystem implemented.

---

# 54. Fifth milestone

Add operational tooling:

```text
device observation adapters
applied/operational models
drift
deployment planner
verification
partial-failure handling
secret materialization
```

Keep deployment separable enough that Pulumi or another external engine could conceivably become one consumer of the semantic plan.

---

# 55. Sixth milestone

Add browser/editor tooling:

```text
Tree-sitter grammar
CodeMirror/Monaco
Idris-JS semantic worker
completions
go-to-definition
hover
quick fixes
topology display
migration display
generated-config preview
```

Idris's JavaScript backend means the same core implementation can potentially execute in the browser rather than rewriting validation logic in TypeScript.

---

# 56. Acceptance criteria for the first serious release

The implementation should not be considered architecturally successful merely because it generates working router configuration.

A successful first serious release should demonstrate all of these:

1. The sample home network can be represented clearly in the external DSL.

2. Invalid CIDR, VLAN, address, and topology relationships receive source-localized, domain-specific diagnostics.

3. The normalized model contains no unresolved string references.

4. At least one global graph property is certificate-checked rather than merely represented as a Boolean.

5. OpenWrt and Cisco consume the **same semantic model**.

6. OpenWrt and Cisco backend renderers contain no generic network decision logic.

7. A backend capability mismatch produces an error rather than silently degrading semantics.

8. Source provenance survives through target compilation.

9. Generated output is deterministic.

10. Generated docs originate from the same semantic model as configuration.

11. Language version appears explicitly in every source document.

12. Compatibility tests exist for every released language version.

13. There are no unchecked secret values embedded in pure generated artifacts.

14. The trusted semantic core contains no casual proof escape hatches.

15. The documentation clearly distinguishes proven model properties from assumptions and observations.

---

# 57. Explicit non-goals for the first version

Do **not** initially attempt to:

- formally model complete Cisco IOS semantics;
- formally model Linux/OpenWrt kernel packet processing;
- replace NetBox as a full inventory/database application;
- implement every routing protocol;
- implement arbitrary custom user code inside the DSL;
- build a distributed transactional deployment controller;
- prove physical connectivity;
- prove that a remote service is really healthy;
- build a perfect incremental dependent-type language server;
- support every vendor;
- solve secret storage;
- provide a general policy theorem prover.

These can be layered onto a sound semantic core later.

---

# 58. Design decisions that should not be revisited casually

The following choices are foundational:

### External ergonomic DSL

The user does not edit Idris or Nix as the primary source language.

### Idris semantic core

Idris owns the authoritative semantic interpretation and validation.

### Rich internal IR

Do not reduce the semantic core to JSON dictionaries.

### Structured model plus graph views

Do not represent the entire network as one untyped generic graph.

### Vendor-neutral semantics

Vendor implementation is downstream.

### Compiler + renderer separation

Do not intermingle semantic compilation with text emission.

### Explicit capability failure

Never silently weaken requested semantics.

### Strong steady state

Do not make migrations possible by weakening the normal network type.

### Explicit migration debt

Temporary brokenness is represented, named, and required to be discharged.

### AAA semantics above protocols

Authentication, authorization, accounting, fallback, and recovery are first-class semantic concepts.

### Proven ≠ observed

This distinction must survive into CLI output and documentation.

### Explicit language version

New compiler versions never silently reinterpret old source.

### Diagnostics as structured data

Do not treat diagnostic prose as an afterthought.

---

# 59. First implementation spike

Before building the entire grammar, implement one thin vertical slice.

Input:

```text
network-language 1.0

network home {
  vlan servers 20 {
    subnet 10.0.20.0/24
    gateway +1

    host nas +10
  }

  switch core {
    driver cisco-ios

    port Gi1/0/2 {
      access servers
    }
  }

  router gateway {
    driver openwrt

    port lan2 {
      access servers
    }
  }
}
```

Implement end-to-end:

```text
text
→ parser
→ spans
→ name resolution
→ VlanId
→ IPv4Prefix
→ relative address
→ semantic Network
→ validation
→ Cisco AST
→ IOS text
→ OpenWrt AST
→ UCI text
→ Markdown docs
```

Intentionally test:

```text
VLAN 4095
bad CIDR
+256 outside a /24
unknown VLAN reference
duplicate IP
description containing renderer metacharacters
```

If that slice is pleasant in Idris and produces excellent diagnostics, proceed.

If it is cumbersome, adjust representation boundaries before adding AAA, migrations, or advanced graph proofs.

---

# 60. Second proof-of-concept spike

Implement one genuinely graph-dependent invariant.

Recommended choice:

```text
service/route dependency acyclicity
```

Use a topological-sort algorithm that returns:

```text
Either
  CycleWitness
  TopologicalCertificate
```

Wrap the successful network in a proof-carrying representation.

This verifies that the intended architecture works beyond simple smart constructors.

---

# 61. Third proof-of-concept spike

Implement one capability failure across two backends.

Example semantic concept:

```text
trunk trusted servers iot
```

Compile to:

- OpenWrt UCI;
- Cisco IOS.

Then introduce an artificial target capability:

```text
maximum tagged VLANs = 2
```

and verify the third target fails with a structured diagnostic.

This validates the “semantic intent + realization contract” architecture.

---

# 62. Fourth proof-of-concept spike

Implement a minimal migration:

```text
servers:
    10.0.20.0/24
        ↓
    10.50.20.0/24
```

Allow an intermediate state with:

```text
DNSAvailable servers
```

explicitly outstanding.

Verify:

```text
StableNetwork
→ MigrationState [DNSAvailable servers]
→ StableNetwork
```

and ensure no API accepting `StableNetwork` can accidentally consume the middle state.

This validates the typestate concept before migration syntax becomes extensive.

---

# 63. Fifth proof-of-concept spike

Implement one AAA policy:

```text
management-shell {
  authenticate infra-aaa
  fallback-on-unavailable local-break-glass

  authorize net-admins -> administrator

  account command-execution required
}
```

Create:

- a fully capable target;
- a target lacking command accounting.

The second must fail realization.

This validates that AAA is modeled semantically rather than as target-specific syntax.

---

# 64. Documentation requirement

Maintain a written language specification alongside implementation.

For each construct define:

```text
syntax
semantic meaning
defaults
invariants
error behavior
version introduced
target realization requirements
```

Especially document shorthand.

For example:

```text
iot -> internet allow
```

must have a precise semantic definition.

Do not allow “obvious meaning” to substitute for specification.

---

# 65. Final mental model

The DSL is not fundamentally a configuration-template language.

It is a **network intent compiler**.

Its job is:

```text
human intent
     ↓
precise semantic model
     ↓
proof/certification of model invariants
     ↓
capability-checked target realization
     ↓
configuration / documentation / migration plans
     ↓
deployment
     ↓
observation
     ↓
comparison with intent
```

The most valuable design property is not that it can produce UCI or IOS.

It is that the same semantic statement:

```text
iot -> gateway allow dns
```

has one defined meaning, can participate in global reasoning, can be checked against migration and AAA dependencies, can be tested for target realizability, can generate documentation, and can later be compared with observed reality.

The implementation should preserve that property even when doing so is less convenient than embedding a target-specific command.

That is the core architectural constraint.