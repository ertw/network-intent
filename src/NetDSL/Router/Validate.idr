module NetDSL.Router.Validate

import NetDSL.Common
import NetDSL.Router.Model
import NetDSL.Router.CheckOptions
import NetDSL.Router.Fields
import NetDSL.Domain.Address
import NetDSL.Graph.DAG
import NetDSL.AAA
import Data.List
import Data.Maybe
import Data.String

%default total

private
ensure : String -> SourceSpan -> Bool -> String -> List Diagnostic
ensure code at ok msg = if ok then [] else [failure code at msg]

private
unique : List (String,SourceSpan) -> List Diagnostic
unique xs = either pure (const []) (uniqueNames xs)

private
netdevName : String -> Bool
netdevName s = validName s && length s <= 15 && s /= "lo" && s /= "none"

private
refOK : List a -> RouterRef k -> Bool
refOK xs r = r.index < length xs

public export
attachmentId : RouterConfig -> Attachment -> Nat
attachmentId c (Physical r) = r.index
attachmentId c (BridgeDevice r) = length c.physicals + r.index
attachmentId c (BondDevice r) = length c.physicals + length c.bridges + r.index
attachmentId c Loopback = length c.physicals + length c.bridges + length c.bonds
attachmentId c Unattached = S (length c.physicals + length c.bridges + length c.bonds)

public export
attachmentOK : RouterConfig -> Attachment -> Bool
attachmentOK c (Physical r) = refOK c.physicals r
attachmentOK c (BridgeDevice r) = refOK c.bridges r
attachmentOK c (BondDevice r) = refOK c.bonds r
attachmentOK c Loopback = True
attachmentOK c Unattached = True

public export
attachmentName : RouterConfig -> Attachment -> String
attachmentName c (Physical r) = maybe "invalid" name (lookupAt r.index c.physicals)
attachmentName c (BridgeDevice r) = maybe "invalid" name (lookupAt r.index c.bridges)
attachmentName c (BondDevice r) = maybe "invalid" name (lookupAt r.index c.bonds)
attachmentName c Loopback = "lo"
attachmentName c Unattached = "none"

public export
linkEdges : RouterConfig -> List (Nat,Nat)
linkEdges c = concatMap (\(i,b) => map (\m => (attachmentId c m.value, length c.physicals + i)) b.members) (indexed c.bridges) ++
  concatMap (\(i,b) => map (\m => (m.value.index, length c.physicals + length c.bridges + i)) b.members) (indexed c.bonds)

private
usable4 : Address4 -> Bool
usable4 a = contains a.subnet a.address &&
  (a.subnet.width >= 31 || (a.address.number /= a.subnet.address.number && a.address.number /= a.subnet.address.number + a.subnet.size - 1))

private
hexString : String -> Bool
hexString s = all (\c => (c >= '0' && c <= '9') || elem (toLower c) (unpack "abcdef")) (unpack s)

private
macValid : String -> Bool
macValid s = let parts = splitOn ':' s in length parts == 6 && all (\p => length p == 2 && hexString p) parts

private
timeValue : String -> Bool
timeValue s = s == "infinite" || case reverse (unpack s) of
  unit :: ds => elem unit ['s','m','h','d','w'] && maybe False (>0) (decimal (pack (reverse ds)))
  _ => False

private
rateValue : String -> Bool
rateValue s = case splitOn '/' s of
  [n,unit] => maybe False (\v => v > 0 && v <= 4294967295) (decimal n) && elem unit ["s","sec","second","minute","hour","day"]
  _ => False

private
icmpValid : String -> Bool
icmpValid s = elem s ["echo-request","echo-reply","destination-unreachable","packet-too-big","time-exceeded","bad-header","unknown-header-type","router-solicitation","neighbour-solicitation","router-advertisement","neighbour-advertisement"] ||
  case splitOn '/' s of
    [t,c] => all (\x => maybe False (<=255) (decimal x)) [t,c]
    [t] => maybe False (<=255) (decimal t)
    _ => False

private
utf8Length : String -> Nat
utf8Length = sum . map (\c => if ord c < 128 then 1 else if ord c < 2048 then 2 else if ord c < 65536 then 3 else 4) . unpack

