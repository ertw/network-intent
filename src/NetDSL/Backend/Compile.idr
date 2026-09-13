module NetDSL.Backend.Compile

import NetDSL.Common
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Validate
import NetDSL.Backend.AST
import Data.List
import Data.Maybe
import Data.String

%default total

private
zoneName : Vlan -> String
zoneName v = "v" ++ show v.vid.number

private
vlanName : Network v -> Ref VLAN -> String
vlanName n r = maybe "invalid" zoneName (getVlan n r)

private
ownedVlans : Network v -> Device -> List (Ref VLAN,Vlan)
ownedVlans n d = map (\(i,v) => (Id i,v)) (filter (\(i,v) => carries d (Id i)) (indexed n.vlans))

private
capabilities : Network V1 -> Device -> Nat -> List Diagnostic
capabilities n d limit =
  (if d.isRouter && leaseCount > 65535 then [failure "backend.capability-mismatch" d.source "OpenWrt v1 profile supports at most 65535 dynamic DHCP leases"] else []) ++
  concatMap (\p => case p.mode of
    Access _ => []
    Trunk vs => if length vs > min limit d.maxTagged then [failure "backend.capability-mismatch" p.source ("Target " ++ d.name ++ " port " ++ p.name ++ " permits at most " ++ show (min limit d.maxTagged) ++ " tagged VLANs; intent requires " ++ show (length vs))] else []) d.ports ++
  (if d.driver == CiscoIOS then
    concatMap (\(_,v) => if v.vid.number >= 1002 && v.vid.number <= 1005 then [failure "backend.capability-mismatch" v.source "The Cisco IOS L2 profile reserves VLANs 1002..1005"] else
      if length v.name > 32 then [failure "backend.capability-mismatch" v.source "Cisco VLAN names must not exceed 32 ASCII characters"] else []) (ownedVlans n d) ++
    (if d.isRouter then [failure "backend.capability-mismatch" d.source "Cisco IOS L2 profile cannot own routed addresses, DHCP, static routes, or stateful policy"] else [])
   else []) ++
  concatMap (\p => if validInterface d.driver p.name then [] else [failure "backend.capability-mismatch" p.source ("Interface label is not supported by " ++ driverName d.driver ++ " profile: " ++ p.name)]) d.ports ++
  (case n.enforcer >>= getDevice n of
    Just router => if router.driver /= OpenWrt && not (null n.policies) then [failure "backend.capability-mismatch" router.source "Stateful policy requires the OpenWrt firewall4 realization profile"] else []
    Nothing => []) ++
  (if d.isRouter then concatMap (\(i,v) => if isJust v.gateway && not (carries d (Id i))
    then [failure "backend.capability-mismatch" v.source ("Routing owner " ++ d.name ++ " has no port carrying gateway VLAN " ++ v.name)] else []) (indexed n.vlans) else [])
  where
    leaseCount : Integer
    leaseCount = sum (map (\v => maybe 0 (\r => r.last.value.number-r.first.value.number+1) v.dhcp) n.vlans)

    validInterface : Driver -> String -> Bool
    validInterface OpenWrt s = length s <= 15 && not (elem '/' (unpack s)) && s /= "." && s /= ".."
    validInterface CiscoIOS s =
      let (family,rest) = span isAlpha (unpack s)
          segments = splitOn '/' (pack rest) in
      elem (pack family) ["Gi","GigabitEthernet","Fa","FastEthernet","Te","TenGigabitEthernet","Twe","TwentyFiveGigE","Fo","FortyGigabitEthernet","Hu","HundredGigE","Eth","Ethernet"] &&
      not (null rest) && all (\p => isJust (decimal p)) segments

private
uciNetwork : Network V1 -> Device -> Either Diagnostic UciPackage
uciNetwork n d = do
  bridge <- section d.source ["device " ++ d.name,"DSA bridge"] "device" "net_bridge"
    [("name","br-net"),("type","bridge"),("vlan_filtering","1")] (map (\p => ("ports",p.name)) d.ports)
  vlans <- concat <$> traverse vlan (ownedVlans n d)
  routes <- traverse route (indexed (filter (\r => maybe False ((==d.name) . name) (getDevice n r.device)) n.routes))
  Right (Package "network" (bridge :: vlans ++ routes))
  where
    taggedPort : Ref VLAN -> Port -> List (String,String)
    taggedPort ref p = case p.mode of
      Access v => if v == ref then [("ports",p.name ++ ":u*")] else []
      Trunk vs => if elem ref vs then [("ports",p.name ++ ":t")] else []

    vlan : (Ref VLAN,Vlan) -> Either Diagnostic (List UciSection)
    vlan (ref,v) = do
      membership <- section v.source ["VLAN " ++ v.name,"port memberships"] "bridge-vlan" ("bridge_" ++ zoneName v)
        [("device","br-net"),("vlan",show v.vid.number)] (concatMap (taggedPort ref) d.ports)
      let addressing = case (d.isRouter,v.gateway) of
            (True,Just gw) => [("proto","static"),("ipaddr",showIPv4 gw.value),("netmask",netmask v.subnet.value)]
            _ => [("proto","none")]
      iface <- section v.source ["VLAN " ++ v.name,"logical interface"] "interface" (zoneName v)
        (("device","br-net." ++ show v.vid.number) :: addressing) []
      Right [membership,iface]

    route : (Nat,Route) -> Either Diagnostic UciSection
    route (i,r) = section r.source ["route " ++ r.name] "route" ("route_" ++ show i)
      [("interface",vlanName n r.vlan),("target",showIPv4 r.destination.value.address),("netmask",netmask r.destination.value),
       ("gateway",showIPv4 r.nextHop.value),("metric",show r.metric)] []

