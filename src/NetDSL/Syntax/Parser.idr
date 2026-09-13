module NetDSL.Syntax.Parser

import NetDSL.Common
import Data.List
import Data.String
import Data.Nat

%default total

public export
data TokenKind = Word | Quoted | Open | Close | EOL | Comment

public export
record Token where
  constructor Tok
  kind : TokenKind
  text : String
  source : SourceSpan

public export
data Statement = Stmt SourceSpan (List Token) (Maybe (List Statement))

public export
stmtSpan : Statement -> SourceSpan
stmtSpan (Stmt s _ _) = s

public export
stmtTokens : Statement -> List Token
stmtTokens (Stmt _ ts _) = ts

public export
stmtWords : Statement -> List String
stmtWords = map text . stmtTokens

public export
children : Statement -> List Statement
children (Stmt _ _ (Just xs)) = xs
children _ = []

public export
hasBlock : Statement -> Bool
hasBlock (Stmt _ _ (Just _)) = True
hasBlock _ = False

public export
keyword : Statement -> String
keyword s = case stmtWords s of
  x :: _ => x
  [] => ""

public export
record Document where
  constructor Parsed
  sourceFile : String
  sourceText : String
  tokens : List Token
  statements : List Statement

private
lexChars : Nat -> String -> Nat -> Nat -> List Char -> Either Diagnostic (List Token)
lexChars Z file line col _ = Left (failure "syntax.limit" (MkSpan file line col line col) "Input exceeds lexer work limit")
lexChars (S fuel) file line col [] = Right []
lexChars (S fuel) file line col (c :: cs) =
  let here = MkSpan file line col line (S col) in
  case c of
    '\r' => lexChars fuel file line col cs
    '\n' => (Tok EOL "\n" here ::) <$> lexChars fuel file (S line) 1 cs
    '{' => (Tok Open "{" here ::) <$> lexChars fuel file line (S col) cs
    '}' => (Tok Close "}" here ::) <$> lexChars fuel file line (S col) cs
    '#' => commentToken fuel file line col (c :: cs)
    '"' => do
      (value, consumed, rest) <- quoted fuel here [] 1 cs
      more <- lexChars fuel file line (col + consumed) rest
      Right (Tok Quoted value (MkSpan file line col line (col + consumed)) :: more)
    _ => if c == '/' && isSlash cs then commentToken fuel file line col (c :: cs)
         else if isSpace c then lexChars fuel file line (S col) cs
         else let (word, rest) = span (\x => not (isSpace x) && x /= '{' && x /= '}' && x /= '#' && x /= '"') (c :: cs)
                  n = length word in
              if n == 0 then Left (failure "syntax.token" here "Unexpected character")
              else (Tok Word (pack word) (MkSpan file line col line (col + n)) ::) <$> lexChars fuel file line (col+n) rest
  where
    isSlash : List Char -> Bool
    isSlash ('/' :: _) = True
    isSlash _ = False

    quoted : Nat -> SourceSpan -> List Char -> Nat -> List Char -> Either Diagnostic (String, Nat, List Char)
    quoted Z s acc n xs = Left (failure "syntax.limit" s "String exceeds lexer work limit")
    quoted (S k) s acc n [] = Left (failure "syntax.unclosed-string" s "Unterminated quoted string")
    quoted (S k) s acc n ('"' :: xs) = Right (pack (reverse acc), S n, xs)
    quoted (S k) s acc n ('\n' :: xs) = Left (failure "syntax.unclosed-string" s "Quoted strings must remain on one line; use an escape")
    quoted (S k) s acc n ('\\' :: x :: xs) = case x of
      'n' => quoted k s ('\n' :: acc) (n+2) xs
      'r' => quoted k s ('\r' :: acc) (n+2) xs
      't' => quoted k s ('\t' :: acc) (n+2) xs
      '"' => quoted k s ('"' :: acc) (n+2) xs
      '\\' => quoted k s ('\\' :: acc) (n+2) xs
      _ => Left (failure "syntax.escape" s "Unknown string escape")
    quoted (S k) s acc n (x :: xs) = quoted k s (x :: acc) (S n) xs

    commentToken : Nat -> String -> Nat -> Nat -> List Char -> Either Diagnostic (List Token)
    commentToken k f l c xs =
      let (body, rest) = span (/= '\n') xs in
      (Tok Comment (pack body) (MkSpan f l c l (c + length body)) ::) <$> lexChars k f l (c + length body) rest

