module WirelessAPI

import NetDSL.Common
import NetDSL.AAA
import NetDSL.Domain.Model
import NetDSL.Domain.Address
import NetDSL.Router.Address
import NetDSL.Syntax.Parser
import NetDSL.Validate
import Data.List
import System
import System.File

%default total

private
rejects : Network V3 -> Bool
rejects n = case certify n of Left _ => True; Right _ => False

private
checks : StableNetwork -> List (String,Bool)
checks stable = case (stable.model.devices, stable.model.wirelessLinks) of
  ([gateway,satellite], [link]) => case satellite.routing of
    Nothing => [("wireless-fixture",False)]
    Just c =>
      let bad : RouterConfig -> Bool = \cfg => rejects ({devices := [gateway,{routing := Just cfg} satellite]} stable.model) in
      [("wireless-valid-certification",not (rejects stable.model)),
       ("wireless-forged-owner",rejects ({wirelessLinks := [{station := At link.source (WiFiId (Id 999) (RRef 0))} link]} stable.model)),
       ("wireless-forged-interface",rejects ({wirelessLinks := [{station := At link.source (WiFiId (Id 1) (RRef 999))} link]} stable.model)),
       ("wireless-reversed-endpoints",rejects ({wirelessLinks := [{station := link.accessPoint, accessPoint := link.station} link]} stable.model)),
       ("wireless-duplicate-uplink",rejects ({wirelessLinks := [link,link]} stable.model))] ++
      (case c.wifiInterfaces of
        s :: rest => [("wireless-station-without-wds",bad ({wifiInterfaces := {settings := {wds := Nothing} s.settings} s :: rest} c)),
                      ("wireless-forged-bssid",bad ({wifiInterfaces := {settings := {bssid := Just "bad\nvalue"} s.settings} s :: rest} c)),
                      ("wireless-cross-device-credential",bad ({wifiInterfaces := {credential := Nothing} s :: rest} c)),
                      ("wireless-station-interface-kind",bad ({wifiInterfaces := {ifaceRef := At s.source (RRef 999)} s :: rest} c))]
        _ => [("wireless-station-fixture",False)]) ++
      (case c.interfaces of
        loopback :: lan :: rest =>
          [("wireless-forged-gateway",bad ({interfaces := loopback :: {gateway := map (\a => At lan.source a.value.address) (head' lan.addresses)} lan :: rest} c)),
           ("wireless-unattached-static",bad ({interfaces := loopback :: {attachment := At lan.source Unattached} lan :: rest} c)),
           ("wireless-duplicate-dns",bad ({interfaces := loopback :: {dnsServers := lan.dnsServers ++ lan.dnsServers} lan :: rest} c))]
        _ => [("wireless-lan-fixture",False)])
  _ => [("wireless-network-fixture",False)]

covering
main : IO ()
main = do
  Right source <- readFile "wds-network.net" | Left e => putStrLn (show e) *> exitFailure
  Right stable <- pure (parse "wds-network.net" source >>= elaborate) | Left ds => traverse_ (putStrLn . diagnosticText) ds *> exitFailure
  let results = checks stable
  traverse_ (\(label,ok) => putStrLn ((if ok then "PASS " else "FAIL ") ++ label)) results
  if all snd results then exitSuccess else exitFailure