private
checkInterface : RouterConfig -> LogicalInterface -> List Diagnostic
checkInterface c i = checkInterfaceOptions i.source i.settings ++
  ensure "reference.unknown-attachment" i.attachment.span (attachmentOK c i.attachment.value) "Interface attachment does not exist" ++
  ensure "topology.slave-interface" i.attachment.span (not (elem (attachmentId c i.attachment.value) (map fst (linkEdges c)))) "Logical interfaces must attach to the bridge/bond master, not an enslaved member" ++
  ensure "router.protocol-required" i.source (isJust i.settings.protocol) "Interface protocol is required" ++
  ensure "address.protocol-conflict" i.source
    (if i.settings.protocol == Just Static then not (null i.addresses && null i.addresses6) else null i.addresses && null i.addresses6)
    "Static interfaces need addresses; dynamic and unnumbered interfaces cannot declare static addresses" ++
  ensure "router.dhcpv6-options" i.source
    (i.settings.protocol == Just DHCPv6Client || all not [isJust i.settings.requestAddress,isJust i.settings.requestPrefix,isJust i.settings.noRelease])
    "DHCPv6 client options require protocol dhcpv6" ++
  ensure "router.ipv6-assignment" i.source (not (isJust i.settings.ipv6Assignment) || i.settings.protocol == Just Static) "IPv6 assignment requires a static downstream interface" ++
  ensure "router.unattached-protocol" i.source (case i.attachment.value of Unattached => elem i.settings.protocol [Just DHCPClient,Just DHCPv6Client,Just Unnumbered]; _ => True) "Unattached interfaces require a dynamic or unnumbered protocol" ++
  maybe [] (\g => ensure "routing.gateway-unreachable" g.span
    (i.settings.protocol == Just Static && all (\a => g.value.number /= a.value.address.number) i.addresses && any (\a => usable4 (OnSubnet4 g.value a.value.subnet)) i.addresses)
    "Gateway must be a usable on-link address distinct from this interface address") i.gateway ++
  ensure "router.duplicate-dns" i.source (let ds = map (number . value) i.dnsServers in length ds == length (nub ds)) "Duplicate interface DNS server" ++
  concatMap (\a => ensure "address.outside-prefix" a.span (usable4 a.value) "Interface address must be usable in its subnet") i.addresses

private
checkPool : RouterConfig -> DHCPServer -> List Diagnostic
checkPool c d = checkDHCPOptions d.source d.settings ++
  ensure "reference.unknown-interface" d.ifaceRef.span (refOK c.interfaces d.ifaceRef.value) "DHCP interface does not exist" ++
  ensure "dhcp.invalid-lease" d.source (maybe True timeValue d.settings.leaseTime) "Invalid DHCP lease duration" ++
  ensure "dhcp.duplicate-ra-flag" d.source (length (nub d.flags) == length d.flags && (not (elem NoFlags d.flags) || d.flags == [NoFlags])) "Invalid or duplicate RA flags" ++
  ensure "dhcp.ra-flags" d.source (null d.flags || d.settings.ra == Just Server) "RA flags require an RA server" ++
  ensure "dhcp.ignored-server" d.source (d.settings.ignore /= Just True || (d.settings.ipv4 /= Just Server && d.settings.ipv6 /= Just Server && d.settings.ra /= Just Server)) "Ignored DHCP interface cannot enable servers" ++
  ensure "dhcp.pool-required" d.source (d.settings.ipv4 /= Just Server || isJust d.pool) "IPv4 DHCP server requires a pool" ++
  (case d.pool of
    Nothing => []
    Just (first,last) =>
      ensure "dhcp.pool-mode" d.source ((d.settings.ignore /= Just True && d.settings.ipv4 == Just Server) || (d.settings.ignore == Just True && d.settings.ipv4 == Just Disabled)) "DHCP pool requires an enabled IPv4 server or explicit ignore true and ipv4 disabled" ++
      (case lookupAt d.ifaceRef.value.index c.interfaces of
        Just i => case i.addresses of
          [a] =>
            ensure "address.invalid-dhcp-range" first.span
              (i.settings.protocol == Just Static && a.value.subnet.width < 31 && first.value.number <= last.value.number &&
               usable4 (OnSubnet4 first.value a.value.subnet) && usable4 (OnSubnet4 last.value a.value.subnet)) "DHCP endpoints must be ordered usable addresses within the static interface subnet" ++
            concatMap (\addr => ensure "address.static-dhcp-overlap" first.span
              (d.settings.ignore == Just True || addr.value.address.number < first.value.number || addr.value.address.number > last.value.number) "DHCP pool overlaps a static interface address") (concatMap addresses c.interfaces)
          _ => [failure "address.dhcp-subnet" d.source "DHCP requires exactly one static IPv4 interface subnet"]
        Nothing => [])) ++
  (if d.settings.ipv6 == Just Server || d.settings.ra == Just Server then
    case lookupAt d.ifaceRef.value.index c.interfaces of
      Just i => ensure "dhcp.ipv6-subnet" d.source (isJust i.settings.ipv6Assignment || not (null i.addresses6)) "IPv6 services require an assigned or static IPv6 prefix"
      Nothing => []
    else [])

