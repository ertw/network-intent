module NetDSL.Compiler

import NetDSL.Syntax.Parser
import NetDSL.Validate
import NetDSL.Backend.Compile
import NetDSL.Backend.Render
import NetDSL.Backend.AST
import NetDSL.Docs
import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Assurance.Witness
import Data.List
import Data.String

%default total

||| The authoritative, pure compiler boundary used by both native and browser
||| callers.  `source` is never read from a file: diagnostics retain `filename`.
public export
compileSource : String -> String -> Either (List Diagnostic) (StableNetwork, List Realization)
compileSource filename source = do
  document <- parse filename source
  stable <- elaborate document
  targets <- traverse (\device => compileTarget stable device.name 4094) stable.model.devices
  Right (stable, targets)

||| Deterministic JSON for an editor or native parity harness.  It deliberately
||| carries independently checked witnesses without implying full admission.
public export
evaluateSource : String -> String -> String
evaluateSource filename source = case compileSource filename source of
  Left diagnostics =>
    "{\"ok\":false,\"model\":null,\"targets\":[],\"witnesses\":[],\"diagnostics\":" ++ jsonArray (map diagnosticJSON diagnostics) ++ "}"
  Right (stable, targets) => case traverse (\output => do
      witness <- makeWitness stable output.target
      checkWitness stable output witness
      Right witness) targets of
    Left diagnostics => "{\"ok\":false,\"model\":null,\"targets\":[],\"witnesses\":[],\"diagnostics\":" ++ jsonArray (map diagnosticJSON diagnostics) ++ "}"
    Right witnesses => "{\"ok\":true,\"model\":" ++ semanticJSON stable ++ ",\"targets\":" ++
      jsonArray (map realizationJSON targets) ++ ",\"witnesses\":" ++ jsonArray (map witnessJSON witnesses) ++ ",\"diagnostics\":[]}"
