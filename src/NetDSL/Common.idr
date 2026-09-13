module NetDSL.Common

import Data.List
import Data.String

%default total

public export
record SourceSpan where
  constructor MkSpan
  file : String
  line : Nat
  column : Nat
  endLine : Nat
  endColumn : Nat

public export
record Located a where
  constructor At
  span : SourceSpan
  value : a

public export
record Diagnostic where
  constructor MkDiagnostic
  code : String
  primary : SourceSpan
  title : String
  detail : String
  related : List (SourceSpan, String)

public export
failure : String -> SourceSpan -> String -> Diagnostic
failure code span msg = MkDiagnostic code span msg "" []

public export
jsonString : String -> String
jsonString s = "\"" ++ concatMap escape (unpack s) ++ "\""
  where
    escape : Char -> String
    escape '"' = "\\\""
    escape '\\' = "\\\\"
    escape '\n' = "\\n"
    escape '\r' = "\\r"
    escape '\t' = "\\t"
    escape c = if ord c < 32 then "?" else singleton c

public export
jsonArray : List String -> String
jsonArray xs = "[" ++ concat (intersperse "," xs) ++ "]"

public export
spanJSON : SourceSpan -> String
spanJSON s = "{\"file\":" ++ jsonString s.file ++ ",\"line\":" ++ show s.line ++
  ",\"column\":" ++ show s.column ++ ",\"endLine\":" ++ show s.endLine ++
  ",\"endColumn\":" ++ show s.endColumn ++ "}"

public export
diagnosticJSON : Diagnostic -> String
diagnosticJSON d = "{\"code\":" ++ jsonString d.code ++ ",\"severity\":\"error\",\"primarySpan\":" ++
  spanJSON d.primary ++ ",\"title\":" ++ jsonString d.title ++ ",\"detail\":" ++ jsonString d.detail ++
  ",\"related\":" ++ jsonArray (map (\(s, m) => "{\"span\":" ++ spanJSON s ++ ",\"message\":" ++ jsonString m ++ "}") d.related) ++ ",\"fixes\":[]}"

public export
diagnosticText : Diagnostic -> String
diagnosticText d = d.primary.file ++ ":" ++ show d.primary.line ++ ":" ++ show d.primary.column ++
  ": error[" ++ d.code ++ "]: " ++ d.title ++
  (if d.detail == "" then "" else "\n  " ++ d.detail) ++ concatMap (\(s,m) => "\n  " ++ s.file ++ ":" ++ show s.line ++ ":" ++ show s.column ++ ": " ++ m) d.related

public export
splitOn : Char -> String -> List String
splitOn c s = map pack (go (unpack s))
  where
    go : List Char -> List (List Char)
    go [] = [[]]
    go (x :: xs) = case go xs of
      [] => [[x]]
      y :: ys => if x == c then [] :: y :: ys else (x :: y) :: ys

public export
decimal : String -> Maybe Integer
decimal s = if s /= "" && length s <= 20 && all (\c => c >= '0' && c <= '9') (unpack s)
               then Just (foldl (\n,c => n * 10 + cast (ord c - ord '0')) 0 (unpack s))
               else Nothing

public export
join : String -> List String -> String
join sep = concat . intersperse sep

public export
indexed : List a -> List (Nat, a)
indexed = go 0
  where
    go : Nat -> List a -> List (Nat, a)
    go _ [] = []
    go n (x :: xs) = (n,x) :: go (S n) xs

public export
lookupAt : Nat -> List a -> Maybe a
lookupAt _ [] = Nothing
lookupAt Z (x :: _) = Just x
lookupAt (S n) (_ :: xs) = lookupAt n xs

public export
nodeIds : List a -> List Nat
nodeIds xs = map Builtin.fst (indexed xs)
