module NetDSL.Router.Fields

import NetDSL.Common
import NetDSL.Syntax.Parser
import NetDSL.Backend.AST
import Data.List
import Data.String

%default total

public export
problem : String -> Statement -> String -> Either Diagnostic a
problem code s msg = Left (failure code (stmtSpan s) msg)

public export
named : Statement -> String
named s = case stmtWords s of
  _ :: n :: _ => n
  _ => ""

public export
fields : List String -> Statement -> Either Diagnostic ()
fields keys s = traverse_ (\c => if elem (keyword c) keys then Right () else problem "syntax.unknown-construct" c "Unsupported field in typed router declaration") (children s)

public export
field : String -> Statement -> Either Diagnostic (Maybe Token)
field key s = case filter ((==key) . keyword) (children s) of
  [] => Right Nothing
  [c] => case (stmtTokens c,hasBlock c) of
    ([_,t],False) => Right (Just t)
    _ => problem "syntax.field" c ("Expected " ++ key ++ " VALUE")
  _ => problem "name.duplicate-field" s ("Duplicate field: " ++ key)

public export
required : String -> Statement -> Either Diagnostic Token
required key s = do
  t <- field key s
  maybe (problem "syntax.missing-field" s ("Required field: " ++ key)) Right t

public export
values : String -> Statement -> Either Diagnostic (List Token)
values key s = concat <$> traverse get (filter ((==key) . keyword) (children s))
  where
    get : Statement -> Either Diagnostic (List Token)
    get c = case (stmtTokens c,hasBlock c) of
      (_ :: t :: ts,False) => Right (t :: ts)
      _ => problem "syntax.field" c ("Expected " ++ key ++ " VALUE...")

public export
optional : (Token -> Either Diagnostic a) -> String -> Statement -> Either Diagnostic (Maybe a)
optional f key s = field key s >>= traverse f

public export
choice : List (String,a) -> Token -> Either Diagnostic a
choice options t = maybe (Left (failure "syntax.invalid-value" t.source "Value is not supported for this typed setting")) Right (lookup t.text options)

public export
boolean : Token -> Either Diagnostic Bool
boolean = choice [("true",True),("false",False)]

public export
natural : Integer -> Integer -> Token -> Either Diagnostic Nat
natural low high t = case decimal t.text of
  Just n => if n >= low && n <= high then Right (cast n) else bad
  Nothing => bad
  where
    bad : Either Diagnostic Nat
    bad = Left (failure "syntax.invalid-number" t.source ("Expected integer in " ++ show low ++ ".." ++ show high))

public export
safeString : Token -> Either Diagnostic String
safeString t = if safeLine t.text && length t.text <= 1024 then Right t.text else
  Left (failure "backend.unsafe-value" t.source "Expected at most 1024 characters without controls or line separators")

public export
validName : String -> Bool
validName s = length s >= 1 && length s <= 63 && all (\c => ord c < 128 && (isAlphaNum c || c == '-' || c == '_')) (unpack s)

public export
namedBlock : Statement -> Either Diagnostic ()
namedBlock s = case (stmtWords s,hasBlock s) of
  ([_,n],True) => if validName n then Right () else problem "name.invalid" s "Invalid router entity name"
  _ => problem "syntax.block" s "Expected KIND NAME { ... }"

public export
uniqueNames : List (String,SourceSpan) -> Either Diagnostic ()
uniqueNames [] = Right ()
uniqueNames ((n,at) :: rest) = case find ((==n) . fst) rest of
  Just (_,other) => Left (MkDiagnostic "name.duplicate" at ("Duplicate name: " ++ n) "" [(other,"Other declaration")])
  Nothing => uniqueNames rest
