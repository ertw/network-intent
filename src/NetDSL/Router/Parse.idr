module NetDSL.Router.Parse

import NetDSL.Common
import NetDSL.Syntax.Parser
import NetDSL.Router.Model
import NetDSL.Router.Fields
import NetDSL.Router.ParseOptions
import NetDSL.AAA
import NetDSL.Domain.Address
import Data.List
import Data.Maybe

%default total

private
statements : String -> Statement -> List Statement
statements key = filter ((==key) . keyword) . children

private
ref : {k : RouterEntity} -> List String -> Token -> Either Diagnostic (RouterRef k)
ref names t = case find ((==t.text) . snd) (indexed names) of
  Just (i,_) => Right (RRef i)
  Nothing => Left (failure "reference.unknown-router-entity" t.source "Unknown reference in typed router configuration")

private
refs : {k : RouterEntity} -> List String -> String -> Statement -> Either Diagnostic (List (Located (RouterRef k)))
refs names key s = values key s >>= traverse (\t => At t.source <$> ref names t)

private
attachment : List String -> List String -> List String -> Token -> Either Diagnostic Attachment
attachment physicals bridges bonds t = if t.text == "none" then Right Unattached else if t.text == "lo" then Right Loopback else
  case find ((==t.text) . snd) (indexed physicals) of
    Just (i,_) => Right (Physical (RRef i))
    Nothing => case find ((==t.text) . snd) (indexed bridges) of
      Just (i,_) => Right (BridgeDevice (RRef i))
      Nothing => BondDevice <$> ref bonds t

private
single : String -> Statement -> Either Diagnostic (Maybe Statement)
single key s = case statements key s of
  [] => Right Nothing
  [st] => if stmtWords st == [key] && hasBlock st then Right (Just st) else problem "syntax.block" st ("Expected " ++ key ++ " { ... }")
  _ => problem "name.duplicate-field" s ("Duplicate " ++ key ++ " block")

private
settings : String -> List String -> (Statement -> Either Diagnostic a) -> Statement -> Either Diagnostic (Maybe (Located a))
settings key keys parser s = single key s >>= traverse (\st => do
  fields keys st
  At (stmtSpan st) <$> parser st)

private
parsePhysical : Statement -> Either Diagnostic PhysicalPort
parsePhysical s = case (stmtWords s,hasBlock s) of
  ([_,n],False) => if validName n && n /= "lo" then Right (Ethernet n (stmtSpan s)) else problem "name.invalid" s "Invalid physical port name"
  _ => problem "syntax.physical" s "Expected physical NAME"

private
parseBridge : (Token -> Either Diagnostic Attachment) -> Statement -> Either Diagnostic Bridge
parseBridge attach s = do
  namedBlock s
  fields ["members", "stp"] s
  members <- values "members" s >>= traverse (\t => At t.source <$> attach t)
  stp <- optional boolean "stp" s
  Right (MkBridge (named s) members stp (stmtSpan s))

private
parseBond : List String -> Statement -> Either Diagnostic Bond
parseBond physicals s = do
  namedBlock s
  fields ("members" :: bondOptionsKeys) s
  members <- refs physicals "members" s
  opts <- parseBondOptions s
  Right (MkBond (named s) members opts (stmtSpan s))

private
parseInterface : (Token -> Either Diagnostic Attachment) -> Statement -> Either Diagnostic LogicalInterface
parseInterface attach s = do
  namedBlock s
  fields (["attach","address","address6","gateway","dns"] ++ interfaceOptionsKeys) s
  token <- required "attach" s
  device <- At token.source <$> attach token
  addresses <- values "address" s >>= traverse (\t => At t.source <$> parseAddress4 t)
  addresses6 <- values "address6" s >>= traverse (\t => At t.source <$> parseAddress6 t)
  opts <- parseInterfaceOptions s
  gateway <- optional (\t => At t.source <$> parseIPv4 t.source t.text) "gateway" s
  dns <- values "dns" s >>= traverse (\t => At t.source <$> parseIPv4 t.source t.text)
  Right (MkInterface (named s) device addresses addresses6 gateway dns opts (stmtSpan s))

private
parseDHCP : List LogicalInterface -> Statement -> Either Diagnostic DHCPServer
parseDHCP interfaces s = do
  namedBlock s
  fields (["interface","dhcp","ra-flags"] ++ dhcpOptionsKeys) s
  token <- required "interface" s
  ifaceRef <- ref (map name interfaces) token
  pool <- case statements "dhcp" s of
    [] => Right Nothing
    [pool] => case lookupAt ifaceRef.index interfaces of
      Just iface => case iface.addresses of
        [a] => Just <$> poolEndpoints a.value.subnet pool
        _ => problem "address.dhcp-subnet" pool "DHCP requires exactly one static IPv4 interface subnet"
      Nothing => problem "reference.unknown-router-entity" s "Unknown DHCP interface"
    _ => problem "name.duplicate-field" s "Only one DHCP pool is supported per interface"
  flags <- values "ra-flags" s >>= traverse (choice [("managed-config",ManagedConfig),("other-config",OtherConfig),("none",NoFlags)])
  opts <- parseDHCPOptions s
  Right (MkDHCPServer (named s) (At token.source ifaceRef) pool flags opts (stmtSpan s))

