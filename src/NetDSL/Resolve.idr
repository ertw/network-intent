module NetDSL.Resolve

import NetDSL.Common
import NetDSL.Syntax.Parser
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Router.Parse
import NetDSL.Router.Address
import Data.List
import Data.String

%default total

public export
named : Statement -> String
named s = case stmtWords s of
  _ :: n :: _ => n
  _ => ""

private
err : String -> Statement -> String -> Either Diagnostic a
err code s msg = Left (failure code (stmtSpan s) msg)

private
identifier : String -> Bool
identifier s = s /= "" && length s <= 63 && all (\c => isAlphaNum c && ord c < 128 || c == '-' || c == '_') (unpack s)

private
checkName : Statement -> String -> Either Diagnostic ()
checkName st n = if identifier n then Right () else err "name.invalid" st "Names must contain 1..63 ASCII letters, digits, hyphens or underscores"

private
allowed : List String -> Statement -> Either Diagnostic ()
allowed keys s = traverse_ check (children s)
  where
    check : Statement -> Either Diagnostic ()
    check c = if elem (keyword c) keys then Right () else err "syntax.unknown-construct" c ("Unsupported construct: " ++ keyword c)

private
field : String -> Statement -> Either Diagnostic (Maybe Token)
field key st = case filter ((== key) . keyword) (children st) of
  [] => Right Nothing
  [s] => case (stmtTokens s, hasBlock s) of
    ([_,t],False) => Right (Just t)
    _ => err "syntax.field" s ("Expected '" ++ key ++ " VALUE'")
  a :: b :: _ => Left (MkDiagnostic "name.duplicate-field" (stmtSpan b) ("Duplicate field: " ++ key) "" [(stmtSpan a,"First field")])

private
required : String -> Statement -> Either Diagnostic Token
required key st = do
  t <- field key st
  case t of
    Nothing => err "syntax.missing-field" st ("Missing required field: " ++ key)
    Just x => Right x

private
repeatFields : String -> Statement -> Either Diagnostic (List Token)
repeatFields key st = concat <$> traverse values (filter ((==key) . keyword) (children st))
  where
    values : Statement -> Either Diagnostic (List Token)
    values s = case (stmtTokens s,hasBlock s) of
      (_ :: x :: xs,False) => Right (x :: xs)
      _ => err "syntax.field" s ("Expected " ++ key ++ " followed by references")

private
duplicates : String -> List (String, SourceSpan) -> Either Diagnostic ()
duplicates kind [] = Right ()
duplicates kind ((n,s) :: xs) = case find ((==n) . fst) xs of
  Just (_,at) => Left (MkDiagnostic "name.duplicate" at ("Duplicate " ++ kind ++ ": " ++ n) "" [(s,"First declaration")])
  Nothing => duplicates kind xs

private
resolveName : {refKind : EntityKind} -> String -> List String -> Token -> Either Diagnostic (Ref refKind)
resolveName kind names t = case find ((==t.text) . snd) (indexed names) of
  Just (i,_) => Right (Id i)
  Nothing => Left (failure ("reference.unknown-" ++ kind) t.source ("Unknown " ++ kind ++ ": " ++ t.text))

private
parseHost : IPv4Prefix -> Statement -> Either Diagnostic HostAssignment
parseHost subnet st = case (stmtTokens st,hasBlock st) of
  ([_,n,a],False) => do
    checkName st n.text
    ip <- assignment subnet a.source a.text
    Right (HostAt n.text (At a.source ip) (stmtSpan st))
  _ => err "syntax.host" st "Expected host NAME ADDRESS"

private
parseVlan : Statement -> Either Diagnostic Vlan
parseVlan st = case (stmtTokens st,hasBlock st) of
  ([_,n,id],True) => do
    checkName st n.text
    if elem n.text ["internet","gateway"] then err "name.reserved" st "VLAN name is reserved for a policy endpoint" else Right ()
    allowed ["subnet","gateway","dhcp","host"] st
    vid <- mkVlanId id.source id.text
    sub <- required "subnet" st
    subnet <- parsePrefix sub.source sub.text
    gw <- field "gateway" st
    gateway <- traverse (\t => At t.source <$> assignment subnet t.source t.text) gw
    hosts <- traverse (parseHost subnet) (filter ((=="host") . keyword) (children st))
    pool <- case filter ((=="dhcp") . keyword) (children st) of
      [] => Right Nothing
      [s] => do
        (first,last) <- poolEndpoints subnet s
        Right (Just (Pool first last))
      _ => err "name.duplicate-field" st "Only one DHCP range is supported per VLAN"
    case (pool,gateway) of
      (Just _,Nothing) => err "address.dhcp-without-gateway" st "A DHCP VLAN requires a gateway"
      _ => Right ()
    Right (MkVlan n.text vid (At sub.source subnet) gateway pool hosts (stmtSpan st))
  _ => err "syntax.vlan" st "Expected vlan NAME ID { ... }"

