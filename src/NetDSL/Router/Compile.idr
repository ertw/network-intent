module NetDSL.Router.Compile

import NetDSL.Common
import NetDSL.Router.Model
import NetDSL.Router.OptionFields
import NetDSL.Router.Validate
import NetDSL.Backend.AST
import NetDSL.Domain.Address
import NetDSL.AAA
import Data.List
import Data.Maybe

%default total

private
ifaceName : RouterConfig -> RouterRef InterfaceEntity -> String
ifaceName c r = maybe "invalid" name (lookupAt r.index c.interfaces)

private
zoneName : RouterConfig -> RouterRef ZoneEntity -> String
zoneName c r = maybe "invalid" name (lookupAt r.index c.zones)

private
networkPackage : RouterConfig -> Either Diagnostic UciPackage
networkPackage c = do
  globals <- traverse (\g => section g.span ["network globals"] "globals" "globals" (fieldsGlobalOptions g.value) []) (maybe [] pure c.globals)
  bridges <- traverse (\(i,b) => section b.source ["bridge " ++ b.name] "device" ("bridge_" ++ show i)
    [("name",b.name),("type","bridge")] (map (\m => ("ports",attachmentName c m.value)) b.members)) (indexed c.bridges)
  bonds <- traverse (\(i,b) => section b.source ["bond " ++ b.name] "device" ("bond_" ++ show i)
    ([("name",b.name),("type","bonding")] ++ fieldsBondOptions b.settings)
    (map (\m => ("ports",maybe "invalid" name (lookupAt m.value.index c.physicals))) b.members)) (indexed c.bonds)
  interfaces <- traverse iface c.interfaces
  Right (Package "network" (globals ++ bridges ++ bonds ++ interfaces))
  where
    iface : LogicalInterface -> Either Diagnostic UciSection
    iface i = section i.source ["logical interface " ++ i.name] "interface" i.name
      (("device",attachmentName c i.attachment.value) :: map (\(k,v) => if k == "multipath" then (k,if v == "1" then "on" else "off") else (k,v)) (fieldsInterfaceOptions i.settings))
      (map (\a => ("ipaddr",showIPv4 a.value.address ++ "/" ++ show a.value.subnet.width)) i.addresses ++
       map (\a => ("ip6addr",showIPv6 a.value.address ++ "/" ++ show a.value.width)) i.addresses6)

private
dhcpPackage : RouterConfig -> Either Diagnostic UciPackage
dhcpPackage c = do
  dns <- traverse (\d => section d.span ["DNS resolver settings"] "dnsmasq" "dns" (fieldsDNSOptions d.value ++ if isNothing d.value.leaseMax && sum (map leaseCount c.dhcpServers) > 150 then [("dhcpleasemax",show (sum (map leaseCount c.dhcpServers)))] else []) []) (maybe [] pure c.dns)
  pools <- traverse dhcp c.dhcpServers
  odhcp <- traverse (\d => section d.span ["IPv6 DHCP daemon"] "odhcpd" "odhcpd" (fieldsODHCPOptions d.value) []) (maybe [] pure c.odhcp)
  Right (Package "dhcp" (dns ++ pools ++ odhcp))
  where
    dhcp : DHCPServer -> Either Diagnostic UciSection
    dhcp d = do
      range <- case d.pool of
        Nothing => Right []
        Just (a,b) => case lookupAt d.ifaceRef.value.index c.interfaces >>= head' . addresses of
          Nothing => Left (failure "backend.invalid-reference" d.source "DHCP static subnet missing from certified model")
          Just addr => Right [("start",show (a.value.number-addr.value.subnet.address.number)),("limit",show (b.value.number-a.value.number+1))]
      section d.source ["DHCP and RA on " ++ ifaceName c d.ifaceRef.value] "dhcp" d.name
        ([("interface",ifaceName c d.ifaceRef.value)] ++ range ++ fieldsDHCPOptions d.settings)
        (map (\f => ("ra_flags",showRAFlag f)) d.flags)

private
firewallPackage : RouterConfig -> Either Diagnostic UciPackage
firewallPackage c = do
  defaults <- section c.defaults.span ["explicit firewall defaults"] "defaults" "defaults" (fieldsFirewallDefaults c.defaults.value) []
  zones <- traverse (\(i,z) => section z.source ["zone " ++ z.name] "zone" ("zone_" ++ show i)
    (("name",z.name) :: fieldsZoneOptions z.settings) (map (\i => ("network",ifaceName c i.value)) z.interfaces)) (indexed c.zones)
  forwards <- traverse (\(i,f) => section f.source ["zone forwarding"] "forwarding" ("forward_" ++ show i)
    [("src",zoneName c f.from.value),("dest",zoneName c f.destination.value)] []) (indexed c.forwardings)
  rules <- traverse rule (indexed c.rules)
  Right (Package "firewall" (defaults :: zones ++ forwards ++ rules))
  where
    rule : (Nat,FirewallRule) -> Either Diagnostic UciSection
    rule (i,r) = section r.source ["ordered firewall rule " ++ r.name] "rule" ("rule_" ++ show i)
      ([("name",r.name),("src",zoneName c r.from.value),("proto",join " " (map showIPProtocol r.protocols))] ++
       (case r.destination.value of LocalInput => []; AnyZone => [("dest","*")]; NamedZone z => [("dest",zoneName c z)]) ++
       (case r.sourcePrefix of Nothing => []; Just (Source4 p) => [("src_ip",showPrefix p)]; Just (Source6 p) => [("src_ip",showPrefix6 p)]) ++
       fieldsRuleOptions r.settings) (map (\t => ("icmp_type",t)) r.icmpTypes)

private
wirelessPackage : RouterConfig -> Either Diagnostic UciPackage
wirelessPackage c = do
  radios <- traverse (\r => section r.source ["radio " ++ r.name] "wifi-device" r.name (fieldsRadioOptions r.settings) []) c.radios
  aps <- traverse ap c.accessPoints
  Right (Package "wireless" (radios ++ aps))
  where
    ap : AccessPoint -> Either Diagnostic UciSection
    ap a = do
      s <- section a.source ["access point " ++ a.name] "wifi-iface" a.name
        ([("device",maybe "invalid" name (lookupAt a.radio.value.index c.radios)),("network",ifaceName c a.ifaceRef.value)] ++ fieldsAPOptions a.settings) []
      Right ({ fields := s.fields ++ maybe [] (\r => [SecretOption "key" r (fromMaybe Open a.settings.security)]) a.credential } s)

public export
compileRouter : RouterConfig -> Either Diagnostic TargetAST
compileRouter c = do
  network <- networkPackage c
  dhcp <- dhcpPackage c
  firewall <- firewallPackage c
  wireless <- wirelessPackage c
  Right (UCI ([network,dhcp,firewall] ++ if null wireless.sections then [] else [wireless]))
