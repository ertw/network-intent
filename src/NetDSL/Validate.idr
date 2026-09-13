module NetDSL.Validate

import NetDSL.Common
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import NetDSL.Resolve
import NetDSL.Syntax.Parser
import Data.List
import Data.So
import Data.Maybe
import Data.String

%default total

private
pairs : (a -> a -> List Diagnostic) -> List a -> List Diagnostic
pairs f [] = []
pairs f (x :: xs) = concatMap (f x) xs ++ pairs f xs

private
conflict : String -> SourceSpan -> SourceSpan -> String -> List Diagnostic
conflict code a b title = [MkDiagnostic code b title "" [(a,"Conflicting declaration")]]

private
staticAddresses : Vlan -> List (Located IPv4)
staticAddresses v = maybe [] pure v.gateway ++ map address v.hosts

private
usableAddress : Vlan -> Located IPv4 -> List Diagnostic
usableAddress v a = case assignment v.subnet.value a.span (showIPv4 a.value) of
  Left d => [d]
  Right _ => []

private
ipam : Vlan -> List Diagnostic
ipam v = concatMap (usableAddress v) (staticAddresses v) ++
  pairs (\a,b => if a.value.number == b.value.number then conflict "address.duplicate-static" a.span b.span ("Duplicate static address: " ++ showIPv4 a.value) else []) (staticAddresses v) ++
  case v.dhcp of
    Nothing => []
    Just pool => usableAddress v pool.first ++ usableAddress v pool.last ++
      (if pool.first.value.number <= pool.last.value.number then [] else [failure "address.inverted-dhcp-range" pool.first.span "DHCP start must not exceed its end"]) ++
      (if v.subnet.value.width <= 30 then [] else [failure "address.dhcp-prefix-too-small" pool.first.span "DHCP requires broadcast and host space"]) ++
      (if isJust v.gateway then [] else [failure "address.dhcp-without-gateway" v.source "A DHCP VLAN requires a gateway"]) ++
      concatMap (\a => if a.value.number >= pool.first.value.number && a.value.number <= pool.last.value.number
      then conflict "address.static-dhcp-overlap" pool.first.span a.span ("Static address " ++ showIPv4 a.value ++ " overlaps the DHCP pool") else []) (staticAddresses v)

private
sameMode : PortMode -> PortMode -> Bool
sameMode (Access a) (Access b) = a == b
sameMode (Trunk as) (Trunk bs) = length as == length bs && all (\a => elem a bs) as
sameMode _ _ = False

private
links : Network v -> List (LinkEnd, Located LinkEnd)
links n = concatMap (\(r,p) => maybe [] (\dst => [(DevicePort r,dst)]) p.peer) (allPorts n)

private
peerChecks : Network v -> (PortRef, Port) -> List Diagnostic
peerChecks n (r,p) = case p.peer of
  Nothing => []
  Just dst => if dst.value == DevicePort r then [failure "topology.self-link" dst.span "A port cannot connect to itself"]
    else case dst.value of
      DevicePort target => case getPort n target of
        Nothing => [failure "topology.unknown-port" dst.span "Link target does not exist"]
        Just other => if sameMode p.mode other.mode then [] else conflict "topology.incompatible-link" other.source dst.span "Connected ports have incompatible tagged/untagged VLAN memberships"
      HostPort host => case lookupAt host.index (allHosts n) of
        Nothing => [failure "topology.unknown-host" dst.span "Host link target does not exist"]
        Just (v,h) => case p.mode of
          Access carried => if v == carried then [] else conflict "topology.host-vlan-mismatch" h.source dst.span "Host address belongs to a different VLAN than its access port"
          Trunk _ => [failure "topology.host-trunk" dst.span "Implicit host eth0 cannot terminate a tagged trunk"]