private
parseZone : List String -> Statement -> Either Diagnostic FirewallZone
parseZone interfaces s = do
  namedBlock s
  fields ("interfaces" :: zoneOptionsKeys) s
  members <- refs interfaces "interfaces" s
  opts <- parseZoneOptions s
  Right (MkZone (named s) members opts (stmtSpan s))

private
parseForward : List String -> Statement -> Either Diagnostic Forwarding
parseForward zones s = case (stmtTokens s,hasBlock s) of
  ([_,a,arrow,b],False) => if arrow.text /= "->" then bad else do
    src <- ref zones a
    dst <- ref zones b
    Right (MkForwarding (At a.source src) (At b.source dst) (stmtSpan s))
  _ => bad
  where
    bad : Either Diagnostic a
    bad = problem "syntax.forwarding" s "Expected forward SOURCE -> DESTINATION"

private
parseRule : List String -> Statement -> Either Diagnostic FirewallRule
parseRule zones s = do
  namedBlock s
  fields (["source","destination","protocols","source-prefix","icmp-types"] ++ ruleOptionsKeys) s
  src <- required "source" s
  from <- ref zones src
  dest <- field "destination" s
  destination <- case dest of
    Nothing => Right (At (stmtSpan s) LocalInput)
    Just t => if t.text == "any" then Right (At t.source AnyZone)
              else At t.source . NamedZone <$> ref zones t
  protos <- values "protocols" s >>= traverse (choice [("all",ProtoAll),("tcp",ProtoTCP),("udp",ProtoUDP),("icmp",ProtoICMP),("icmpv6",ProtoICMP6),("igmp",ProtoIGMP),("esp",ProtoESP)])
  parsedPrefix <- optional (\t => if elem ':' (unpack t.text) then Source6 <$> parsePrefix6 t.source t.text else Source4 <$> parsePrefix t.source t.text) "source-prefix" s
  icmpTypes <- values "icmp-types" s >>= traverse safeString
  opts <- parseRuleOptions s
  Right (MkFirewallRule (named s) (At src.source from) destination protos parsedPrefix icmpTypes opts (stmtSpan s))

private
parseRadio : Statement -> Either Diagnostic Radio
parseRadio s = do
  namedBlock s
  fields radioOptionsKeys s
  opts <- parseRadioOptions s
  Right (MkRadio (named s) opts (stmtSpan s))

private
parseWiFi : List String -> List String -> Statement -> Either Diagnostic WiFiInterface
parseWiFi radios interfaces s = do
  namedBlock s
  fields (["radio","interface","credential"] ++ wifiOptionsKeys) s
  r <- required "radio" s
  radio <- ref radios r
  i <- required "interface" s
  iface <- ref interfaces i
  credential <- optional (\t => wifiReference t.source t.text) "credential" s
  opts <- parseWiFiOptions s
  Right (MkWiFiInterface (named s) (At r.source radio) (At i.source iface) credential opts (stmtSpan s))

private
parseConfig : Statement -> Either Diagnostic RouterConfig
parseConfig s = do
  fields ["physical","bridge","bond","interface","globals","dns","dhcp-server","odhcp","firewall","zone","forward","firewall-rule","radio","wireless-interface"] s
  let p = statements "physical" s
  let b = statements "bridge" s
  let l = statements "bond" s
  traverse_ (\kind => uniqueNames (map (\st => (named st,stmtSpan st)) (statements kind s))) ["physical","bridge","bond","interface","dhcp-server","zone","firewall-rule","radio","wireless-interface"]
  uniqueNames (map (\st => (named st,stmtSpan st)) (p ++ b ++ l))
  let attach = attachment (map named p) (map named b) (map named l)
  physicals <- traverse parsePhysical p
  bridges <- traverse (parseBridge attach) b
  bonds <- traverse (parseBond (map named p)) l
  interfaces <- traverse (parseInterface attach) (statements "interface" s)
  globals <- settings "globals" globalOptionsKeys parseGlobalOptions s
  dns <- settings "dns" dnsOptionsKeys parseDNSOptions s
  dhcp <- traverse (parseDHCP interfaces) (statements "dhcp-server" s)
  odhcp <- settings "odhcp" odhcpOptionsKeys parseODHCPOptions s
  fw <- settings "firewall" firewallDefaultsKeys parseFirewallDefaults s
  defaults <- maybe (problem "syntax.missing-field" s "Explicit routing requires firewall defaults") Right fw
  zones <- traverse (parseZone (map name interfaces)) (statements "zone" s)
  forwardings <- traverse (parseForward (map name zones)) (statements "forward" s)
  rules <- traverse (parseRule (map name zones)) (statements "firewall-rule" s)
  radios <- traverse parseRadio (statements "radio" s)
  aps <- traverse (parseWiFi (map name radios) (map name interfaces)) (statements "wireless-interface" s)
  Right (MkRouterConfig physicals bridges bonds interfaces globals dns dhcp odhcp defaults zones forwardings rules radios aps (stmtSpan s))

public export
parseRouting : Statement -> Either Diagnostic (Maybe RouterConfig)
parseRouting s = single "routing" s >>= traverse parseConfig
