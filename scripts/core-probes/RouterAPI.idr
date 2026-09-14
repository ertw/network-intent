module RouterAPI

import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Router.Validate
import NetDSL.Syntax.Parser
import NetDSL.Validate
import NetDSL.Backend.AST
import NetDSL.Backend.Render
import NetDSL.AAA
import Data.List
import Data.Maybe
import Data.String
import System
import System.File

%default total

private
rejects : Network V3 -> Bool
rejects n = case certify n of Left _ => True; Right _ => False

private
checks : StableNetwork -> List (String,Bool)
checks stable = case stable.model.devices of
  [d] => case d.routing of
    Nothing => [("explicit-router-present",False)]
    Just c =>
      let bad : RouterConfig -> Bool = \cfg => rejects ({devices := [{routing := Just cfg} d]} stable.model) in
      [("router-valid-certification",not (rejects stable.model)),
       ("device-independent-of-enforcer",not (rejects ({devices := [{isRouter := False} d], enforcer := Nothing} stable.model))),
       ("router-duplicate-interfaces",bad ({interfaces := c.interfaces ++ c.interfaces} c)),
       ("router-missing-default-policy",bad ({defaults := At c.defaults.span ({input := Nothing} c.defaults.value)} c)),
       ("router-invalid-duid",maybe False (\g => bad ({globals := Just (At g.span ({dhcpDefaultDuid := Just "not-hex"} g.value))} c)) c.globals)] ++
      (case c.bridges of
        b :: bs => [("router-forged-attachment",bad ({bridges := [{members := [At b.source (Physical (RRef 999))]} b]} c)),
                    ("router-cycle-at-certification",bad ({bridges := [{members := [At b.source (BridgeDevice (RRef 0))]} b]} c)),
                    ("router-empty-bridge",bad ({bridges := [{members := []} b]} c))]
        _ => [("router-bridge-fixture",False)]) ++
      (case c.bonds of
        b :: _ => [("router-bond-invalid-min-links",bad ({bonds := [{settings := {minLinks := Just 0} b.settings} b]} c)),
                   ("router-bond-invalid-mtu",bad ({bonds := [{settings := {mtu := Just 1} b.settings} b]} c)),
                   ("router-bond-duplicate-members",bad ({bonds := [{members := b.members ++ b.members} b]} c))]
        _ => [("router-bond-fixture",False)]) ++
      (case c.dhcpServers of
        server :: _ => [("router-dhcp-unknown-interface",bad ({dhcpServers := [{ifaceRef := At server.source (RRef 999)} server]} c)),
                        ("router-dhcp-ignored-pool",bad ({dhcpServers := [{settings := {ignore := Just True} server.settings} server]} c))]
        _ => [("router-dhcp-fixture",False)]) ++
      (case c.zones of
        z :: zs => [("router-zone-unknown-interface",bad ({zones := {interfaces := [At z.source (RRef 999)]} z :: zs} c)),
                    ("router-dhcp-control-policy",bad ({zones := {settings := {input := Just Drop} z.settings} z :: zs} c))]
        _ => [("router-zone-fixture",False)]) ++
      (case c.rules of
        r :: rs => [("router-rule-port-bounds",bad ({rules := {settings := {destinationPort := Just 65536} r.settings} r :: rs} c)),
                    ("router-rule-family-mismatch",bad ({rules := {protocols := [ProtoIGMP], settings := {family := Just Family6} r.settings} r :: rs} c))]
        _ => [("router-rule-fixture",False)]) ++
      (case c.wifiInterfaces of
        a :: rest => [("router-forged-radio-ref",bad ({wifiInterfaces := {radio := At a.source (RRef 999)} a :: rest} c)),
                      ("router-unsafe-ssid",bad ({wifiInterfaces := {settings := {ssid := Just "bad\nssid"} a.settings} a :: rest} c)),
                      ("router-secured-ap-missing-reference",bad ({wifiInterfaces := {settings := {security := Just WPA3} a.settings, credential := Nothing} a :: rest} c)),
                      ("router-invalid-reference-from-generic-api",case secretReference {kind=WiFiCredential} a.source "secret://../invalid" of
                        Left _ => False
                        Right ref => bad ({wifiInterfaces := {settings := {security := Just WPA3} a.settings, credential := Just ref} a :: rest} c))]
        _ => [("router-ap-fixture",False)])
  _ => [("router-device-fixture",False)]

private
quotingTest : Bool
quotingTest =
  let at = MkSpan "synthetic" 1 1 1 1 in
  case section at ["synthetic quoting probe"] "wifi-iface" "probe" [("key","synthetic'password")] [] of
    Left _ => False
    Right s => case render (UCI [Package "synthetic" [s]]) of
      [artifact] => isInfixOf (unpack "'synthetic'\\''password'") (unpack artifact.content)
      _ => False

covering
main : IO ()
main = do
  Right source <- readFile "core-router.net" | Left e => putStrLn (show e) *> exitFailure
  Right stable <- pure (parse "core-router.net" source >>= elaborate) | Left ds => traverse_ (putStrLn . diagnosticText) ds *> exitFailure
  let results = ("synthetic-credential-uci-quoting",quotingTest) :: checks stable
  traverse_ (\(label,ok) => putStrLn ((if ok then "PASS " else "FAIL ") ++ label)) results
  if all snd results then exitSuccess else exitFailure