private
degreeChecks : Network v -> List Diagnostic
degreeChecks n = concatMap check endpoints
  where
    edges : List (LinkEnd,Located LinkEnd)
    edges = links n
    endpoints : List LinkEnd
    endpoints = nub (concatMap (\(a,b) => [a,b.value]) edges)
    check : LinkEnd -> List Diagnostic
    check endpoint =
      let peers = map (\(a,b) => (if a == endpoint then b.value else a,b.span)) (filter (\(a,b) => a == endpoint || b.value == endpoint) edges) in
      if length (nub (map fst peers)) <= 1 then [] else
        case peers of
          (_,a) :: (_,b) :: _ => conflict "topology.multiple-peers" a b "A physical interface is connected to more than one peer"
          _ => []

private
portChecks : Network v -> (PortRef,Port) -> List Diagnostic
portChecks n (r,p) =
  (if p.owner == r.owner then [] else [failure "topology.owner-mismatch" p.source "Port owner differs from its containing device"]) ++
  (if null (memberships p.mode) then [failure "topology.empty-membership" p.source "Port requires VLAN membership"] else []) ++
  (if length (nub (memberships p.mode)) == length (memberships p.mode) then [] else [failure "topology.duplicate-membership" p.source "Duplicate VLAN membership"]) ++
  concatMap (\v => case getVlan n v of
    Nothing => [failure "reference.unknown-vlan" p.source "Port VLAN reference is outside the inventory"]
    Just _ => []) (memberships p.mode) ++ peerChecks n (r,p)

public export
carries : Device -> Ref VLAN -> Bool
carries d v = any (elem v . memberships . mode) d.ports

private
routeChecks : Network v -> Route -> List Diagnostic
routeChecks n r =
  (case getVlan n r.vlan of
    Nothing => [failure "reference.unknown-vlan" r.source "Route VLAN reference is outside the inventory"]
    Just v => case assignment v.subnet.value r.nextHop.span (showIPv4 r.nextHop.value) of
      Left _ => [failure "routing.next-hop-unreachable" r.nextHop.span "Static next hop must be a usable on-link address in the selected VLAN"]
      Right _ => case v.gateway of
        Just gw => if gw.value.number == r.nextHop.value.number then [failure "routing.next-hop-self" r.nextHop.span "Route next hop is the router's own gateway address"] else []
        Nothing => [failure "routing.missing-gateway" r.source "Route VLAN requires a gateway address"]) ++
  (case getDevice n r.device of
    Nothing => [failure "reference.unknown-device" r.source "Route device reference is outside the inventory"]
    Just d => if d.isRouter && carries d r.vlan then [] else [failure "routing.invalid-egress" r.source "Route owner must be a router carrying its selected VLAN"])
  ++ (if r.metric >= 0 && r.metric <= 4294967295 then [] else [failure "routing.invalid-metric" r.source "Route metric must fit in an unsigned 32-bit integer"])

private
policyChecks : Network v -> Policy -> List Diagnostic
policyChecks n p = case n.enforcer >>= getDevice n of
  Nothing => [failure "policy.no-enforcer" p.source "Stateful policy requires one routing owner"]
  Just d => concatMap (zoneCheck d) (the (List (Ref VLAN)) (p.from :: (case p.destination of Zone v => [v]; _ => []))) ++
    concatMap (\s => if s.index < length n.services then [] else [failure "reference.unknown-service" p.source "Policy service reference is outside the inventory"]) p.services ++
    (case p.destination of
      Gateway r => if Just r == n.enforcer then [] else [failure "policy.invalid-gateway" p.source "Gateway endpoint must reference the routing owner"]
      Zone v => if v == p.from then [failure "policy.same-zone" p.source "Inter-zone policy cannot govern traffic within one VLAN"] else []
      _ => []) ++ dhcpConflict
  where
    dhcpConflict : List Diagnostic
    dhcpConflict = case (p.action,p.destination,getVlan n p.from) of
      (Deny,Gateway _,Just v) => if isJust v.dhcp &&
        (null p.services || any (\r => maybe False (any (\(proto,port) => (proto == UDP && port == 67) || port == 53) . transports) (lookupAt r.index n.services)) p.services)
        then [failure "policy.dhcp-control-conflict" p.source "DHCP requires gateway DHCP and DNS control traffic; this denial contradicts the DHCP declaration"] else []
      _ => []

    zoneCheck : Device -> Ref VLAN -> List Diagnostic
    zoneCheck d v = case getVlan n v of
      Nothing => [failure "reference.unknown-vlan" p.source "Policy VLAN reference is outside the inventory"]
      Just zone => if carries d v && isJust zone.gateway then [] else [failure "policy.uncovered-zone" p.source ("Routing owner must carry VLAN " ++ zone.name ++ " and it must declare a gateway")]