private
checkRule : RouterConfig -> FirewallRule -> List Diagnostic
checkRule c r = checkRuleOptions r.source r.settings ++
  ensure "reference.unknown-zone" r.from.span (refOK c.zones r.from.value) "Rule source zone does not exist" ++
  (case r.destination.value of
    NamedZone z => ensure "reference.unknown-zone" r.destination.span (refOK c.zones z) "Rule destination zone does not exist"
    _ => []) ++
  ensure "policy.action-required" r.source (isJust r.settings.action) "Firewall rule action is required" ++
  ensure "policy.protocol-required" r.source (not (null r.protocols) && length (nub r.protocols) == length r.protocols && (not (elem ProtoAll r.protocols) || r.protocols == [ProtoAll])) "Declare nonduplicate protocols; all must occur alone" ++
  ensure "policy.port-protocol" r.source (isNothing r.settings.destinationPort || all (\p => elem p [ProtoTCP,ProtoUDP]) r.protocols) "Destination ports require TCP/UDP protocols" ++
  ensure "policy.icmp-protocol" r.source (null r.icmpTypes || all (\p => elem p [ProtoICMP,ProtoICMP6]) r.protocols) "ICMP types require ICMP protocols" ++
  ensure "policy.icmp-type" r.source (all icmpValid r.icmpTypes) "Unsupported ICMP type or code" ++
  ensure "policy.icmp-family" r.source
    (r.settings.family == Just Family6 || all (\t => elem t ["echo-request","echo-reply","destination-unreachable","time-exceeded"] || maybe False (<=255) (decimal (fromMaybe "" (head' (splitOn '/' t))))) r.icmpTypes)
    "IPv6-specific ICMP type names require family ipv6" ++
  ensure "policy.rate" r.source (maybe True rateValue r.settings.limit) "Invalid firewall rate limit" ++
  ensure "policy.family-protocol" r.source
    ((not (elem ProtoIGMP r.protocols) || r.settings.family == Just Family4) &&
     (not (elem ProtoICMP6 r.protocols) || r.settings.family == Just Family6)) "IGMP requires IPv4 and ICMPv6 requires IPv6" ++
  (case r.sourcePrefix of
    Just (Source4 _) => ensure "policy.family-prefix" r.source (r.settings.family == Just Family4) "IPv4 source prefix requires family ipv4"
    Just (Source6 _) => ensure "policy.family-prefix" r.source (r.settings.family == Just Family6) "IPv6 source prefix requires family ipv6"
    Nothing => [])

private
checkRadio : Radio -> List Diagnostic
checkRadio r = checkRadioOptions r.source r.settings ++
  ensure "wireless.required-setting" r.source (all id [isJust r.settings.driver,isJust r.settings.path,isJust r.settings.band,isJust r.settings.channel,isJust r.settings.width]) "Radio requires driver, path, band, channel, and width" ++
  ensure "wireless.path" r.source (maybe False (/= "") r.settings.path) "Radio hardware path cannot be empty" ++
  ensure "wireless.country" r.source (maybe True (\s => length s == 2 && all (\c => c >= 'A' && c <= 'Z') (unpack s)) r.settings.country) "Country must be two uppercase ASCII letters" ++
  ensure "wireless.band-width" r.source
    (r.settings.band /= Just Band2 || not (elem r.settings.width (map Just [HE80,HE160,VHT20,VHT40,VHT80,VHT160]))) "Unsupported 2.4 GHz width" ++
  ensure "wireless.band-channel" r.source (r.settings.band /= Just Band2 || maybe False (<=14) r.settings.channel) "2.4 GHz channel must be 1..14"

private
checkWiFi : RouterConfig -> WiFiInterface -> List Diagnostic
checkWiFi c a = checkWiFiOptions a.source a.settings ++
  ensure "reference.unknown-radio" a.radio.span (refOK c.radios a.radio.value) "AP radio does not exist" ++
  ensure "reference.unknown-interface" a.ifaceRef.span (refOK c.interfaces a.ifaceRef.value) "AP interface does not exist" ++
  ensure "wireless.required-setting" a.source (isJust a.settings.mode && isJust a.settings.ssid && isJust a.settings.security) "Access point requires mode, SSID, and security" ++
  ensure "wireless.ssid" a.source (maybe False (\s => utf8Length s >= 1 && utf8Length s <= 32) a.settings.ssid) "SSID must contain 1..32 UTF-8 bytes" ++
  ensure "secret.wifi-credential" a.source (case a.settings.security of
    Just Open => isNothing a.credential
    Just _ => isJust a.credential
    Nothing => False) "Secured APs require an opaque credential reference; open APs cannot have credentials" ++
  ensure "secret.invalid-reference" a.source (maybe True (validWiFiReference . secretURI) a.credential) "Invalid Wi-Fi credential reference" ++
  ensure "wireless.bssid" a.source (maybe True macValid a.settings.bssid) "Invalid station BSSID" ++
  ensure "wireless.mac-address" a.source (maybe True macValid a.settings.macAddress) "Invalid wireless MAC address" ++
  ensure "wireless.station-fields" a.source (isNothing a.settings.bssid || a.settings.mode == Just Station) "BSSID selection requires station mode" ++
  ensure "wireless.hidden-mode" a.source (isNothing a.settings.hidden || a.settings.mode == Just AccessPoint) "Hidden SSID requires AP mode" ++
  ensure "wireless.station-bridge" a.source (a.settings.mode /= Just Station || a.settings.wds == Just True) "Bridged stations require WDS four-address operation" ++
  ensure "wireless.bridge-required" a.source (case lookupAt a.ifaceRef.value.index c.interfaces of
    Just i => case i.attachment.value of BridgeDevice _ => True; _ => False
    Nothing => True) "AP must attach to a bridged logical interface"

public export
leaseCount : DHCPServer -> Integer
leaseCount d = if d.settings.ignore == Just True || d.settings.ipv4 /= Just Server then 0 else case d.pool of
  Nothing => 0
  Just (a,b) => b.value.number - a.value.number + 1

private
pairChecks : (a -> a -> List Diagnostic) -> List a -> List Diagnostic
pairChecks f [] = []
pairChecks f (x :: xs) = concatMap (f x) xs ++ pairChecks f xs

private
prefixChecks : LogicalInterface -> LogicalInterface -> List Diagnostic
prefixChecks a b =
  concatMap (\x => concatMap (\y => ensure "address.prefix-overlap" b.source (not (overlaps x.value.subnet y.value.subnet)) "Static interface IPv4 subnets overlap in one routing context") b.addresses) a.addresses ++
  concatMap (\x => concatMap (\y =>
    let width = min x.value.width y.value.width
        size = pow2 (128 `minus` width) in
    ensure "address.prefix-overlap" b.source (div x.value.address.number size /= div y.value.address.number size) "Static interface IPv6 subnets overlap in one routing context") b.addresses6) a.addresses6

private
controlVerdict : RouterConfig -> Nat -> IPProtocol -> Nat -> Verdict
controlVerdict c zone proto port = fromMaybe Drop (go c.rules)
  where
    go : List FirewallRule -> Maybe Verdict
    go [] = lookupAt zone c.zones >>= (.settings.input)
    go (r :: rest) =
      let local = case r.destination.value of LocalInput => True; _ => False in
      if r.from.value.index == zone && local && r.settings.family /= Just Family6 &&
        (elem ProtoAll r.protocols || elem proto r.protocols) &&
        maybe True (==port) r.settings.destinationPort && null r.icmpTypes
      then if isNothing r.sourcePrefix && isNothing r.settings.limit then r.settings.action
        else if r.settings.action == Just Accept then go rest else r.settings.action
      else go rest

private
checkControl : RouterConfig -> DHCPServer -> List Diagnostic
checkControl c d = if leaseCount d == 0 then [] else
  case find (\(_,z) => elem d.ifaceRef.value.index (map (index . value) z.interfaces)) (indexed c.zones) of
    Nothing => [failure "policy.dhcp-zone" d.source "DHCP-serving interface requires a firewall zone"]
    Just (i,z) => ensure "policy.dhcp-control-conflict" d.source
      (all (\(proto,port) => controlVerdict c i proto port == Accept) [(ProtoUDP,67),(ProtoUDP,53),(ProtoTCP,53)])
      "DHCP server requires permitted IPv4 router input for UDP/67 and TCP+UDP/53"

public export
validateRouter : RouterConfig -> List Diagnostic
validateRouter c =
  unique (map (\p => (p.name,p.source)) c.physicals ++ map (\b => (b.name,b.source)) c.bridges ++ map (\b => (b.name,b.source)) c.bonds) ++
  concat [unique (map (\i => (i.name,i.source)) c.interfaces), unique (map (\z => (z.name,z.source)) c.zones), unique (map (\r => (r.name,r.source)) c.rules), unique (map (\r => (r.name,r.source)) c.radios), unique (map (\a => (a.name,a.source)) c.wifiInterfaces), unique (map (\d => (d.name,d.source)) c.dhcpServers)] ++
  concatMap (\(n,at) => ensure "name.invalid" at (validName n) "Invalid router entity name")
    (map (\i => (i.name,i.source)) c.interfaces ++ map (\z => (z.name,z.source)) c.zones ++ map (\r => (r.name,r.source)) c.rules ++ map (\r => (r.name,r.source)) c.radios ++ map (\a => (a.name,a.source)) c.wifiInterfaces ++ map (\d => (d.name,d.source)) c.dhcpServers) ++
  concatMap (\(n,at) => ensure "backend.capability-mismatch" at (netdevName n) "Link name must be at most 15 ASCII identifier characters and cannot be lo")
    (map (\p => (p.name,p.source)) c.physicals ++ map (\b => (b.name,b.source)) c.bridges ++ map (\b => (b.name,b.source)) c.bonds) ++
  concatMap (\b => ensure "topology.empty-bridge" b.source (not (null b.members)) "Bridge requires members" ++
    concatMap (\m => ensure "reference.unknown-attachment" m.span (attachmentOK c m.value && attachmentId c m.value < attachmentId c Loopback) "Bridge member must reference a declared link") b.members) c.bridges ++
  concatMap (\b => checkBondOptions b.source b.settings ++
    ensure "topology.empty-bond" b.source (length b.members >= 2) "LACP bond requires at least two physical members" ++
    ensure "bond.policy-required" b.source (b.settings.policy == Just LACP) "Bond requires policy 802.3ad" ++
    ensure "bond.min-links" b.source (maybe True (<= length b.members) b.settings.minLinks) "Minimum active links exceeds bond membership" ++
    ensure "bond.mac-address" b.source (maybe True macValid b.settings.macAddress) "Invalid bond MAC address" ++
    concatMap (\m => ensure "reference.unknown-physical" m.span (refOK c.physicals m.value) "Bond physical member does not exist") b.members) c.bonds ++
  ensure "topology.multiple-masters" c.source (let members = map fst (linkEdges c) in length members == length (nub members)) "A link cannot be a member of multiple bridges/bonds or occur twice" ++
  (case certifyDAG (map fst (indexed (replicate (attachmentId c Loopback) ()))) (linkEdges c) of
    Right _ => []
    Left (DirectedCycle _) => [failure "topology.attachment-cycle" c.source "Bridge/bond attachments contain a directed cycle"]
    Left (InvalidGraph _) => [failure "reference.unknown-attachment" c.source "Attachment graph contains an invalid reference"]) ++
  concatMap (checkInterface c) c.interfaces ++
  pairChecks prefixChecks c.interfaces ++
  ensure "address.duplicate-ip" c.source (let ips = map (number . address . value) (concatMap addresses c.interfaces) in length ips == length (nub ips)) "Duplicate static IPv4 interface address" ++
  ensure "address.duplicate-ipv6" c.source (let ips = map (number . address . value) (concatMap addresses6 c.interfaces) in length ips == length (nub ips)) "Duplicate static IPv6 interface address" ++
  maybe [] (\g => checkGlobalOptions g.span g.value ++
    ensure "router.duid" g.span (maybe True (\s => length s >= 4 && length s <= 260 && mod (the Integer (cast (length s))) 2 == 0 && hexString s) g.value.dhcpDefaultDuid) "DUID must be an even-length hexadecimal value") c.globals ++
  maybe [] (\d => checkDNSOptions d.span d.value ++
    ensure "dhcp.lease-capacity" d.span (maybe True (\n => cast n >= sum (map leaseCount c.dhcpServers)) d.value.leaseMax) "DNS lease capacity is smaller than the declared DHCP pools") c.dns ++
  ensure "dhcp.dns-required" c.source (not (any ((>0) . leaseCount) c.dhcpServers) || isJust c.dns) "IPv4 DHCP serving requires DNS daemon configuration" ++
  ensure "dhcp.odhcp-required" c.source (not (any (\d => d.settings.ipv6 == Just Server || d.settings.ra == Just Server) c.dhcpServers) || isJust c.odhcp) "IPv6 serving requires odhcp configuration" ++
  concatMap (checkPool c) c.dhcpServers ++
  concatMap (checkControl c) c.dhcpServers ++
  ensure "dhcp.duplicate-interface" c.source (let refs = map (index . value . ifaceRef) c.dhcpServers in length refs == length (nub refs)) "Only one DHCP declaration is permitted per interface" ++
  ensure "backend.capability-mismatch" c.source (sum (map leaseCount c.dhcpServers) <= 65535) "OpenWrt profile permits at most 65535 DHCP leases" ++
  maybe [] (\d => checkODHCPOptions d.span d.value) c.odhcp ++
  checkFirewallDefaults c.defaults.span c.defaults.value ++
  ensure "policy.defaults-required" c.defaults.span (all isJust [c.defaults.value.input,c.defaults.value.output,c.defaults.value.forward]) "Explicit input/output/forward defaults are required" ++
  concatMap (\z => checkZoneOptions z.source z.settings ++
    ensure "policy.zone-empty" z.source (not (null z.interfaces)) "Zone requires logical interfaces" ++
    ensure "policy.zone-defaults" z.source (all isJust [z.settings.input,z.settings.output,z.settings.forward]) "Zone requires explicit input/output/forward policies" ++
    concatMap (\i => ensure "reference.unknown-interface" i.span (refOK c.interfaces i.value) "Zone interface does not exist") z.interfaces) c.zones ++
  ensure "policy.multiple-zones" c.source (let refs = concatMap (map (index . value) . interfaces) c.zones in length refs == length (nub refs)) "Logical interface may belong to only one firewall zone" ++
  concatMap (\f => ensure "reference.unknown-zone" f.source (refOK c.zones f.from.value && refOK c.zones f.destination.value) "Forwarding zone does not exist" ++
    ensure "policy.same-zone" f.source (f.from.value /= f.destination.value) "Forwarding requires distinct zones") c.forwardings ++
  concatMap (checkRule c) c.rules ++
  concatMap checkRadio c.radios ++
  concatMap (checkWiFi c) c.wifiInterfaces
