module NetDSL.Version

import NetDSL.Common
import NetDSL.Router.Schema

%default total

public export
languageVersion : String
languageVersion = "2.0"

public export
manifest : String
manifest = "{\"manifestVersion\":1,\"languageVersion\":\"2.0\",\"compilerVersion\":\"0.2.0\",\"backendApiVersion\":\"2.0\",\"exportVersion\":\"2.0\",\"constructs\":" ++
  jsonArray (map (\(n,required) => "{\"name\":" ++ jsonString n ++ ",\"required\":" ++ required ++ ",\"introduced\":\"2.0\"}")
    [("network-language","true"),("network","true"),("domain","false"),("vlan","false"),("subnet","true"),
     ("gateway","false"),("dhcp","false"),("host","false"),("router","false"),("switch","false"),("device","false"),
     ("driver","true"),("port","false"),("access","false"),("trunk","false"),("connect","false"),("description","false"),
     ("service","false"),("tcp","false"),("udp","false"),("depends","false"),("policy","false"),("route","false"),
     ("destination","true"),("via","true"),("metric","false"),("max-tagged-vlans","false"),("routing","false"),("physical","false"),("bridge","false"),("bond","false"),
     ("interface","false"),("globals","false"),("dns","false"),("dhcp-server","false"),("odhcp","false"),
     ("firewall","false"),("zone","false"),("forward","false"),("firewall-rule","false"),("radio","false"),("access-point","false"),("credential","false")]) ++
  ",\"routingSchema\":" ++ routingSchema ++
  ",\"constraints\":{\"vlanMin\":1,\"vlanMax\":4094,\"ipv4PrefixMin\":0,\"ipv4PrefixMax\":32,\"ipv6PrefixMin\":0,\"ipv6PrefixMax\":128,\"sourceMaxCharacters\":1048576,\"sourceMaxTokens\":32768,\"sourceMaxNesting\":64},\"semantics\":{\"policy\":\"ipv4-stateful-ordered-default-deny-v2\",\"dhcp\":\"gateway-dns-control-required-v2\",\"address\":\"network-relative-offset-v2\",\"link\":\"exact-tagged-membership-v2\",\"explicitRouting\":\"typed-dualstack-zones-and-wireless-v2\",\"dhcpForms\":\"inclusive-range-or-offset-count-v2\",\"secrets\":\"opaque-references-template-manifest-v1\"}}"

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
  "syntax" :: _ => Just "Use network-language 2.0 followed by one network block. Braces delimit declarations and newlines delimit scalar statements; comments begin with # or //. Unknown constructs, malformed quoted strings and input resource limits are explicit errors. See docs/language.md for the complete grammar."
  "version" :: _ => Just "Every source must explicitly declare network-language 2.0. Unsupported versions are rejected rather than reinterpreted. Language 1.0 is no longer accepted. Migrate source explicitly to 2.0; there is no compatibility adapter or automatic migration command."
  "policy" :: _ => Just "Policy requires one routing owner covering both zones. Use comma-separated service names. Gateway means local traffic; other zones and internet mean forwarding. DHCP requires gateway DHCP/DNS control traffic, so contradictory denials fail."
  "network" :: _ => Just "Gateway, DHCP, route and policy declarations require one routing owner carrying their VLANs. Add or correct the router's memberships; target capability checking separately establishes whether its backend can realize them."
  "backend" :: _ => Just "The selected target profile cannot faithfully serialize or realize this intent. Correct unsupported interface names, unsafe text or resource limits, or choose a capable target. The compiler will not broaden policy or discard requirements to make output succeed."
  "router" :: _ => Just "Explicit routing declares physical links, bridges, bonds, logical interfaces and typed settings. Keep link membership acyclic, bind logical interfaces to masters, and declare the correct protocol-specific settings."
  "bond" :: _ => Just "The native OpenWrt bond profile requires 802.3ad and at least two unique physical members. Minimum links must fit membership; MAC and timing settings are checked. Peer LACP state remains unknown."
  "wireless" :: _ => Just "Radios require matching hardware paths, bands, channels and widths. APs bind to a radio and bridged logical interface. Secured APs require opaque credential references; country and SSID constraints are checked."
  "dhcp" :: _ => Just "DHCP serving needs a static subnet, valid finite pool, adequate capacity and permitted DHCP/DNS input. IPv6 servers need downstream IPv6 configuration and odhcp settings. Ignored interfaces cannot enable servers."
  "aaa" :: _ => Just "The AAA foundation API requires exact role permissions, required accounting events and independent recovery. Explicit authentication rejection never triggers unavailable-service fallback. Full AAA source syntax and target commands are later milestones."
  "secret" :: _ => Just "Wi-Fi credentials use secret:// identifiers. Compilation produces a wireless template and a binding manifest; it never retrieves or embeds secret material. See docs/secrets.md. Literal credentials are rejected without echoing their values."
  "io" :: _ => Just "The source file could not be read. Check its path and read permissions. The compiler does not contact devices."
  "graph" :: _ => Just "Graph vertices must be unique and all edge endpoints must exist. A successful topological certificate covers every vertex and orders every directed edge forward."
  _ => Nothing