private
unique : String -> List (String,SourceSpan) -> List Diagnostic
unique kind = pairs (\(a,sa),(b,sb) => if a == b then conflict "name.duplicate" sa sb ("Duplicate " ++ kind ++ ": " ++ a) else [])

private
checkNames : List (String,SourceSpan) -> List Diagnostic
checkNames = concatMap (\(n,s) => if n /= "" && length n <= 63 && all (\c => ord c < 128 && (isAlphaNum c || c == '-' || c == '_')) (unpack n)
  then [] else [failure "name.invalid" s "Names must contain 1..63 ASCII letters, digits, hyphens or underscores"])

private
serviceDependencyChecks : Network v -> Service -> List Diagnostic
serviceDependencyChecks n service = concatMap check service.dependencies
  where
    check : Located (Ref ServiceKind) -> List Diagnostic
    check d = if d.value.index < length n.services then [] else [failure "reference.unknown-service" d.span "Dependency service reference is outside the inventory"]

private
routeDependencyChecks : Network v -> Route -> List Diagnostic
routeDependencyChecks n route = concatMap check route.dependencies
  where
    check : Located (Ref RouteKind) -> List Diagnostic
    check d = if d.value.index < length n.routes then [] else [failure "reference.unknown-route" d.span "Dependency route reference is outside the inventory"]

private
structural : Network V1 -> List Diagnostic
structural n =
  checkNames ((n.name,n.source) :: map (\v => (v.name,v.source)) n.vlans ++ map (\d => (d.name,d.source)) n.devices ++
    map (\(_,h) => (h.name,h.source)) (allHosts n) ++ map (\s => (s.name,s.source)) n.services ++ map (\r => (r.name,r.source)) n.routes) ++
  unique "VLAN" (map (\v => (v.name,v.source)) n.vlans) ++ unique "VLAN ID" (map (\v => (show v.vid.number,v.source)) n.vlans) ++
  unique "physical endpoint" (map (\d => (d.name,d.source)) n.devices ++ map (\(_,h) => (h.name,h.source)) (allHosts n)) ++
  unique "service" (map (\s => (s.name,s.source)) n.services) ++ unique "route" (map (\r => (r.name,r.source)) n.routes) ++
  concatMap (\d => unique "port" (map (\p => (p.name,p.source)) d.ports)) n.devices ++
  concatMap (\(r,p) =>
    (if p.name /= "" && length p.name <= 63 && all (\c => ord c < 128 && (isAlphaNum c || elem c ['/', '.', '-', '_'])) (unpack p.name) then [] else [failure "name.invalid-interface" p.source "Invalid physical interface label"]) ++
    (case p.description of Nothing => []; Just s => if all (\c => ord c >= 32 && (ord c < 127 || ord c > 159) && ord c /= 8232 && ord c /= 8233) (unpack s) then [] else [failure "backend.unsafe-value" p.source "Description contains control characters"])) (allPorts n) ++
  concatMap (\s => if null s.transports || any (\(_,p) => p < 1 || p > 65535) s.transports then [failure "service.invalid-port" s.source "Services require TCP/UDP ports in 1..65535"] else []) n.services ++
  concatMap (serviceDependencyChecks n) n.services ++
  concatMap (routeDependencyChecks n) n.routes ++
  concatMap (\v => if elem v.name ["internet","gateway"] then [failure "name.reserved" v.source "VLAN name is a reserved policy endpoint"] else []) n.vlans ++
  (case n.domain of Nothing => []; Just s => if s /= "" && length s <= 253 && all (\c => ord c < 128 && (isAlphaNum c || c == '-' || c == '.')) (unpack s) then [] else [failure "name.invalid-domain" n.source "Invalid DNS domain"])

