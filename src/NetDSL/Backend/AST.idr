module NetDSL.Backend.AST

import NetDSL.Common
import NetDSL.AAA
import NetDSL.Router.Options
import Data.So
import Data.List

%default total

public export
safeLine : String -> Bool
safeLine s = all (\c => ord c >= 32 && (ord c < 127 || ord c > 159) && ord c /= 8232 && ord c /= 8233) (unpack s)

public export
record SafeText where
  constructor Text
  value : String
  0 safe : So (safeLine value)

public export
checkedText : SourceSpan -> String -> Either Diagnostic SafeText
checkedText at s = case choose (safeLine s) of
  Left prf => Right (Text s prf)
  Right _ => Left (failure "backend.unsafe-value" at "Target value contains a control character or line separator")

public export
data UciField = Option String SafeText | ListEntry String SafeText | SecretOption String (SecretRef WiFiCredential) Security

public export
record UciSection where
  constructor Section
  kind : String
  identifier : String
  fields : List UciField
  origin : SourceSpan
  derivation : List String

public export
record UciPackage where
  constructor Package
  name : String
  sections : List UciSection

public export
data IOSCommand = Command String (List SafeText) (List IOSCommand) SourceSpan (List String)

public export
data TargetAST = UCI (List UciPackage) | IOS (List IOSCommand)

public export
record Realization where
  constructor Intended
  target : String
  profile : String
  assumptions : List String
  ast : TargetAST

public export
record Artifact where
  constructor File
  path : String
  content : String

public export
option : SourceSpan -> String -> String -> Either Diagnostic UciField
option s k v = Option k <$> checkedText s v

public export
listEntry : SourceSpan -> String -> String -> Either Diagnostic UciField
listEntry s k v = ListEntry k <$> checkedText s v

public export
section : SourceSpan -> List String -> String -> String -> List (String,String) -> List (String,String) -> Either Diagnostic UciSection
section at chain kind name options lists = do
  if name /= "" && all (\c => ord c < 128 && (isAlphaNum c || c == '_')) (unpack name)
    then Right () else Left (failure "backend.capability-mismatch" at "UCI section identifiers require ASCII letters, digits, or underscores")
  opts <- traverse (\(k,v) => option at k v) options
  items <- traverse (\(k,v) => listEntry at k v) lists
  Right (Section kind name (opts ++ items) at chain)

public export
command : SourceSpan -> List String -> String -> List String -> List IOSCommand -> Either Diagnostic IOSCommand
command at chain key args nested = do
  values <- traverse (checkedText at) args
  Right (Command key values nested at chain)

public export
checkTargetAST : TargetAST -> Either Diagnostic TargetAST
checkTargetAST ast@(IOS _) = Right ast
checkTargetAST ast@(UCI packages) = do
  traverse_ checkPackage packages
  Right ast
  where
    checkPackage : UciPackage -> Either Diagnostic ()
    checkPackage p = go [] p.sections
      where
        go : List String -> List UciSection -> Either Diagnostic ()
        go seen [] = Right ()
        go seen (s :: rest) = if elem s.identifier seen then
          Left (failure "backend.section-collision" s.origin ("Duplicate generated UCI section in " ++ p.name ++ ": " ++ s.identifier))
          else go (s.identifier :: seen) rest
