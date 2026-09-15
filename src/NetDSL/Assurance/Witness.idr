module NetDSL.Assurance.Witness

import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Domain.Address
import NetDSL.Router.Model
import NetDSL.Router.Validate
import NetDSL.Validate
import NetDSL.Backend.AST
import Data.List
import Data.Maybe

%default total

public export
data FieldForm = Scalar | OrderedList | SectionKind

public export
Eq FieldForm where
  Scalar == Scalar = True
  OrderedList == OrderedList = True
  SectionKind == SectionKind = True
  _ == _ = False

public export
record FieldBinding where
  constructor BindField
  packageName : String
  sectionName : String
  fieldName : String
  form : FieldForm
  expected : List String

public export
Eq FieldBinding where
  a == b = a.packageName == b.packageName && a.sectionName == b.sectionName &&
    a.fieldName == b.fieldName && a.form == b.form && a.expected == b.expected

public export
record AssuranceClaim where
  constructor Obligation
  claimId : String
  kind : String
  target : String
  source : SourceSpan
  bindings : List FieldBinding
  unsupported : Maybe String

private
sameSpan : SourceSpan -> SourceSpan -> Bool
sameSpan a b = a.file == b.file && a.line == b.line && a.column == b.column &&
  a.endLine == b.endLine && a.endColumn == b.endColumn

public export
Eq AssuranceClaim where
  a == b = a.claimId == b.claimId && a.kind == b.kind && a.target == b.target &&
    sameSpan a.source b.source && a.bindings == b.bindings && a.unsupported == b.unsupported

public export
record RealizationWitness where
  constructor Witness
  version : Nat
  target : String
  profile : String
  claims : List AssuranceClaim

private
blocked : String -> String -> SourceSpan -> String -> AssuranceClaim
blocked target label at reason = Obligation (target ++ "/" ++ label) "unsupported" target at [] (Just reason)

-- This projection interprets semantic values independently of target compilation.
-- It does not import Router.Compile or reuse the option-to-UCI mapping functions.
private
routerClaims : String -> RouterConfig -> List AssuranceClaim
routerClaims target c = concatMap bridge (indexed c.bridges) ++ concatMap iface c.interfaces ++
  map (\b => blocked target ("bond/" ++ b.name) b.source "LACP partner/selection assurance is not implemented") c.bonds ++
  map (\a => blocked target ("wireless/" ++ a.name) a.source "Radio and wireless-link coverage requires device observation bindings") c.wifiInterfaces ++
  map (\r => blocked target ("radio/" ++ r.name) r.source "Radio capability and operational coverage is not implemented") c.radios ++
  map (\d => blocked target ("dhcp/" ++ d.name) d.source "DHCP/RA lease behavior needs an independent client contract") c.dhcpServers ++
  maybe [] (\d => [blocked target "dns-resolver" d.span "Resolver settings need an explicitly bound UDP and TCP DNS service contract"]) c.dns ++
  maybe [] (\g => [blocked target "globals" g.span "Global device settings lack complete operational coverage"]) c.globals ++
  maybe [] (\o => [blocked target "odhcp" o.span "odhcpd settings lack complete client-observed coverage"]) c.odhcp ++
  [blocked target "firewall" c.defaults.span "Firewall policy requires symbolic coverage; sampled probe timeout cannot prove isolation"]
  where
    fact : String -> String -> FieldForm -> List String -> FieldBinding
    fact section field form expected = BindField "network" section field form expected

    bridge : (Nat,Bridge) -> List AssuranceClaim
    bridge (i,b) =
      [Obligation (target ++ "/bridge/" ++ b.name) "bridge" target b.source
        [fact ("bridge_" ++ show i) "" SectionKind ["device"],
         fact ("bridge_" ++ show i) "name" Scalar [b.name],
         fact ("bridge_" ++ show i) "type" Scalar ["bridge"],
         fact ("bridge_" ++ show i) "ports" OrderedList (map (\m => attachmentName c m.value) b.members)] Nothing] ++
      maybe [] (\_ => [blocked target ("stp/" ++ b.name) b.source "STP selected topology requires operational coverage"]) b.stp

    iface : LogicalInterface -> List AssuranceClaim
    iface i =
      [Obligation (target ++ "/interface/" ++ i.name) "interface" target i.source
        ([fact i.name "" SectionKind ["interface"],
          fact i.name "proto" Scalar (maybe [] (pure . showInterfaceProtocol) i.settings.protocol)]) Nothing,
       Obligation (target ++ "/attachment/" ++ i.name) "attachment" target i.attachment.span
        [fact i.name "device" Scalar (case i.attachment.value of Unattached => []; a => [attachmentName c a])] Nothing,
       Obligation (target ++ "/addressing/" ++ i.name) "addressing" target i.source
        [fact i.name "ipaddr" OrderedList (map (\a => showIPv4 a.value.address ++ "/" ++ show a.value.subnet.width) i.addresses),
         fact i.name "ip6addr" OrderedList (map (\a => showIPv6 a.value.address ++ "/" ++ show a.value.width) i.addresses6)] Nothing,
       Obligation (target ++ "/gateway/" ++ i.name) "route" target i.source
        [fact i.name "gateway" Scalar (maybe [] (pure . showIPv4 . value) i.gateway)] Nothing,
       Obligation (target ++ "/dns/" ++ i.name) "dns-configuration" target i.source
        [fact i.name "dns" OrderedList (map (showIPv4 . value) i.dnsServers)] Nothing] ++
      (if any id [isJust i.settings.ipv6Assignment, isJust i.settings.multipath, isJust i.settings.requestAddress,
                 isJust i.settings.requestPrefix, isJust i.settings.noRelease] then
        [blocked target ("interface-options/" ++ i.name) i.source "Extended interface settings require additional assurance contracts"] else []) ++
      (if maybe False (\p => p == DHCPClient || p == DHCPv6Client) i.settings.protocol then
        [blocked target ("dynamic-addressing/" ++ i.name) i.source "Dynamic addressing requires lease/prefix observations and expectations"] else [])