private
ownership : Network V1 -> List Diagnostic
ownership n =
  let routers : List (Ref DeviceKind) = map (\(i,_) => Id i) (filter (\pair => (snd pair).isRouter) (indexed n.devices))
      needsRouter = any (isJust . gateway) n.vlans || not (null n.policies) || not (null n.routes) in
  (if length routers > 1 then [failure "policy.ambiguous-enforcer" n.source "Language 1.0 requires a single routing owner"] else []) ++
  (if n.enforcer == head' routers then [] else [failure "policy.invalid-enforcer" n.source "Enforcer must be the declared routing owner"]) ++
  (if needsRouter && null routers then [failure "network.missing-routing-owner" n.source "Gateway, DHCP, routing and policy intent require a routing owner"] else []) ++
  (case n.enforcer >>= getDevice n of
    Nothing => []
    Just d => concatMap (\(i,v) => if isJust v.gateway && not (carries d (Id i)) then [failure "network.uncovered-gateway" v.source ("Routing owner does not carry gateway VLAN " ++ v.name)] else []) (indexed n.vlans))

public export
validate : Network V1 -> List Diagnostic
validate n = structural n ++ ownership n ++ concatMap ipam n.vlans ++
  pairs (\a,b => if overlaps a.subnet.value b.subnet.value then conflict "address.prefix-overlap" a.subnet.span b.subnet.span (showPrefix a.subnet.value ++ " overlaps " ++ showPrefix b.subnet.value) else []) n.vlans ++
  concatMap (portChecks n) (allPorts n) ++ degreeChecks n ++
  concatMap (routeChecks n) n.routes ++ concatMap (policyChecks n) n.policies

public export
record StableNetwork where
  constructor Certified
  model : Network V1
  serviceCertificate : TopologicalCertificate (nodeIds model.services) (serviceEdges model)
  routeCertificate : TopologicalCertificate (nodeIds model.routes) (routeEdges model)
  0 invariants : So (null (validate model))

private
cycleDiagnostic : String -> List (String,SourceSpan) -> SourceSpan -> CycleWitness -> Diagnostic
cycleDiagnostic code nodes fallback witness =
  let selected = mapMaybe (\i => lookupAt i nodes) witness.path
      at = maybe fallback snd (head' selected) in
  MkDiagnostic code at ("Dependency cycle: " ++ join " -> " (map fst selected))
    "Dependencies must terminate. The reported path is a cycle witness, not a physical-topology cycle."
    (map (\(n,s) => (s,n)) selected)

public export
certify : (n : Network V1) -> Either (List Diagnostic) StableNetwork
certify n = case choose (null (validate n)) of
  Right _ => Left (validate n)
  Left prf => do
    serviceCert <- case certifyDAG (map fst (indexed n.services)) (serviceEdges n) of
      Left (DirectedCycle witness) => Left [cycleDiagnostic "service.dependency-cycle" (map (\s => (s.name,s.source)) n.services) n.source witness]
      Left (InvalidGraph msg) => Left [failure "graph.invalid-input" n.source msg]
      Right c => Right c
    routeCert <- case certifyDAG (map fst (indexed n.routes)) (routeEdges n) of
      Left (DirectedCycle witness) => Left [cycleDiagnostic "routing.dependency-cycle" (map (\r => (r.name,r.source)) n.routes) n.source witness]
      Left (InvalidGraph msg) => Left [failure "graph.invalid-input" n.source msg]
      Right c => Right c
    Right (Certified n serviceCert routeCert prf)

public export
elaborate : Document -> Either (List Diagnostic) StableNetwork
elaborate doc = resolve doc >>= certify
