module NetDSL.Tests

import NetDSL.Common
import NetDSL.Syntax.Parser
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import NetDSL.Validate
import NetDSL.Migration
import NetDSL.AAA
import NetDSL.State
import Data.List
import Data.Maybe
import System

%default total

sample : String
sample = "network-language 3.0\nnetwork test {\nvlan servers 20 {\nsubnet 10.0.20.0/24\ngateway +1\nhost nas +10\n}\nrouter gateway {\ndriver openwrt\nport lan1 { access servers }\n}\n}\n"

loadSample : Either (List Diagnostic) StableNetwork
loadSample = parse "test.net" sample >>= elaborate

at : SourceSpan
at = MkSpan "test.net" 1 1 1 1

subsets : List a -> List (List a)
subsets [] = [[]]
subsets (x :: xs) = let rest = subsets xs in rest ++ map (x ::) rest

-- Independent exhaustive oracle: a 3-vertex graph is acyclic iff at least
-- one of the six possible orderings puts every edge forward.
graphTests : Bool
graphTests = all checkGraph (subsets [(0,1),(0,2),(1,0),(1,2),(2,0),(2,1),(0,0),(1,1),(2,2)])
  where
    pathValid : List (Nat,Nat) -> List Nat -> Bool
    pathValid edges (a :: b :: rest) = elem (a,b) edges && pathValid edges (b :: rest)
    pathValid _ _ = True

    checkGraph : List (Nat,Nat) -> Bool
    checkGraph edges =
      let oracle = any (certificateOK [0,1,2] edges) [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]] in
      case certifyDAG [0,1,2] edges of
        Right cert => oracle && certificateOK [0,1,2] edges cert.order
        Left (DirectedCycle witness) => not oracle && pathValid edges witness.path
        Left (InvalidGraph _) => False


prefixTests : Bool
prefixTests = case parsePrefix at "10.0.20.0/23" of
  Left _ => False
  Right subnet => case assignment subnet at "+300" of
    Left _ => False
    Right ip => showIPv4 ip == "10.0.21.44" && subnet.size == 512 && netmask subnet == "255.255.254.0"

rejects : Network V3 -> Bool
rejects n = case certify n of Left _ => True; Right _ => False

modelTests : Bool
modelTests = case loadSample of
  Left _ => False
  Right stable => case (stable.model.vlans, parseIPv4 at "192.0.2.10") of
    (v :: _,Right outside) =>
      let badHost = HostAt "outside" (At at outside) at
          badVlan = { hosts := badHost :: v.hosts } v
          badPool = Pool (At at outside) (At at outside)
          noRouter = { devices := [], enforcer := Nothing } stable.model in
      rejects ({ vlans := [badVlan] } stable.model) &&
      rejects ({ vlans := [v,v] } stable.model) &&
      rejects ({ vlans := [{ dhcp := Just badPool } v] } stable.model) &&
      rejects ({ services := [MkService "bad" [] [] at] } stable.model) && rejects noRouter
    _ => False

migrationTests : Bool
migrationTests = case loadSample of
  Left _ => False
  Right stable =>
    case finish (discharge (RepairEvidence "model listener restored; operational status remains unknown")
      (introduce (Owed (DNSAvailable (Id 0)) KnownViolation "DNS listener moves next") (begin stable))) of
      Left _ => False
      Right _ => True

aaaTests : Bool
aaaTests =
  let contract = ManagementShell ["state.read","config.write"] True True at
      capable = AAAProfile [["state.read","config.write"]] True
      noAccounting = AAAProfile [["state.read","config.write"]] False
      excessive = AAAProfile [["state.read","config.write","device.reboot"]] True in
  null (checkAAA contract capable) && not (null (checkAAA contract noAccounting)) && not (null (checkAAA contract excessive)) &&
  (case decideAuthentication True Rejected of Refuse => True; _ => False) &&
  (case decideAuthentication True Unavailable of UseIndependentRecovery => True; _ => False) &&
  (case secretReference {kind=TACACSSharedKey} at "plaintext-secret" of Left _ => True; Right _ => False)

covering
main : IO ()
main = do
  let tests = [("512 directed-graph cases",graphTests),("prefix arithmetic",prefixTests),("public certification rejects malformed models",modelTests),
               ("migration obligation introduction/discharge",migrationTests),("AAA fallback, exact roles, accounting and secret references",aaaTests)]
  traverse_ (\(name,ok) => putStrLn ((if ok then "PASS " else "FAIL ") ++ name)) tests
  if all snd tests then pure () else exitFailure
