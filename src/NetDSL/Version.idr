module NetDSL.Version

import NetDSL.Common

%default total

public export
languageVersion : String
languageVersion = "1.0"

public export
manifest : String
manifest = "{\"manifestVersion\":1,\"languageVersion\":\"1.0\",\"compilerVersion\":\"0.1.0\",\"backendApiVersion\":\"1.0\",\"exportVersion\":\"1.0\",\"constructs\":" ++
  jsonArray (map (\(n,required) => "{\"name\":" ++ jsonString n ++ ",\"required\":" ++ required ++ ",\"introduced\":\"1.0\"}")
    [("network-language","true"),("network","true"),("domain","false"),("vlan","false"),("subnet","true"),
     ("gateway","false"),("dhcp","false"),("host","false"),("router","false"),("switch","false"),("device","false"),
     ("driver","true"),("port","false"),("access","false"),("trunk","false"),("connect","false"),("description","false"),
     ("service","false"),("tcp","false"),("udp","false"),("depends","false"),("policy","false"),("route","false"),
     ("destination","true"),("via","true"),("metric","false"),("max-tagged-vlans","false")]) ++
  ",\"constraints\":{\"vlanMin\":1,\"vlanMax\":4094,\"ipv4PrefixMin\":0,\"ipv4PrefixMax\":32,\"sourceMaxCharacters\":1048576,\"sourceMaxTokens\":32768,\"sourceMaxNesting\":64},\"semantics\":{\"policy\":\"ipv4-stateful-ordered-default-deny-v1\",\"dhcp\":\"gateway-dns-control-required-v1\",\"address\":\"network-relative-offset-v1\",\"link\":\"exact-tagged-membership-v1\"}}"

public export
explain : String -> Maybe String
explain code = case splitOn '.' code of
  "address" :: _ => Just "IPv4 assignments must be usable addresses in a canonical prefix. Static addresses (including gateways) must be unique and outside DHCP pools; VLAN prefixes must not overlap. Correct the address or pool at the primary span and compare related declarations. +N is an offset from the network address."
  "vlan" :: _ => Just "Generic VLAN IDs must be integers in 1..4094. A target may reserve additional IDs; those produce backend capability diagnostics. Choose a usable ID without duplicating another VLAN."
  "topology" :: _ => Just "A physical port has one owner, one mode and at most one peer. Connected ports must agree on exact tagged/untagged memberships; implicit host eth0 requires its VLAN on an access port. Fix the referenced endpoint or conflicting memberships. Physical cycles themselves are allowed."
  "routing" :: _ => Just "Static routes require a router carrying the selected VLAN, an on-link next hop distinct from its own address, and terminating dependencies. Follow the reported dependency cycle and remove or correct a dependency; physical reachability is not inferred."
  "service" :: _ => Just "Services require TCP/UDP ports in 1..65535 and terminating service dependencies. Correct an invalid port or the named dependency cycle. Service declarations describe transport contracts, not observed health."
  "reference" :: _ => Just "References resolve in their expected namespace before certification. Declare the missing VLAN, device, service or route, or correct its spelling. Host and device names cannot collide where a physical endpoint would be ambiguous."
  "name" :: _ => Just "Names must use supported ASCII identifier characters and be unique in their namespace. Fields such as subnet and driver may occur only once. Inspect related spans for the earlier declaration."
  "syntax" :: _ => Just "Use network-language 1.0 followed by one network block. Braces delimit declarations and newlines delimit scalar statements; comments begin with # or //. Unknown constructs, malformed quoted strings and input resource limits are explicit errors. See docs/language.md for the complete grammar."
  "version" :: _ => Just "Every source must explicitly declare network-language 1.0. Unsupported versions are rejected rather than reinterpreted. Keep the original version and use a compiler supporting it; this release has no 2.0 language upgrade."
  "policy" :: _ => Just "Policy requires one routing owner covering both zones. Use comma-separated service names. Gateway means local traffic; other zones and internet mean forwarding. DHCP requires gateway DHCP/DNS control traffic, so contradictory denials fail."
  "network" :: _ => Just "Gateway, DHCP, route and policy declarations require one routing owner carrying their VLANs. Add or correct the router's memberships; target capability checking separately establishes whether its backend can realize them."
  "backend" :: _ => Just "The selected target profile cannot faithfully serialize or realize this intent. Correct unsupported interface names, unsafe text or resource limits, or choose a capable target. The compiler will not broaden policy or discard requirements to make output succeed."
  "aaa" :: _ => Just "The AAA foundation API requires exact role permissions, required accounting events and independent recovery. Explicit authentication rejection never triggers unavailable-service fallback. Full AAA source syntax and target commands are later milestones."
  "secret" :: _ => Just "Pure compilation accepts opaque secret:// references only; it never reads or embeds secret material. Full secret deployment is outside this compiler."
  "io" :: _ => Just "The source file could not be read. Check its path and read permissions. The compiler does not contact devices."
  "graph" :: _ => Just "Graph vertices must be unique and all edge endpoints must exist. A successful topological certificate covers every vertex and orders every directed edge forward."
  _ => Nothing
