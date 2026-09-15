module WitnessAPI

import NetDSL.Common
import NetDSL.Compiler
import NetDSL.Assurance.Witness
import NetDSL.Backend.AST
import NetDSL.Validate
import Data.List
import System
import System.File

%default total

isRight : Either a b -> Bool
isRight (Right _) = True
isRight _ = False

corrupt : TargetAST -> TargetAST
corrupt (UCI packages) = UCI (map (\p => { sections := map (\s => { fields := [] } s) p.sections } p) packages)
corrupt x = x

test : StableNetwork -> Realization -> Bool
test stable output = case makeWitness stable output.target of
  Left _ => False
  Right witness => isRight (checkWitness stable output witness) &&
    not (isRight (checkWitness stable output ({ claims := [] } witness))) &&
    not (isRight (checkWitness stable output ({ profile := "forged" } witness))) &&
    not (isRight (checkWitness stable ({ ast := corrupt output.ast } output) witness)) &&
    not (isRight (checkAdmission stable output witness)) && not (null (coverageBlockers witness))

covering
main : IO ()
main = do
  Right source <- readFile "core-router.net" | Left err => printLn err *> exitFailure
  Right (stable,outputs) <- pure (compileSource "core-router.net" source) | Left _ => exitFailure
  if not (null outputs) && all (test stable) outputs then
    putStrLn "PASS independent realization witness, forgery, missing bindings and unsupported admission"
    else exitFailure