private
parseDriver : Token -> Either Diagnostic Driver
parseDriver t = case t.text of
  "openwrt" => Right OpenWrt
  "cisco-ios" => Right CiscoIOS
  _ => Left (failure "backend.unknown-driver" t.source ("Unsupported driver: " ++ t.text))

private
portStatements : Statement -> List Statement
portStatements = filter ((=="port") . keyword) . children

private
resolvePeer : List Statement -> List String -> Token -> Either Diagnostic LinkEnd
resolvePeer devices hosts t = case splitOn '.' t.text of
  [device,port] => case find ((==device) . named . snd) (indexed devices) of
    Just (i,d) => case find ((==port) . named . snd) (indexed (portStatements d)) of
      Just (j,_) => Right (DevicePort (PortId (Id i) j))
      Nothing => Left (failure "topology.unknown-port" t.source ("Unknown device port: " ++ t.text))
    Nothing => case find ((==device) . snd) (indexed hosts) of
      Just (i,_) => if port == "eth0" then Right (HostPort (Id i)) else Left (failure "topology.unknown-port" t.source "Static hosts expose one implicit interface named eth0 in language 3.0")
      Nothing => Left (failure "topology.unknown-endpoint" t.source ("Unknown link endpoint: " ++ t.text))
  _ => Left (failure "topology.invalid-endpoint" t.source "Expected device.port or host.eth0")

private
singleLine : String -> Bool
singleLine s = all (\c => ord c >= 32 && ord c /= 127 && ord c /= 8232 && ord c /= 8233) (unpack s)

private
parsePort : List Vlan -> List Statement -> List String -> Ref DeviceKind -> Statement -> Either Diagnostic Port
parsePort vlans devices hosts owner st = case (stmtTokens st,hasBlock st) of
  ([_,n],True) => do
    if n.text /= "" && length n.text <= 63 && all (\c => ord c < 128 && (isAlphaNum c || elem c ['/', '.', '-', '_'])) (unpack n.text)
      then Right () else err "name.invalid-interface" st "Invalid physical interface label"
    allowed ["description","access","trunk","connect"] st
    desc <- field "description" st
    case desc of
      Just d => if singleLine d.text then Right () else Left (failure "backend.unsafe-value" d.source "Descriptions cannot contain control characters or line separators")
      Nothing => Right ()
    mode <- case filter (\s => elem (keyword s) ["access","trunk"]) (children st) of
      [s] => case (stmtTokens s,hasBlock s) of
        ([a,v],False) => if a.text == "access" then Access <$> resolveName "vlan" (map name vlans) v
                          else Trunk . pure <$> resolveName "vlan" (map name vlans) v
        (a :: v :: vs,False) => if a.text == "trunk" then do
          refs <- traverse (resolveName "vlan" (map name vlans)) (v :: vs)
          if length (nub refs) == length refs then Right (Trunk refs) else err "topology.duplicate-membership" s "A VLAN may occur only once in a trunk"
          else err "topology.port-mode" s "Access ports carry exactly one VLAN"
        _ => err "topology.port-mode" s "Expected access VLAN or trunk VLAN..."
      _ => err "topology.port-mode" st "A port must have exactly one access or trunk declaration"
    peerToken <- field "connect" st
    peer <- traverse (\t => At t.source <$> resolvePeer devices hosts t) peerToken
    Right (MkPort owner n.text (map text desc) mode peer (stmtSpan st))
  _ => err "syntax.port" st "Expected port NAME { ... }"

private
parseDevice : List Vlan -> List Statement -> List String -> (Nat,Statement) -> Either Diagnostic Device
parseDevice vlans devices hosts (i,st) = case (stmtTokens st,hasBlock st) of
  ([_,n],True) => do
    checkName st n.text
    allowed ["driver","port","max-tagged-vlans","routing"] st
    driver <- required "driver" st >>= parseDriver
    cap <- field "max-tagged-vlans" st
    limit <- case cap of
      Nothing => Right 4094
      Just t => case decimal t.text of
        Just x => if x <= 4094 then Right (cast x) else err "backend.invalid-capability" st "Tagged VLAN budget must be in 0..4094"
        Nothing => err "backend.invalid-capability" st "Tagged VLAN budget must be an integer"
    duplicates "port" (map (\p => (named p,stmtSpan p)) (portStatements st))
    ports <- traverse (parsePort vlans devices hosts (Id i)) (portStatements st)
    routing <- parseRouting st
    Right (MkDevice n.text (keyword st == "router") driver ports limit routing (stmtSpan st))
  _ => err "syntax.device" st "Expected router/switch/device NAME { ... }"

