module NetDSL.Backend.Render

import NetDSL.Common
import NetDSL.Backend.AST
import Data.List
import Data.String

%default total

private
quote : String -> String
quote s = "'" ++ concatMap (\c => if c == '\'' then "'\\''" else singleton c) (unpack s) ++ "'"

private
fieldText : UciField -> String
fieldText (Option k v) = "\toption " ++ k ++ " " ++ quote v.value ++ "\n"
fieldText (ListEntry k v) = "\tlist " ++ k ++ " " ++ quote v.value ++ "\n"

private
sectionText : UciSection -> String
sectionText s = "config " ++ s.kind ++ " " ++ quote s.identifier ++ "\n" ++ concatMap fieldText s.fields ++ "\n"

mutual
  private
  iosText : Nat -> IOSCommand -> String
  iosText depth (Command key args nested _ _) =
    pack (replicate depth ' ') ++ key ++ (if null args then "" else " " ++ join " " (map value args)) ++ "\n" ++
    iosMany (S depth) nested ++ (if depth == 0 then "!\n" else "")

  private
  iosMany : Nat -> List IOSCommand -> String
  iosMany depth [] = ""
  iosMany depth (x :: xs) = iosText depth x ++ iosMany depth xs

public export
render : TargetAST -> List Artifact
render (UCI packages) = map (\p => File ("/etc/config/" ++ p.name) (concatMap sectionText p.sections)) packages
render (IOS commands) = [File "config.ios" (concatMap (iosText 0) commands)]

private
originJSON : String -> SourceSpan -> List String -> String
originJSON key span chain = "{\"generated\":" ++ jsonString key ++ ",\"source\":" ++ spanJSON span ++ ",\"derivation\":" ++ jsonArray (map jsonString chain) ++ "}"

mutual
  private
  iosOrigins : IOSCommand -> List String
  iosOrigins (Command key _ nested at chain) = originJSON key at chain :: originsMany nested

  private
  originsMany : List IOSCommand -> List String
  originsMany [] = []
  originsMany (x :: xs) = iosOrigins x ++ originsMany xs

public export
sourceMap : TargetAST -> String
sourceMap (UCI packages) = jsonArray (concatMap (\p => map (\s => originJSON (p.name ++ "." ++ s.identifier) s.origin s.derivation) p.sections) packages)
sourceMap (IOS commands) = jsonArray (concatMap iosOrigins commands)

public export
realizationJSON : Realization -> String
realizationJSON r = "{\"target\":" ++ jsonString r.target ++ ",\"profile\":" ++ jsonString r.profile ++
  ",\"state\":\"Intended\",\"realization\":\"conditional\",\"assumptions\":" ++ jsonArray (map jsonString r.assumptions) ++
  ",\"files\":" ++ jsonArray (map (\f => "{\"path\":" ++ jsonString f.path ++ ",\"content\":" ++ jsonString f.content ++ "}") (render r.ast)) ++
  ",\"sourceMap\":" ++ sourceMap r.ast ++ "}"

public export
realizationText : Realization -> String
realizationText r = let commentPrefix = case r.ast of UCI _ => "# "; IOS _ => "! " in
  commentPrefix ++ "Intended configuration for " ++ r.target ++ " (" ++ r.profile ++ "); not applied or observed\n" ++
  concatMap (\s => commentPrefix ++ "Assumption: " ++ s ++ "\n") r.assumptions ++
  concatMap (\f => commentPrefix ++ f.path ++ "\n" ++ f.content) (render r.ast)
