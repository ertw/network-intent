module NetDSL.Docs

import NetDSL.Common
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Validate
import NetDSL.Graph.DAG
import Data.List
import Data.String
import Data.Maybe

%default total

private
cell : String -> String
cell s = concatMap (\c => case c of '|' => "\\|"; '<' => "&lt;"; '>' => "&gt;"; '&' => "&amp;"; _ => singleton c) (unpack s)

private
table : List String -> List (List String) -> String
table headings rows = "| " ++ join " | " headings ++ " |\n| " ++ join " | " (map (const "---") headings) ++ " |\n" ++
  concatMap (\row => "| " ++ join " | " (map cell row) ++ " |\n") rows ++ "\n"

public export
vlanLabel : Network v -> Ref VLAN -> String
vlanLabel n r = maybe "invalid" name (getVlan n r)

private
deviceLabel : Network v -> Ref DeviceKind -> String
deviceLabel n r = maybe "invalid" name (getDevice n r)

private
endpointLabel : Network v -> PolicyEndpoint -> String
endpointLabel n (Zone r) = vlanLabel n r
endpointLabel n Internet = "internet"
endpointLabel n (Gateway r) = "gateway (" ++ deviceLabel n r ++ ")"

private
peerLabel : Network v -> LinkEnd -> String
peerLabel n (DevicePort p) = deviceLabel n p.owner ++ "." ++ maybe "invalid" name (getPort n p)
peerLabel n (HostPort h) = maybe "invalid" (\pair => (snd pair).name ++ ".eth0") (lookupAt h.index (allHosts n))

private
portModeLabel : Network v -> PortMode -> String
portModeLabel n (Access r) = "access " ++ vlanLabel n r
portModeLabel n (Trunk rs) = "trunk " ++ join ", " (map (vlanLabel n) rs)

private
endpointId : LinkEnd -> String
endpointId (DevicePort p) = "d" ++ show p.owner.index ++ "p" ++ show p.index
endpointId (HostPort h) = "h" ++ show h.index

public export
graph : StableNetwork -> String
graph stable = let n = stable.model in
  "flowchart LR\n  %% Desired physical topology; not observed connectivity\n" ++
  concatMap (\(r,p) => "  " ++ endpointId (DevicePort r) ++ "[\"" ++ deviceLabel n r.owner ++ "." ++ p.name ++ "\"]\n") (allPorts n) ++
  concatMap (\(i,(_,h)) => "  h" ++ show i ++ "[\"" ++ h.name ++ ".eth0\"]\n") (indexed (allHosts n)) ++
  concatMap (\(a,b) => "  " ++ a ++ " --- " ++ b ++ "\n") (nub (map canonical (mapMaybe edge (allPorts n))))
  where
    edge : (PortRef,Port) -> Maybe (String,String)
    edge (r,p) = map (\peer => (endpointId (DevicePort r),endpointId peer.value)) p.peer
    canonical : (String,String) -> (String,String)
    canonical (a,b) = if a < b then (a,b) else (b,a)

public export
dependencyGraph : StableNetwork -> String
dependencyGraph stable = let n = stable.model in
  "flowchart LR\n" ++ concatMap (\(i,s) => "  s" ++ show i ++ "[\"" ++ s.name ++ "\"]\n") (indexed n.services) ++
  concatMap (\(i,r) => "  r" ++ show i ++ "[\"" ++ r.name ++ "\"]\n") (indexed n.routes) ++
  concatMap (\(a,b) => "  s" ++ show a ++ " --> s" ++ show b ++ "\n") (serviceEdges n) ++
  concatMap (\(a,b) => "  r" ++ show a ++ " --> r" ++ show b ++ "\n") (routeEdges n)