private
builtins : SourceSpan -> List Service
builtins at = [MkService "dns" [(UDP,53),(TCP,53)] [] at, MkService "ntp" [(UDP,123)] [] at,
               MkService "http" [(TCP,80)] [] at, MkService "https" [(TCP,443)] [] at, MkService "ssh" [(TCP,22)] [] at]

private
parseService : List String -> Statement -> Either Diagnostic Service
parseService names st = case (stmtTokens st,hasBlock st) of
  ([_,n],True) => do
    checkName st n.text
    allowed ["tcp","udp","depends"] st
    ts <- concat <$> traverse ports (filter (\s => elem (keyword s) ["tcp","udp"]) (children st))
    if null ts then err "service.empty" st "A service must declare at least one TCP or UDP port" else Right ()
    deps <- repeatFields "depends" st >>= traverse (\t => At t.source <$> resolveName "service" names t)
    Right (MkService n.text (nub ts) deps (stmtSpan st))
  _ => err "syntax.service" st "Expected service NAME { tcp PORT; udp PORT } (one field per line)"
  where
    ports : Statement -> Either Diagnostic (List (Transport,Integer))
    ports s = case (stmtTokens s,hasBlock s) of
      ([k,t],False) => case decimal t.text of
        Just n => if n >= 1 && n <= 65535 then Right [(if k.text == "tcp" then TCP else UDP,n)] else err "service.invalid-port" s "Transport port must be in 1..65535"
        Nothing => err "service.invalid-port" s "Expected an integer transport port"
      _ => err "syntax.service-port" s "Expected tcp PORT or udp PORT"

private
parsePolicy : List Vlan -> List Device -> List Service -> Maybe (Ref DeviceKind) -> Statement -> Either Diagnostic Policy
parsePolicy vlans devices services enforcer st = case (stmtTokens st,hasBlock st) of
  (a :: arrow :: b :: act :: rest,False) => do
    if arrow.text == "->" then Right () else err "syntax.policy" st "Expected SOURCE -> DESTINATION allow/deny [SERVICE,...]"
    src <- resolveName "vlan" (map name vlans) a
    dst <- if b.text == "internet" then Right Internet
           else if b.text == "gateway" then case enforcer of
             Just r => Right (Gateway r)
             Nothing => err "policy.no-enforcer" st "Gateway policy requires exactly one router"
           else Zone <$> resolveName "vlan" (map name vlans) b
    action <- case act.text of
      "allow" => Right Allow
      "deny" => Right Deny
      _ => err "policy.invalid-action" st "Policy action must be allow or deny"
    serviceTokens <- case rest of
      [] => Right []
      first :: _ => let parts = map trim (splitOn ',' (join " " (map text rest))) in
        if all identifier parts then Right (map (\s => Tok Word s first.source) parts)
          else err "policy.invalid-service-list" st "Expected comma-separated service names; empty service entries are not permitted"
    refs <- traverse (resolveName "service" (map name services)) serviceTokens
    if src == (case dst of Zone v => v; _ => Id (S (length vlans))) then err "policy.same-zone" st "Inter-zone policy cannot govern traffic within one VLAN" else Right ()
    Right (Rule src dst action (nub refs) (stmtSpan st))
  _ => err "syntax.policy" st "Expected SOURCE -> DESTINATION allow/deny [SERVICE,...]"

private
parseRoute : List Vlan -> List Device -> List String -> Statement -> Either Diagnostic Route
parseRoute vlans devices names st = case (stmtTokens st,hasBlock st) of
  ([_,n],True) => do
    checkName st n.text
    allowed ["destination","via","vlan","device","metric","depends"] st
    d <- required "destination" st
    dst <- parsePrefix d.source d.text
    h <- required "via" st
    hop <- parseIPv4 h.source h.text
    v <- required "vlan" st >>= resolveName "vlan" (map name vlans)
    target <- required "device" st >>= resolveName "device" (map name devices)
    m <- field "metric" st
    metric <- case m of
      Nothing => Right 0
      Just t => case decimal t.text of
        Just x => if x <= 4294967295 then Right x else err "routing.invalid-metric" st "Route metric exceeds 32-bit range"
        Nothing => err "routing.invalid-metric" st "Route metric must be a nonnegative integer"
    deps <- repeatFields "depends" st >>= traverse (\t => At t.source <$> resolveName "route" names t)
    Right (MkRoute n.text (At d.source dst) (At h.source hop) v target metric deps (stmtSpan st))
  _ => err "syntax.route" st "Expected route NAME { ... }"

