module NetDSL.Router.Docs

import NetDSL.Common
import NetDSL.Router.Model
import NetDSL.Router.OptionJSON
import NetDSL.Router.Validate
import NetDSL.Domain.Address
import NetDSL.AAA
import Data.List
import Data.String
import Data.Maybe

%default total

private
obj : List (String,String) -> String
obj entries = "{" ++ join "," (map (\(k,v) => jsonString k ++ ":" ++ v) entries) ++ "}"

private
entity : Nat -> String -> SourceSpan -> List (String,String)
entity i name at = [("id",show i),("name",jsonString name),("source",spanJSON at)]

private
attachmentJSON : Attachment -> String
attachmentJSON (Physical r) = obj [("kind",jsonString "physical"),("ref",show r.index)]
attachmentJSON (BridgeDevice r) = obj [("kind",jsonString "bridge"),("ref",show r.index)]
attachmentJSON (BondDevice r) = obj [("kind",jsonString "bond"),("ref",show r.index)]
attachmentJSON Loopback = obj [("kind",jsonString "loopback")]

private
interfaceJSON : (Nat,LogicalInterface) -> String
interfaceJSON (i,f) = obj (entity i f.name f.source ++
  [("attachment",attachmentJSON f.attachment.value),
   ("addresses",jsonArray (map (\a => jsonString (showIPv4 a.value.address ++ "/" ++ show a.value.subnet.width)) f.addresses)),
   ("addresses6",jsonArray (map (\a => jsonString (showIPv6 a.value.address ++ "/" ++ show a.value.width)) f.addresses6)),
   ("settings",jsonInterfaceOptions f.settings),
   ("dynamicAddressState",jsonString (if elem f.settings.protocol [Just DHCPClient,Just DHCPv6Client] then "Unknown" else "NotApplicable"))])

private
poolJSON : DHCPServer -> String
poolJSON d = case d.pool of
  Nothing => "null"
  Just (a,b) => obj [("first",jsonString (showIPv4 a.value)),("last",jsonString (showIPv4 b.value)),("count",show (leaseCount d))]

private
ruleJSON : (Nat,FirewallRule) -> String
ruleJSON (i,r) = obj (entity i r.name r.source ++
  [("sourceZoneRef",show r.from.value.index),
   ("destination",case r.destination.value of
     LocalInput => obj [("kind",jsonString "local")]
     AnyZone => obj [("kind",jsonString "any-zone")]
     NamedZone z => obj [("kind",jsonString "zone"),("ref",show z.index)]),
   ("protocols",jsonArray (map (jsonString . showIPProtocol) r.protocols)),
   ("sourcePrefix",case r.sourcePrefix of Nothing => "null"; Just (Source4 p) => jsonString (showPrefix p); Just (Source6 p) => jsonString (showPrefix6 p)),
   ("icmpTypes",jsonArray (map jsonString r.icmpTypes)),("settings",jsonRuleOptions r.settings)])

public export
routerJSON : RouterConfig -> String
routerJSON c = obj
  [("physicals",jsonArray (map (\(i,p) => obj (entity i p.name p.source)) (indexed c.physicals))),
   ("bridges",jsonArray (map (\(i,b) => obj (entity i b.name b.source ++ [("members",jsonArray (map (attachmentJSON . value) b.members))])) (indexed c.bridges))),
   ("bonds",jsonArray (map (\(i,b) => obj (entity i b.name b.source ++ [("physicalRefs",jsonArray (map (show . index . value) b.members)),("settings",jsonBondOptions b.settings)])) (indexed c.bonds))),
   ("interfaces",jsonArray (map interfaceJSON (indexed c.interfaces))),
   ("globals",maybe "null" (jsonGlobalOptions . value) c.globals),
   ("dns",maybe "null" (jsonDNSOptions . value) c.dns),
   ("dhcpServers",jsonArray (map (\(i,d) => obj (entity i d.name d.source ++ [("interfaceRef",show d.ifaceRef.value.index),("pool",poolJSON d),("raFlags",jsonArray (map (jsonString . showRAFlag) d.flags)),("settings",jsonDHCPOptions d.settings)])) (indexed c.dhcpServers))),
   ("odhcp",maybe "null" (jsonODHCPOptions . value) c.odhcp),
   ("firewallDefaults",jsonFirewallDefaults c.defaults.value),
   ("zones",jsonArray (map (\(i,z) => obj (entity i z.name z.source ++ [("interfaceRefs",jsonArray (map (show . index . value) z.interfaces)),("settings",jsonZoneOptions z.settings)])) (indexed c.zones))),
   ("forwardings",jsonArray (map (\f => obj [("sourceZoneRef",show f.from.value.index),("destinationZoneRef",show f.destination.value.index),("source",spanJSON f.source)]) c.forwardings)),
   ("rules",jsonArray (map ruleJSON (indexed c.rules))),
   ("radios",jsonArray (map (\(i,r) => obj (entity i r.name r.source ++ [("settings",jsonRadioOptions r.settings)])) (indexed c.radios))),
   ("accessPoints",jsonArray (map (\(i,a) => obj (entity i a.name a.source ++ [("radioRef",show a.radio.value.index),("interfaceRef",show a.ifaceRef.value.index),("credentialRef",maybe "null" (jsonString . secretURI) a.credential),("settings",jsonAPOptions a.settings)])) (indexed c.accessPoints))),
   ("attachmentEdges",jsonArray (map (\(a,b) => jsonArray [show a,show b]) (linkEdges c)))]

