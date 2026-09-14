module NetDSL.Backend.Render

import NetDSL.Common
import NetDSL.AAA
import NetDSL.Router.Options
import NetDSL.Backend.AST
import Data.List
import Data.String

%default total

private
quote : String -> String
quote s = "'" ++ concatMap (\c => if c == '\'' then "'\\''" else singleton c) (unpack s) ++ "'"

private
placeholder : String -> String -> String
placeholder sectionId key = "__NETC_SECRET_" ++ sectionId ++ "_" ++ key ++ "__"

private
fieldText : String -> UciField -> String
fieldText sectionId (Option k v) = "\toption " ++ k ++ " " ++ quote v.value ++ "\n"
fieldText sectionId (ListEntry k v) = "\tlist " ++ k ++ " " ++ quote v.value ++ "\n"

fieldText sectionId (SecretOption k _ _) = "\toption " ++ k ++ " " ++ quote (placeholder sectionId k) ++ "\n"

private
secretField : UciField -> Bool
secretField (SecretOption _ _ _) = True
secretField _ = False

private
hasSecrets : UciPackage -> Bool
hasSecrets p = any (any secretField . fields) p.sections

private
artifactPath : UciPackage -> String
artifactPath p = "/etc/config/" ++ p.name ++ if hasSecrets p then ".template" else ""

private
sectionText : UciSection -> String
sectionText s = "config " ++ s.kind ++ " " ++ quote s.identifier ++ "\n" ++ concatMap (fieldText s.identifier) s.fields ++ "\n"

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

private
bindings : UciPackage -> UciSection -> List String
bindings p s = mapMaybe binding s.fields
  where
    binding : UciField -> Maybe String
    binding (SecretOption key ref security) = Just (
      "{\"reference\":" ++ jsonString (secretURI ref) ++ ",\"kind\":\"wifi-credential\",\"security\":" ++ jsonString (showSecurity security) ++
      ",\"templateArtifact\":" ++ jsonString (artifactPath p) ++ ",\"installationPath\":" ++ jsonString ("/etc/config/" ++ p.name) ++
      ",\"section\":" ++ jsonString s.identifier ++ ",\"option\":" ++ jsonString key ++ ",\"placeholder\":" ++ jsonString (placeholder s.identifier key) ++ "}")
    binding _ = Nothing

private
bindingManifest : List UciPackage -> String
bindingManifest packages = "{\"manifestVersion\":1,\"status\":\"requires-secret-binding\",\"bindings\":" ++ jsonArray (concatMap (\p => concatMap (bindings p) p.sections) packages) ++ "}"

public export
requiresSecretBinding : TargetAST -> Bool
requiresSecretBinding (UCI packages) = any hasSecrets packages
requiresSecretBinding _ = False

public export
render : TargetAST -> List Artifact
render (UCI packages) = map (\p => File (artifactPath p)
  ((if hasSecrets p then "# TEMPLATE: requires external secret binding; do not install directly\n" else "") ++ concatMap sectionText p.sections)) packages ++
  (if any hasSecrets packages then [File "secret-bindings.json" (bindingManifest packages ++ "\n")] else [])
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
sourceMap (UCI packages) = jsonArray (concatMap (\p => concatMap (origins p) p.sections) packages)
  where
    origins : UciPackage -> UciSection -> List String
    origins p s = originJSON (artifactPath p ++ "#" ++ s.identifier) s.origin s.derivation :: mapMaybe secretOrigin s.fields
      where
        secretOrigin : UciField -> Maybe String
        secretOrigin (SecretOption key _ _) = Just (originJSON ("secret-bindings.json#" ++ placeholder s.identifier key) s.origin (s.derivation ++ ["external credential binding"]))
        secretOrigin _ = Nothing
sourceMap (IOS commands) = jsonArray (concatMap iosOrigins commands)

public export
realizationJSON : Realization -> String
realizationJSON r = "{\"target\":" ++ jsonString r.target ++ ",\"profile\":" ++ jsonString r.profile ++
  ",\"state\":\"Intended\",\"realization\":\"conditional\",\"assumptions\":" ++ jsonArray (map jsonString r.assumptions) ++
  ",\"readiness\":" ++ jsonString (if requiresSecretBinding r.ast then "requires-secret-binding" else "intended-config") ++
  ",\"requiresSecretBinding\":" ++ (if requiresSecretBinding r.ast then "true" else "false") ++
  ",\"files\":" ++ jsonArray (map (\f => "{\"path\":" ++ jsonString f.path ++ ",\"content\":" ++ jsonString f.content ++ "}") (render r.ast)) ++
  ",\"sourceMap\":" ++ sourceMap r.ast ++ "}"

public export
realizationText : Realization -> String
realizationText r = let commentPrefix = case r.ast of UCI _ => "# "; IOS _ => "! " in
  commentPrefix ++ "Intended configuration for " ++ r.target ++ " (" ++ r.profile ++ "); not applied or observed\n" ++
  (if requiresSecretBinding r.ast then commentPrefix ++ "REQUIRES SECRET BINDING: templates are not installation-ready\n" else "") ++
  concatMap (\s => commentPrefix ++ "Assumption: " ++ s ++ "\n") r.assumptions ++
  concatMap (\f => commentPrefix ++ f.path ++ "\n" ++ f.content) (render r.ast)
