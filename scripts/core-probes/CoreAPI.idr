module CoreAPI

import NetDSL.Common
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import NetDSL.Validate
import Data.So
import Data.List
import System

%default total

at : SourceSpan
at = MkSpan "core-api" 1 1 1 2

subnet : IPv4Prefix
subnet = Prefix4 (IP4 167772160 Oh) 24 256 Oh Oh Oh

vlan : Vlan
vlan = MkVlan "trusted" (VID 10 Oh) (At at subnet) (Just (At at (IP4 167772161 Oh))) Nothing [] at

port : Port
port = MkPort (Id 0) "lan1" Nothing (Access (Id 0)) Nothing at

router : Device
router = MkDevice "router" True OpenWrt [port] 4094 Nothing at

network : Network V2
network = MkNetwork "home" Nothing [vlan] [router] [] [] [] (Just (Id 0)) at

rejects : Network V2 -> Bool
rejects n = case certify n of
  Left _ => True
  Right _ => False

rejectsWith : String -> Network V2 -> Bool
rejectsWith expected n = case certify n of
  Left diagnostics => any ((== expected) . code) diagnostics
  Right _ => False

accepts : Network V2 -> Bool
accepts = not . rejects

service : Service
service = MkService "dns" [(UDP,53)] [] at

route : Route
route = MkRoute "upstream" (At at (Prefix4 (IP4 0 Oh) 0 4294967296 Oh Oh Oh)) (At at (IP4 167772162 Oh)) (Id 0) (Id 0) 0 [] at

validWitness : List (Nat,Nat) -> List Nat -> Bool
validWitness edges [] = False
validWitness edges [_] = False
validWitness edges path@(x :: xs) = last' path == Just x && all (\edge => elem edge edges) (zip path xs)

cycleTest : Bool
cycleTest = case certifyDAG [0,1,2] [(0,1),(1,2),(2,0)] of
  Left (DirectedCycle w) => validWitness [(0,1),(1,2),(2,0)] w.path
  _ => False

checks : List (String,Bool)
checks =
 [ ("valid-core-model", accepts network)
 , ("duplicate-vlan-name", rejects ({vlans := [vlan,vlan]} network))
 , ("invalid-network-name", rejects ({name := "bad\nname"} network))
 , ("invalid-interface-name", rejects ({devices := [{ports := [{name := "bad\nport"} port]} router]} network))
 , ("owner-mismatch", rejects ({devices := [{ports := [{owner := Id 8} port]} router]} network))
 , ("vlan-ref-out-of-range", rejects ({devices := [{ports := [{mode := Access (Id 3)} port]} router]} network))
 , ("empty-trunk", rejects ({devices := [{ports := [{mode := Trunk []} port]} router]} network))
 , ("duplicate-memberships", rejects ({devices := [{ports := [{mode := Trunk [Id 0,Id 0]} port]} router]} network))
 , ("device-peer-out-of-range", rejects ({devices := [{ports := [{peer := Just (At at (DevicePort (PortId (Id 9) 0)))} port]} router]} network))
 , ("host-peer-out-of-range", rejects ({devices := [{ports := [{peer := Just (At at (HostPort (Id 8)))} port]} router]} network))
 , ("enforcer-out-of-range", rejects ({enforcer := Just (Id 99)} network))
 , ("gateway-outside-prefix", rejects ({vlans := [{gateway := Just (At at (IP4 0 Oh))} vlan]} network))
 , ("gateway-network-address", rejects ({vlans := [{gateway := Just (At at (IP4 167772160 Oh))} vlan]} network))
 , ("invalid-service-port", rejects ({services := [MkService "bad" [(UDP,65536)] [] at]} network))
 , ("empty-service", rejects ({services := [MkService "bad" [] [] at]} network))
 , ("missing-service-dependency", rejectsWith "reference.unknown-service" ({services := [{dependencies := [At at (Id 3)]} service]} network))
 , ("missing-policy-service", rejects ({policies := [Rule (Id 0) Internet Allow [Id 2] at]} network))
 , ("missing-policy-zone", rejects ({policies := [Rule (Id 8) Internet Allow [] at]} network))
 , ("missing-policy-gateway", rejects ({policies := [Rule (Id 0) (Gateway (Id 8)) Allow [] at]} network))
 , ("route-valid", accepts ({routes := [route]} network))
 , ("route-missing-vlan", rejects ({routes := [{vlan := Id 8} route]} network))
 , ("route-missing-device", rejects ({routes := [{device := Id 8} route]} network))
 , ("route-missing-dependency", rejectsWith "reference.unknown-route" ({routes := [{dependencies := [At at (Id 8)]} route]} network))
 , ("route-negative-metric", rejects ({routes := [{metric := -1} route]} network))
 , ("route-self-next-hop", rejects ({routes := [{nextHop := At at (IP4 167772161 Oh)} route]} network))
 , ("dag-valid", case certifyDAG [0,1,2] [(0,1),(1,2)] of Right _ => True; Left _ => False)
 , ("dag-missing-endpoint-rejected", case certifyDAG [0,1] [(0,3)] of Left (InvalidGraph _) => True; _ => False)
 , ("dag-duplicate-inventory-invalid", case certifyDAG [0,0] [] of Left (InvalidGraph _) => True; _ => False)
 , ("dag-empty-graph-valid", case certifyDAG [] [] of Right _ => True; Left _ => False)
 , ("dag-repeated-edge-valid", case certifyDAG [0,1] [(0,1),(0,1)] of Right _ => True; Left _ => False)
 , ("dag-order-missing-vertex-rejected", not (certificateOK [0,1,2] [(0,1)] [0,1]))
 , ("dag-order-backward-edge-rejected", not (certificateOK [0,1] [(1,0)] [0,1]))
 , ("dag-duplicate-inventory-rejected", not (certificateOK [0,0] [] [0,1]))
 , ("dag-cycle-witness-directed", cycleTest)
 ]

main : IO ()
main = do
 traverse_ (\(label,ok) => putStrLn ((if ok then "PASS " else "FAIL ") ++ label)) checks
 if all snd checks then exitSuccess else exitFailure