private
resolveWiFi : List Device -> Token -> Either Diagnostic WiFiRef
resolveWiFi devices t = case splitOn '.' t.text of
  [owner,iface] => case find (\(_,d) => (the Device d).name == owner) (indexed devices) of
    Nothing => Left (failure "reference.unknown-wireless-device" t.source "Unknown wireless device")
    Just (i,d) => case d.routing of
      Nothing => Left (failure "reference.unknown-wireless-interface" t.source "Device has no explicit Wi-Fi configuration")
      Just c => case find (\(_,a) => (the WiFiInterface a).name == iface) (indexed c.wifiInterfaces) of
        Nothing => Left (failure "reference.unknown-wireless-interface" t.source "Unknown wireless interface")
        Just (j,_) => Right (WiFiId (Id i) (RRef j))
  _ => Left (failure "syntax.wireless-reference" t.source "Expected DEVICE.WIFI_INTERFACE")

private
parseWirelessLink : List Device -> Statement -> Either Diagnostic WirelessLink
parseWirelessLink devices s = case (stmtTokens s,hasBlock s) of
  ([_,a,arrow,b],False) => if arrow.text /= "->" then bad else do
    station <- resolveWiFi devices a
    ap <- resolveWiFi devices b
    Right (MkWirelessLink (At a.source station) (At b.source ap) (stmtSpan s))
  _ => bad
  where
    bad : Either Diagnostic WirelessLink
    bad = err "syntax.wireless-link" s "Expected wireless-link STATION -> ACCESS_POINT"

public export
resolve : Document -> Either (List Diagnostic) (Network V3)
resolve doc = either (Left . pure) Right (run doc.statements)
  where
    run : List Statement -> Either Diagnostic (Network V3)
    run [version,network] = do
      if stmtWords version == ["network-language","3.0"] && not (hasBlock version) then Right ()
        else err "version.unsupported" version "Every document must begin with network-language 3.0; other versions are not reinterpreted"
      case (stmtWords network,hasBlock network) of
        (["network",n],True) => checkName network n
        _ => err "syntax.network" network "Expected network NAME { ... }"
      allowed ["domain","vlan","router","switch","device","service","policy","route","wireless-link"] network
      domain <- field "domain" network
      case domain of
        Just t => if t.text /= "" && length t.text <= 253 && all (\c => ord c < 128 && (isAlphaNum c || c == '-' || c == '.')) (unpack t.text)
                    then Right () else err "name.invalid-domain" network "Invalid DNS domain"
        Nothing => Right ()
      let vlanStmts = filter ((=="vlan") . keyword) (children network)
      let deviceStmts = filter (\s => elem (keyword s) ["router","switch","device"]) (children network)
      let serviceStmts = filter ((=="service") . keyword) (children network)
      let routeStmts = filter ((=="route") . keyword) (children network)
      duplicates "VLAN" (map (\s => (named s,stmtSpan s)) vlanStmts)
      duplicates "device" (map (\s => (named s,stmtSpan s)) deviceStmts)
      duplicates "route" (map (\s => (named s,stmtSpan s)) routeStmts)
      vlans <- traverse parseVlan vlanStmts
      duplicates "VLAN ID" (map (\v => (show v.vid.number,v.source)) vlans)
      let hosts = concatMap hosts vlans
      duplicates "host" (map (\h => (h.name,h.source)) hosts)
      duplicates "physical endpoint" (map (\h => (h.name,h.source)) hosts ++ map (\s => (named s,stmtSpan s)) deviceStmts)
      devices <- traverse (parseDevice vlans deviceStmts (map name hosts)) (indexed deviceStmts)
      let builtin = builtins (stmtSpan network)
      duplicates "service" (map (\s => (s.name,s.source)) builtin ++ map (\s => (named s,stmtSpan s)) serviceStmts)
      custom <- traverse (parseService (map name builtin ++ map named serviceStmts)) serviceStmts
      let services = builtin ++ custom
      let routers = filter (isRouter . snd) (indexed devices)
      enforcer <- case routers of
        [] => Right Nothing
        [(i,_)] => Right (Just (Id i))
        _ => err "policy.ambiguous-enforcer" network "Language 3.0 supports one routing/enforcement owner; multiple routers require explicit future ownership semantics"
      let blocks = filter ((=="policy") . keyword) (children network)
      traverse_ (\s => if stmtWords s == ["policy"] && hasBlock s then Right () else err "syntax.policy" s "Expected policy { ... }") blocks
      policies <- traverse (parsePolicy vlans devices services enforcer) (concatMap children blocks)
      routes <- traverse (parseRoute vlans devices (map named routeStmts)) routeStmts
      links <- traverse (parseWirelessLink devices) (filter ((=="wireless-link") . keyword) (children network))
      Right (MkNetwork (named network) (map text domain) vlans devices services routes policies enforcer links (stmtSpan network))
    run [] = Left (failure "version.missing" (MkSpan doc.sourceFile 1 1 1 1) "Missing network-language 3.0 header")
    run (s :: _) = err "version.document-shape" s "Expected exactly a network-language 3.0 header and one network block"
