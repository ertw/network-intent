module Main
import NetDSL.Syntax.Parser
import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import NetDSL.Resolve
import NetDSL.Validate
import NetDSL.Backend.Compile
import NetDSL.Backend.Render
import NetDSL.Backend.AST
import NetDSL.Docs
import NetDSL.Version
import NetDSL.Assurance.Witness
import NetDSL.Compiler
import Data.List
import Data.String
import Data.Maybe
import System
import System.File

%default total

record Options where
  constructor Opts
  json : Bool
  target : Maybe String
  allTargets : Bool
  maxTagged : Nat
  dependencies : Bool

options : List String -> List String -> Options -> Either String Options
options seen [] o = Right o
options seen ("--format" :: value :: rest) o =
  if elem "--format" seen then Left "Duplicate --format" else
  if value == "json" || value == "text" then options ("--format" :: seen) rest ({ json := value == "json" } o) else Left "--format must be json or text"
options seen ("--target" :: value :: rest) o =
  if elem "--target" seen || o.allTargets then Left "Choose one --target or --all" else options ("--target" :: seen) rest ({ target := Just value } o)
options seen ("--all" :: rest) o = case o.target of
  Just _ => Left "Choose one --target or --all"
  Nothing => if o.allTargets then Left "Duplicate --all" else options ("--all" :: seen) rest ({ allTargets := True } o)
options seen ("--max-tagged-vlans" :: value :: rest) o = case decimal value of
  Just n => if n <= 4094 && not (elem "--max-tagged-vlans" seen) then options ("--max-tagged-vlans" :: seen) rest ({ maxTagged := cast n } o) else Left "--max-tagged-vlans must occur once and be in 0..4094"
  Nothing => Left "--max-tagged-vlans must be an integer"
options seen ("--dependencies" :: rest) o = options ("--dependencies" :: seen) rest ({ dependencies := True } o)
options _ (x :: _) _ = Left ("Unknown or incomplete option: " ++ x)

help : String
help = "netc 0.3.0 — Network Intent DSL 3.0\n\n" ++
  "Usage:\n  netc check FILE [--format json]\n  netc fmt FILE\n" ++
  "  netc compile FILE (--target NAME | --all) [--format json] [--max-tagged-vlans N]\n" ++
  "  netc docs FILE\n  netc graph FILE [--dependencies]\n  netc export FILE\n  netc assurance FILE\n  netc evaluate FILE\n  netc schema\n  netc explain CODE\n\n" ++
  "Compiler-only: no device access, deployment, secret resolution, or observation.\n"

covering
die : String -> IO ()
die msg = do
  _ <- fPutStrLn stderr msg
  exitFailure

covering
report : Bool -> List Diagnostic -> IO ()
report json ds = do
  if json then putStrLn ("{\"ok\":false,\"diagnostics\":" ++ jsonArray (map diagnosticJSON ds) ++ "}")
    else fPutStrLn stderr (join "\n" (map diagnosticText ds)) *> pure ()
  exitFailure

covering
execute : String -> String -> Options -> IO ()
execute cmd file opts = do
  result <- readFile file
  case result of
    Left e => report opts.json [failure "io.read" (MkSpan file 1 1 1 1) (show e)]
    Right input => if cmd == "evaluate" then putStrLn (evaluateSource file input) else case parse file input of
      Left ds => report opts.json ds
      Right document => if cmd == "fmt" then case resolve document of
        Left ds => report opts.json ds
        Right _ => putStr (trim (formatDocument document) ++ "\n")
        else case elaborate document of
          Left ds => report opts.json ds
          Right stable => case cmd of
            "check" => if opts.json then putStrLn ("{\"ok\":true,\"languageVersion\":\"3.0\",\"assurance\":\"certified-model\",\"observed\":\"unknown\",\"diagnostics\":[],\"model\":" ++ semanticJSON stable ++ "}")
              else putStrLn ("OK: " ++ stable.model.name ++ " — model certified; target realization not checked; operational state unknown")
            "docs" => putStr (markdown stable)
            "graph" => putStr (if opts.dependencies then dependencyGraph stable else graph stable)
            "export" => putStrLn (semanticJSON stable)
            "assurance" => case traverse (\t => do
                output <- compileTarget stable t 4094
                witness <- makeWitness stable t
                checkWitness stable output witness
                Right witness) (map name stable.model.devices) of
              Left ds => report True ds
              Right witnesses => putStrLn ("{\"ok\":true,\"version\":1,\"admission\":\"requires-complete-assurance-plan\",\"witnesses\":" ++ jsonArray (map witnessJSON witnesses) ++ "}")
            "compile" => do
              let targets = if opts.allTargets then map name stable.model.devices else maybe [] pure opts.target
              if null targets then die "compile requires --target NAME or --all and at least one target" else
                case traverse (\t => compileTarget stable t opts.maxTagged) targets of
                  Left ds => report opts.json ds
                  Right outputs => if opts.json then putStrLn ("{\"ok\":true,\"exportVersion\":\"3.0\",\"targets\":" ++ jsonArray (map realizationJSON outputs) ++ "}") else putStr (join "\n" (map realizationText outputs))
            _ => die ("Unknown command: " ++ cmd)

covering
run : List String -> IO ()
run [] = putStr help
run ["--help"] = putStr help
run ["--version"] = putStrLn "netc 0.3.0 (language 3.0)"
run ["schema"] = putStrLn manifest
run ["explain",code] = case explain code of
  Just help => putStrLn (code ++ "\n\n" ++ help)
  Nothing => die ("Unknown diagnostic family: " ++ code)
run (cmd :: file :: flags) = if not (elem cmd ["check","fmt","compile","docs","graph","export","assurance","evaluate"]) then die ("Unsupported command: " ++ cmd ++ "\n" ++ help) else
  case options [] flags (Opts False Nothing False 4094 False) of
    Left msg => die msg
    Right opts => if cmd /= "compile" && (opts.allTargets || isJust opts.target || opts.maxTagged /= 4094) then die "Target options apply only to compile"
      else if cmd /= "graph" && opts.dependencies then die "--dependencies applies only to graph"
      else execute cmd file opts
run _ = die help

covering
main : IO ()
main = getArgs >>= run . drop 1