private
uciDHCP : Network V1 -> Device -> Either Diagnostic UciPackage
uciDHCP n d = if not d.isRouter then Right (Package "dhcp" []) else do
  let leaseCount = sum (map (\v => maybe 0 (\r => r.last.value.number-r.first.value.number+1) v.dhcp) n.vlans)
  dns <- section d.source ["router DNS and DHCP profile"] "dnsmasq" "net_dns"
    ([("domainneeded","1"),("boguspriv","1"),("localise_queries","1"),("dhcpleasemax",show (max 150 leaseCount))] ++ maybe [] (\s => [("domain",s),("local","/" ++ s ++ "/"),("expandhosts","1")]) n.domain) []
  vlans <- concat <$> traverse vlan (ownedVlans n d)
  Right (Package "dhcp" (dns :: vlans))
  where
    host : Vlan -> (Nat,HostAssignment) -> Either Diagnostic UciSection
    host v (i,h) = section h.source ["host " ++ h.name,"static DNS record; address must be set on host"] "domain"
      (zoneName v ++ "_host_" ++ show i) [("name",h.name),("ip",showIPv4 h.address.value)] []

    vlan : (Ref VLAN,Vlan) -> Either Diagnostic (List UciSection)
    vlan (_,v) = do
      pool <- case v.dhcp of
        Nothing => pure <$> section v.source ["VLAN " ++ v.name,"DHCP disabled"] "dhcp" (zoneName v) [("interface",zoneName v),("ignore","1")] []
        Just range => pure <$> section range.first.span ["VLAN " ++ v.name,"DHCP range"] "dhcp" (zoneName v)
          [("interface",zoneName v),("start",show (range.first.value.number - v.subnet.value.address.number)),
           ("limit",show (range.last.value.number-range.first.value.number+1)),("leasetime","12h"),("dhcpv4","server")] []
      hosts <- traverse (host v) (indexed v.hosts)
      Right (pool ++ hosts)

private
uciFirewall : Network V1 -> Device -> Either Diagnostic UciPackage
uciFirewall n d = if not d.isRouter then Right (Package "firewall" []) else do
  defaults <- section d.source ["stateful policy default profile"] "defaults" "net_defaults"
    [("input","DROP"),("output","ACCEPT"),("forward","DROP"),("synflood_protect","1")] []
  zones <- traverse (zone . snd) (ownedVlans n d)
  wan <- if any ((== Internet) . destination) n.policies then
    pure <$> section d.source ["external WAN binding"] "zone" "net_wan"
      [("name","wan"),("input","DROP"),("output","ACCEPT"),("forward","DROP")] [("network","wan")]
    else Right []
  dhcpRules <- concat <$> traverse (dhcpRule . snd) (filter (isJust . dhcp . snd) (ownedVlans n d))
  rules <- concat <$> traverse policy (indexed n.policies)
  Right (Package "firewall" (defaults :: zones ++ wan ++ dhcpRules ++ rules))
  where
    zone : Vlan -> Either Diagnostic UciSection
    zone v = section v.source ["VLAN " ++ v.name,"security zone"] "zone" ("zone_" ++ zoneName v)
      [("name",zoneName v),("input","DROP"),("output","ACCEPT"),("forward","DROP")] [("network",zoneName v)]

    dhcpRule : Vlan -> Either Diagnostic (List UciSection)
    dhcpRule v = sequence [section v.source ["VLAN " ++ v.name,"DHCP server control traffic"] "rule" ("dhcp_" ++ zoneName v)
      [("name","DHCP " ++ v.name),("src",zoneName v),("proto","udp"),("dest_port","67"),("family","ipv4"),("target","ACCEPT")] [],
      section v.source ["VLAN " ++ v.name,"DNS advertised by DHCP"] "rule" ("dhcp_dns_" ++ zoneName v)
      [("name","DHCP DNS " ++ v.name),("src",zoneName v),("proto","tcp udp"),("dest_port","53"),("family","ipv4"),("target","ACCEPT")] []]

    service : Nat -> Policy -> List (String,String) -> (Nat,Ref ServiceKind) -> Either Diagnostic (List UciSection)
    service i p base (j,ref) = case lookupAt ref.index n.services of
      Nothing => Left (failure "backend.invalid-reference" p.source "Certified policy service reference could not be dereferenced")
      Just s => traverse (\(k,(proto,port)) => section p.source ["policy " ++ show i,"service " ++ s.name,"transport expansion"] "rule"
        ("policy_" ++ show i ++ "_" ++ show j ++ "_" ++ show k)
        (base ++ [("proto",transportName proto),("dest_port",show port)]) []) (indexed s.transports)

    policy : (Nat,Policy) -> Either Diagnostic (List UciSection)
    policy (i,p) = do
      let destination = case p.destination of
            Zone v => [("dest",vlanName n v)]
            Internet => [("dest","wan")]
            Gateway _ => []
      let base = [("src",vlanName n p.from),("family","ipv4"),("target",case p.action of Allow => "ACCEPT"; Deny => "DROP")] ++ destination
      if null p.services then pure <$> section p.source ["policy " ++ show i,"all IPv4 protocols"] "rule" ("policy_" ++ show i) (base ++ [("proto","all")]) []
        else concat <$> traverse (service i p base) (indexed p.services)