private
meaningful : Token -> Bool
meaningful (Tok Comment _ _) = False
meaningful _ = True

mutual
  private
  block : Nat -> SourceSpan -> Bool -> List Token -> Either Diagnostic (List Statement, List Token)
  block Z s nested ts = Left (failure "syntax.limit" s "Syntax nesting/work limit exceeded")
  block (S fuel) s nested [] = if nested then Left (failure "syntax.unclosed-block" s "Missing closing brace") else Right ([], [])
  block (S fuel) s nested (Tok EOL _ _ :: ts) = block fuel s nested ts
  block (S fuel) s nested (Tok Close _ close :: ts) =
    if nested then Right ([], ts) else Left (failure "syntax.unexpected-close" close "Unexpected closing brace")
  block (S fuel) s nested ts = do
    (stmt, rest) <- statement fuel s [] ts
    (more, end) <- block fuel s nested rest
    Right (stmt :: more, end)

  private
  statement : Nat -> SourceSpan -> List Token -> List Token -> Either Diagnostic (Statement, List Token)
  statement Z s acc ts = Left (failure "syntax.limit" s "Statement work limit exceeded")
  statement (S fuel) s [] [] = Left (failure "syntax.statement" s "Expected statement")
  statement (S fuel) s acc [] = Right (Stmt s (reverse acc) Nothing, [])
  statement (S fuel) s [] (Tok Open _ at :: ts) = Left (failure "syntax.statement" at "Expected declaration before opening brace")
  statement (S fuel) s acc (Tok Open _ at :: ts) = do
    (body, rest) <- block fuel at True ts
    Right (Stmt s (reverse acc) (Just body), rest)
  statement (S fuel) s acc (Tok EOL _ _ :: ts) = Right (Stmt s (reverse acc) Nothing, ts)
  statement (S fuel) s acc ts@(Tok Close _ _ :: _) = Right (Stmt s (reverse acc) Nothing, ts)
  statement (S fuel) s [] (t :: ts) = statement fuel t.source [t] ts
  statement (S fuel) s acc (t :: ts) = statement fuel s (t :: acc) ts

public export
parse : String -> String -> Either (List Diagnostic) Document
parse file input =
  let start = MkSpan file 1 1 1 1 in
  if length input > 1048576 then Left [failure "syntax.limit" start "Source exceeds 1 MiB limit"]
  else case lexChars (S (length input)) file 1 1 (unpack input) of
    Left d => Left [d]
    Right ts => if length ts > 32768 || not (boundedNesting 0 ts) then Left [failure "syntax.limit" start "Source exceeds 32768 tokens or 64 nested blocks"] else case block (S (length ts)) start False (filter meaningful ts) of
      Left d => Left [d]
      Right (stmts, []) => Right (Parsed file input ts stmts)
      Right (_, t :: _) => Left [failure "syntax.trailing" t.source "Unexpected trailing input"]
  where
    boundedNesting : Nat -> List Token -> Bool
    boundedNesting depth [] = True
    boundedNesting depth (t :: ts) = case t.kind of
      Open => if depth >= 64 then False else boundedNesting (S depth) ts
      Close => boundedNesting (pred depth) ts
      _ => boundedNesting depth ts

-- Token-based formatting preserves comments and quoted values. Braces delimit
-- declarations; newlines delimit scalar statements. It does not elaborate.
public export
formatDocument : Document -> String
formatDocument doc = go 0 True doc.tokens ++ "\n"
  where
    indent : Nat -> String
    indent n = pack (replicate (n*2) ' ')

    go : Nat -> Bool -> List Token -> String
    go n fresh [] = ""
    go n fresh (Tok EOL _ _ :: ts) = (if fresh then "" else "\n") ++ go n True ts
    go n fresh (Tok Open _ _ :: ts) = " {\n" ++ go (S n) True ts
    go n fresh (Tok Close _ _ :: ts) = (if fresh then "" else "\n") ++ indent (pred n) ++ "}" ++ go (pred n) False ts
    go n fresh (Tok Comment t _ :: ts) = (if fresh then indent n else " ") ++ t ++ go n False ts
    go n fresh (Tok Quoted t _ :: ts) = (if fresh then indent n else " ") ++ jsonString t ++ go n False ts
    go n fresh (Tok Word t _ :: ts) = (if fresh then indent n else " ") ++ t ++ go n False ts