private
escape : String -> String
escape s = concatMap (\c => case c of '&' => "&amp;"; '"' => "&quot;"; '<' => "&lt;"; '>' => "&gt;"; '|' => "&#124;"; _ => singleton c) (unpack s)

private
table : List String -> List (List String) -> String
table headings rows = "| " ++ join " | " headings ++ " |\n| " ++ join " | " (map (const "---") headings) ++ " |\n" ++
  concatMap (\r => "| " ++ join " | " (map escape r) ++ " |\n") rows ++ "\n"

public export
routerGraph : String -> RouterConfig -> String
routerGraph nodePrefix c =
  concatMap (\(i,p) => node (linkId i) p.name) (indexed c.physicals) ++
  concatMap (\(i,b) => node (linkId (length c.physicals+i)) (b.name ++ " bridge")) (indexed c.bridges) ++
  concatMap (\(i,b) => node (linkId (length c.physicals+length c.bridges+i)) (b.name ++ " bond")) (indexed c.bonds) ++
  node (linkId (attachmentId c Loopback)) "lo" ++
  concatMap (\(a,b) => edge (linkId a) (linkId b)) (linkEdges c) ++
  concatMap (\(i,f) => node (ifaceId i) (f.name ++ " / " ++ maybe "?" showInterfaceProtocol f.settings.protocol) ++ edge (linkId (attachmentId c f.attachment.value)) (ifaceId i)) (indexed c.interfaces) ++
  concatMap (\(i,r) => node (nodePrefix ++ "radio" ++ show i) r.name) (indexed c.radios) ++
  concatMap (\(i,a) => node (nodePrefix ++ "ap" ++ show i) (a.name ++ " / " ++ fromMaybe "" a.settings.ssid ++ if a.settings.disabled == Just True then " (disabled)" else "") ++
    edge (nodePrefix ++ "radio" ++ show a.radio.value.index) (nodePrefix ++ "ap" ++ show i) ++ edge (nodePrefix ++ "ap" ++ show i) (ifaceId a.ifaceRef.value.index)) (indexed c.accessPoints)
  where
    linkId : Nat -> String
    linkId i = nodePrefix ++ "link" ++ show i
    ifaceId : Nat -> String
    ifaceId i = nodePrefix ++ "iface" ++ show i
    node : String -> String -> String
    node id label = "  " ++ id ++ "[\"" ++ escape label ++ "\"]\n"
    edge : String -> String -> String
    edge a b = "  " ++ a ++ " --> " ++ b ++ "\n"

public export
routerMarkdown : String -> RouterConfig -> String
routerMarkdown routerName c =
  "## Router " ++ routerName ++ "\n\nExplicit dual-stack interface and zone intent. Dynamic addresses, delegated prefixes, link state, and radio operation are **Unknown**.\n\n" ++
  table ["Link","Kind","Members"]
    (map (\b => [b.name,"bridge",join ", " (map (attachmentName c . value) b.members)]) c.bridges ++
     map (\b => [b.name,"bond",join ", " (map (\m => maybe "invalid" name (lookupAt m.value.index c.physicals)) b.members)]) c.bonds) ++
  table ["Interface","Attachment","Protocol","IPv4","IPv6 assignment"]
    (map (\i => [i.name,attachmentName c i.attachment.value,maybe "?" showInterfaceProtocol i.settings.protocol,
      join ", " (map (\a => showIPv4 a.value.address ++ "/" ++ show a.value.subnet.width) i.addresses),maybe "none" (\n => "/" ++ show n) i.settings.ipv6Assignment]) c.interfaces) ++
  table ["DHCP interface","Pool","IPv6 server","Router advertisements"]
    (map (\d => [maybe "invalid" name (lookupAt d.ifaceRef.value.index c.interfaces),case d.pool of Nothing => "none"; Just (a,b) => showIPv4 a.value ++ " .. " ++ showIPv4 b.value,
      maybe "default" showServerMode d.settings.ipv6,maybe "default" showServerMode d.settings.ra]) c.dhcpServers) ++
  table ["Zone","Interfaces","Input","Output","Forward","IPv4 NAT"]
    (map (\z => [z.name,join ", " (map (\i => maybe "invalid" name (lookupAt i.value.index c.interfaces)) z.interfaces),maybe "?" showVerdict z.settings.input,maybe "?" showVerdict z.settings.output,maybe "?" showVerdict z.settings.forward,if z.settings.masquerade == Just True then "yes" else "no"]) c.zones) ++
  "Firewall rules retain declaration order.\n\n" ++ table ["Rule","From","To","Protocols","Action"]
    (map (\r => [r.name,maybe "invalid" name (lookupAt r.from.value.index c.zones),case r.destination.value of LocalInput => "router"; AnyZone => "any zone"; NamedZone z => maybe "invalid" name (lookupAt z.index c.zones),join ", " (map showIPProtocol r.protocols),maybe "?" showVerdict r.settings.action]) c.rules) ++
  table ["AP","SSID","Radio","Interface","Security","Enabled","Credential reference"]
    (map (\a => [a.name,fromMaybe "" a.settings.ssid,maybe "invalid" name (lookupAt a.radio.value.index c.radios),maybe "invalid" name (lookupAt a.ifaceRef.value.index c.interfaces),maybe "?" showSecurity a.settings.security,if a.settings.disabled == Just True then "no" else "yes",maybe "none" secretURI a.credential]) c.accessPoints) ++
  "Secret references identify externally held credentials. Wireless templates require binding before installation; the compiler never resolves credentials.\n\n"