private
iosCompile : Network V1 -> Device -> Either Diagnostic TargetAST
iosCompile n d = do
  native <- if any (\p => case p.mode of Trunk _ => True; _ => False) d.ports
    then pure <$> command d.source ["tagged-only trunk profile"] "vlan dot1q tag native" [] [] else Right []
  vlans <- traverse vlan (ownedVlans n d)
  ports <- traverse port d.ports
  Right (IOS (native ++ vlans ++ ports))
  where
    vlan : (Ref VLAN,Vlan) -> Either Diagnostic IOSCommand
    vlan (_,v) = do
      label <- command v.source ["VLAN " ++ v.name] "name" [v.name] []
      command v.source ["VLAN " ++ v.name] "vlan" [show v.vid.number] [label]

    vlanNumber : Ref VLAN -> String
    vlanNumber ref = maybe "invalid" (show . number . vid) (getVlan n ref)

    port : Port -> Either Diagnostic IOSCommand
    port p = do
      description <- case p.description of
        Nothing => Right []
        Just desc => pure <$> command p.source ["port " ++ p.name,"description"] "description" [desc] []
      mode <- case p.mode of
        Access v => sequence [command p.source ["access membership"] "switchport mode access" [] [],
                              command p.source ["access membership"] "switchport access vlan" [vlanNumber v] []]
        Trunk vs => sequence [command p.source ["tagged memberships"] "switchport mode trunk" [] [],
                              command p.source ["tagged memberships"] "switchport trunk allowed vlan" [join "," (map vlanNumber vs)] [],
                              command p.source ["static trunk profile"] "switchport nonegotiate" [] []]
      command p.source ["device " ++ d.name,"port " ++ p.name] "interface" [p.name] (description ++ mode)

public export
compileTarget : StableNetwork -> String -> Nat -> Either (List Diagnostic) Realization
compileTarget stable targetName limit =
  let n = stable.model in case find ((==targetName) . name) n.devices of
    Nothing => Left [failure "backend.unknown-target" n.source ("Unknown target: " ++ targetName)]
    Just d => case capabilities n d limit of
      ds@(_ :: _) => Left ds
      [] => either (Left . pure) Right $ do
        ast <- case d.driver of
          OpenWrt => do
            network <- uciNetwork n d
            dhcp <- uciDHCP n d
            firewall <- uciFirewall n d
            Right (UCI (if d.isRouter then [network,dhcp,firewall] else [network]))
          CiscoIOS => iosCompile n d
        let assumptions = case d.driver of
              OpenWrt => ["OpenWrt DSA/netifd, firewall4 and dnsmasq profile; physical port labels match hardware",
                          "Owns generated bridge/interfaces and, for the router, DHCP/firewall packages; deployment must reconcile existing conflicting configuration",
                          "IPv4 only; IPv6 behavior is outside this profile and must be disabled or separately governed"] ++
                         (if any ((==Internet) . destination) n.policies && d.isRouter then ["Existing external logical interface wan and an upstream route are required; NAT is not inferred"] else [])
              CiscoIOS => ["Catalyst IOS L2 profile supports native VLAN tagging and static trunk configuration",
                           "Owns configured VLAN/interface fields and global native-tagging mode; existing conflicting configuration must be reconciled",
                           "Routing, DHCP and stateful policy are realized by the network router; this target realizes its L2 projection"]
        Right (Intended d.name (case d.driver of OpenWrt => "openwrt-dsa-fw4-ipv4-v1"; CiscoIOS => "cisco-ios-l2-v1") assumptions ast)