public export
markdown : StableNetwork -> String
markdown stable = let n = stable.model in
  "# Network " ++ n.name ++ "\n\nLanguage 1.0. State: **Desired**. Domain: " ++ fromMaybe "unspecified" n.domain ++ ".\n\n" ++
  "Model invariants and dependency ordering are certificate-checked by the Idris semantic core. Target realization is conditional on documented profiles. Applied state, physical connectivity, service health and observations are **Unknown**.\n\n" ++
  "## VLANs and addressing\n\n" ++ table ["VLAN","ID","IPv4 prefix","Gateway","DHCP"]
    (map (\v => [v.name,show v.vid.number,showPrefix v.subnet.value,maybe "none" (showIPv4 . value) v.gateway,
      maybe "disabled" (\r => showIPv4 r.first.value ++ " .. " ++ showIPv4 r.last.value) v.dhcp]) n.vlans) ++
  "## Hosts\n\nStatic IP assignments must be configured on the hosts. No MAC-bound reservations are inferred.\n\n" ++ table ["Host","VLAN","Address"]
    (map (\(v,h) => [h.name,vlanLabel n v,showIPv4 h.address.value]) (allHosts n)) ++
  "## Devices and ports\n\n" ++ table ["Device","Driver","Port","Membership","Connects to"]
    (concatMap (\d => map (\p => [d.name,driverName d.driver,p.name,portModeLabel n p.mode,maybe "unspecified" (peerLabel n . value) p.peer]) d.ports) n.devices) ++
  "## Services\n\n" ++ table ["Service","Transport ports","Dependencies"]
    (map (\s => [s.name,join ", " (map (\(t,p) => transportName t ++ "/" ++ show p) s.transports),
      join ", " (map (\r => maybe "invalid" name (lookupAt r.value.index n.services)) s.dependencies)]) n.services) ++
  "## Static routes\n\n" ++ table ["Route","Destination","Next hop","Device","VLAN","Metric"]
    (map (\r => [r.name,showPrefix r.destination.value,showIPv4 r.nextHop.value,deviceLabel n r.device,vlanLabel n r.vlan,show r.metric]) n.routes) ++
  "## Policy matrix\n\nIPv4 connection initiation; established/related return traffic is accepted. New input and forwarding default to deny. Rules use declaration order. Gateway means local input; other destinations mean forwarding. DHCP requires gateway UDP/67 and TCP+UDP/53; contradictory denials are rejected. NAT is not inferred. Existing flows are not revoked.\n\n" ++ table ["From","To","Action","Services"]
    (map (\p => [vlanLabel n p.from,endpointLabel n p.destination,actionName p.action,
      if null p.services then "all IPv4 protocols" else join ", " (map (\r => maybe "invalid" name (lookupAt r.index n.services)) p.services)]) n.policies) ++
  "## AAA and migration\n\nAAA realization and migration source syntax are deferred beyond language 1.0. No AAA assurance or operational observation is inferred. This document represents a stable model with no migration debt.\n\n" ++
  "## Physical topology\n\n```mermaid\n" ++ graph stable ++ "```\n\n## Dependency graph\n\n```mermaid\n" ++ dependencyGraph stable ++ "```\n"

public export
semanticJSON : StableNetwork -> String
semanticJSON stable = let n = stable.model in
  "{\"exportVersion\":\"1.0\",\"languageVersion\":\"1.0\",\"state\":\"Desired\",\"name\":" ++ jsonString n.name ++
  ",\"vlans\":" ++ jsonArray (map (\(i,v) => "{\"id\":" ++ show i ++ ",\"name\":" ++ jsonString v.name ++ ",\"vlanId\":" ++ show v.vid.number ++
    ",\"subnet\":" ++ jsonString (showPrefix v.subnet.value) ++ ",\"gateway\":" ++ maybe "null" (jsonString . showIPv4 . value) v.gateway ++
    ",\"source\":" ++ spanJSON v.source ++ "}") (indexed n.vlans)) ++
  ",\"hosts\":" ++ jsonArray (map (\(i,(v,h)) => "{\"id\":" ++ show i ++ ",\"name\":" ++ jsonString h.name ++ ",\"vlanRef\":" ++ show v.index ++ ",\"address\":" ++ jsonString (showIPv4 h.address.value) ++ "}") (indexed (allHosts n))) ++
  ",\"devices\":" ++ jsonArray (map (\(i,d) => "{\"id\":" ++ show i ++ ",\"name\":" ++ jsonString d.name ++ ",\"driver\":" ++ jsonString (driverName d.driver) ++
    ",\"ports\":" ++ jsonArray (map (\p => "{\"name\":" ++ jsonString p.name ++ ",\"ownerRef\":" ++ show p.owner.index ++ ",\"vlanRefs\":" ++ jsonArray (map (show . index) (memberships p.mode)) ++ "}") d.ports) ++ "}") (indexed n.devices)) ++
  ",\"certificates\":{\"serviceOrder\":" ++ jsonArray (map show stable.serviceCertificate.order) ++ ",\"routeOrder\":" ++ jsonArray (map show stable.routeCertificate.order) ++ "}}"