public export
deriveClaims : StableNetwork -> String -> Either (List Diagnostic) (String,List AssuranceClaim)
deriveClaims stable target = case find ((== target) . name) stable.model.devices of
  Nothing => Left [failure "assurance.unknown-target" stable.model.source ("Unknown target: " ++ target)]
  Just d => case (d.driver,d.routing) of
    (OpenWrt,Just c) => Right ("openwrt-device-fw4-dualstack-v3",routerClaims target c ++
      map (\r => blocked target ("route/" ++ r.name) r.source "Static route operational selection coverage is not implemented") (filter (\r => maybe False ((== target) . name) (getDevice stable.model r.device)) stable.model.routes) ++
      (if null stable.model.policies then [] else [blocked target "network-policy" stable.model.source "Cross-device policy requires complete symbolic coverage"]))
    (OpenWrt,Nothing) => Right ("openwrt-dsa-fw4-ipv4-v2", [blocked target "dsa-profile" d.source "DSA VLAN realization and operational membership checker is not implemented"])
    (CiscoIOS,_) => Right ("cisco-ios-l2-v2", [blocked target "ios-profile" d.source "Cisco configuration and operational observation primitives are not implemented"])

public export
makeWitness : StableNetwork -> String -> Either (List Diagnostic) RealizationWitness
makeWitness stable target = do
  (profile,claims) <- deriveClaims stable target
  Right (Witness 1 target profile claims)

private
fieldValues : FieldForm -> String -> List UciField -> Maybe (List String)
fieldValues form key fields = traverse read (filter matches fields)
  where
    matches : UciField -> Bool
    matches (Option k _) = k == key
    matches (ListEntry k _) = k == key
    matches (SecretOption k _ _) = k == key

    read : UciField -> Maybe String
    read (Option _ v) = if form == Scalar then Just v.value else Nothing
    read (ListEntry _ v) = if form == OrderedList then Just v.value else Nothing
    read _ = Nothing

private
bindingHolds : TargetAST -> FieldBinding -> Bool
bindingHolds (IOS _) _ = False
bindingHolds (UCI packages) b = case filter ((== b.packageName) . name) packages of
  [p] => case filter ((== b.sectionName) . identifier) p.sections of
    [s] => case b.form of
      SectionKind => [s.kind] == b.expected
      form => fieldValues form b.fieldName s.fields == Just b.expected && (form /= Scalar || length b.expected <= 1)
    _ => False
  _ => False

-- Checks the submitted witness against the authoritative semantic projection
-- and checks its bindings against the submitted target AST. It never recompiles
-- that AST or trusts the generator's claim list. Unsupported claims are retained.
public export
checkWitness : StableNetwork -> Realization -> RealizationWitness -> Either (List Diagnostic) ()
checkWitness stable output witness = do
  expected <- makeWitness stable output.target
  if witness.version /= 1 || witness.target /= output.target || witness.profile /= expected.profile ||
     output.profile /= expected.profile || witness.claims /= expected.claims then
    Left [failure "assurance.witness-forged" stable.model.source "Witness version, target, profile or complete semantic claim set differs"]
    else case concatMap checkClaim witness.claims of
      [] => Right ()
      ds => Left ds
  where
    checkClaim : AssuranceClaim -> List Diagnostic
    checkClaim c = if all (bindingHolds output.ast) c.bindings then [] else
      [failure "assurance.binding-mismatch" c.source ("Target configuration does not realize " ++ c.claimId)]

public export
coverageBlockers : RealizationWitness -> List Diagnostic
coverageBlockers witness = mapMaybe (\c => map (failure "assurance.unsupported" c.source . ((c.claimId ++ ": ") ++)) c.unsupported) witness.claims

public export
checkAdmission : StableNetwork -> Realization -> RealizationWitness -> Either (List Diagnostic) ()
checkAdmission stable output witness = do
  checkWitness stable output witness
  case coverageBlockers witness of
    [] => Right ()
    ds => Left ds

private
bindingJSON : FieldBinding -> String
bindingJSON b = "{\"package\":" ++ jsonString b.packageName ++ ",\"section\":" ++ jsonString b.sectionName ++
  ",\"field\":" ++ jsonString b.fieldName ++ ",\"form\":" ++ jsonString (case b.form of Scalar => "scalar"; OrderedList => "ordered-list"; SectionKind => "section-kind") ++
  ",\"expected\":" ++ jsonArray (map jsonString b.expected) ++ "}"

private
claimJSON : AssuranceClaim -> String
claimJSON c = "{\"id\":" ++ jsonString c.claimId ++ ",\"kind\":" ++ jsonString c.kind ++ ",\"target\":" ++ jsonString c.target ++
  ",\"source\":" ++ spanJSON c.source ++ ",\"bindings\":" ++ jsonArray (map bindingJSON c.bindings) ++
  ",\"unsupported\":" ++ maybe "null" jsonString c.unsupported ++ "}"

public export
witnessJSON : RealizationWitness -> String
witnessJSON w = "{\"version\":" ++ show w.version ++ ",\"target\":" ++ jsonString w.target ++ ",\"profile\":" ++ jsonString w.profile ++
  ",\"claims\":" ++ jsonArray (map claimJSON w.claims) ++ ",\"coverageBlockers\":" ++ jsonArray (map diagnosticJSON (coverageBlockers w)) ++ "}"
