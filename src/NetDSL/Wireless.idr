module NetDSL.Wireless

import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Domain.Address
import NetDSL.Router.Validate
import NetDSL.AAA
import Data.List
import Data.Maybe
import Data.String

%default total

public export
wifiRefJSON : WiFiRef -> String
wifiRefJSON r = "{\"deviceRef\":" ++ show r.owner.index ++ ",\"interfaceRef\":" ++ show r.iface.index ++ "}"

public export
getWiFi : Network v -> WiFiRef -> Maybe (RouterConfig, WiFiInterface)
getWiFi n r = do
  d <- getDevice n r.owner
  c <- d.routing
  a <- lookupAt r.iface.index c.wifiInterfaces
  pure (c,a)

private
ensure : String -> SourceSpan -> Bool -> String -> List Diagnostic
ensure code at ok message = if ok then [] else [failure code at message]

private
checkLink : Network v -> WirelessLink -> List Diagnostic
checkLink n l = case (getWiFi n l.station.value, getWiFi n l.accessPoint.value) of
  (Just (sc,s), Just (ac,a)) =>
    ensure "wireless.link-role" l.source (s.settings.mode == Just Station && a.settings.mode == Just AccessPoint) "Wireless links run from a station to an access point" ++
    ensure "wireless.link-device" l.source (l.station.value.owner /= l.accessPoint.value.owner) "Wireless backhaul endpoints must belong to different devices" ++
    ensure "wireless.link-disabled" l.source (s.settings.disabled /= Just True && a.settings.disabled /= Just True) "Declared wireless links require enabled endpoints" ++
    ensure "wireless.link-wds" l.source (s.settings.wds == Just True && a.settings.wds == Just True) "Bridged wireless links require WDS at both endpoints" ++
    ensure "wireless.link-ssid" l.source (s.settings.ssid == a.settings.ssid) "Wireless link SSIDs differ" ++
    ensure "wireless.link-security" l.source (s.settings.security == a.settings.security) "Wireless link security modes differ" ++
    ensure "wireless.link-credential" l.source (map secretURI s.credential == map secretURI a.credential) "Wireless link endpoints must share the same opaque credential reference" ++
    ensure "wireless.link-bssid" l.source (case (s.settings.bssid,a.settings.macAddress) of
      (Just b,Just m) => toLower b == toLower m
      _ => True) "Station BSSID differs from the declared AP MAC address" ++
    (case (lookupAt s.radio.value.index sc.radios, lookupAt a.radio.value.index ac.radios) of
      (Just sr,Just ar) =>
        ensure "wireless.link-band" l.source (sr.settings.band == ar.settings.band) "Wireless link radio bands differ" ++
        ensure "wireless.link-channel" l.source (sr.settings.channel == ar.settings.channel) "Wireless link configured channels differ"
      _ => [failure "reference.unknown-radio" l.source "Wireless link references an unknown radio"])
  _ => [failure "reference.unknown-wireless-interface" l.source "Wireless link references an unknown device or Wi-Fi interface"]

-- A segment endpoint is a device-local logical interface. Wi-Fi links connect
-- these endpoints; repeated expansion computes the finite connected component.
private
segment : Network v -> WiFiRef -> Maybe (Nat,Nat)
segment n r = do
  (c,a) <- getWiFi n r
  i <- lookupAt a.ifaceRef.value.index c.interfaces
  pure (r.owner.index,a.ifaceRef.value.index)

private
segmentEdges : Network v -> List ((Nat,Nat),(Nat,Nat))
segmentEdges n = mapMaybe (\l => do
  s <- segment n l.station.value
  a <- segment n l.accessPoint.value
  pure (s,a)) n.wirelessLinks

private
reachable : Nat -> List ((Nat,Nat),(Nat,Nat)) -> List (Nat,Nat) -> List (Nat,Nat)
reachable Z edges seen = seen
reachable (S fuel) edges seen =
  let next = nub (seen ++ concatMap (\(a,b) => (if elem a seen then [b] else []) ++ (if elem b seen then [a] else [])) edges) in
  if length next == length seen then seen else reachable fuel edges next

private
segmentMembers : Network v -> List ((Nat,Nat),RouterConfig,LogicalInterface)
segmentMembers n = concatMap (\(d,device) => case device.routing of
  Nothing => []
  Just c => map (\(i,iface) => ((d,i),c,iface)) (indexed c.interfaces)) (indexed n.devices)

private
poolContains : IPv4 -> DHCPServer -> Bool
poolContains address d = leaseCount d > 0 && case d.pool of
  Just (first,last) => address.number >= first.value.number && address.number <= last.value.number
  Nothing => False

private
sharedChecks : ((Nat,Nat),RouterConfig,LogicalInterface) -> ((Nat,Nat),RouterConfig,LogicalInterface) -> List Diagnostic
sharedChecks ((_,ai),ac,a) ((_,bi),bc,b) =
  ensure "address.duplicate-segment-ip" b.source
    (not (any (\x => any (\y => x.value.address.number == y.value.address.number) b.addresses) a.addresses))
    "Duplicate static IPv4 address on the declared wireless bridge segment" ++
  ensure "address.duplicate-segment-ipv6" b.source
    (not (any (\x => any (\y => x.value.address.number == y.value.address.number) b.addresses6) a.addresses6))
    "Duplicate static IPv6 address on the declared wireless bridge segment" ++
  let apools = filter (\d => d.ifaceRef.value.index == ai && leaseCount d > 0) ac.dhcpServers
      bpools = filter (\d => d.ifaceRef.value.index == bi && leaseCount d > 0) bc.dhcpServers in
  ensure "address.segment-dhcp-overlap" b.source
    (not (any (\x => any (poolContains x.value.address) apools) b.addresses || any (\x => any (poolContains x.value.address) bpools) a.addresses))
    "Static address overlaps an active DHCP pool on the wireless bridge segment" ++
  ensure "dhcp.segment-pool-overlap" b.source
    (not (any (\x => any (\y => case (x.pool,y.pool) of
      (Just (af,al),Just (bf,bl)) => af.value.number <= bl.value.number && bf.value.number <= al.value.number
      _ => False) bpools) apools))
    "Active DHCP pools overlap on the wireless bridge segment"

private
checkSegments : Network v -> List Diagnostic
checkSegments n =
  let members = segmentMembers n
      edges = segmentEdges n in
  concatMap (\(i,a) => concatMap (\(j,b) =>
    if i < j && elem (fst b) (reachable (length members) edges [fst a]) then sharedChecks a b else []) (indexed members)) (indexed members)

public export
validateWireless : Network v -> List Diagnostic
validateWireless n = concatMap (checkLink n) n.wirelessLinks ++
  ensure "wireless.multiple-uplinks" n.source
    (let stations = map (value . station) n.wirelessLinks in length stations == length (nub stations))
    "A station may have only one declared upstream access point" ++ checkSegments n
