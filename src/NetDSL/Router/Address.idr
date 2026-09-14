module NetDSL.Router.Address

import NetDSL.Common
import NetDSL.Domain.Address
import NetDSL.Syntax.Parser
import Data.List
import Data.String
import Data.So
import Data.Maybe

%default total

public export
record IPv6 where
  constructor IP6
  number : Integer
  0 bounded : So (number >= 0 && number < pow2 128)

public export
record IPv6Prefix where
  constructor Prefix6
  address : IPv6
  width : Nat
  0 validWidth : So (width <= 128)
  0 canonical : So (mod address.number (pow2 (128 `minus` width)) == 0)

private
hexGroup : String -> Maybe Integer
hexGroup s = if length s < 1 || length s > 4 then Nothing else
  foldl (\acc,c => do
    n <- acc
    d <- if c >= '0' && c <= '9' then Just (ord c - ord '0')
         else if toLower c >= 'a' && toLower c <= 'f' then Just (10 + ord (toLower c) - ord 'a') else Nothing
    Just (n * 16 + cast d)) (Just 0) (unpack s)

public export
parseIPv6 : SourceSpan -> String -> Either Diagnostic IPv6
parseIPv6 at s = do
  let parts = splitOn ':' s
  let emptyCount = length (filter (== "") parts)
  groups <- if emptyCount == 0 then
      if length parts == 8 then maybe bad Right (traverse hexGroup parts) else bad
    else do
      let middle = filter (/= "") parts
      let leadingEmpty = isPrefixOf [':', ':'] (unpack s)
      let trailingEmpty = isSuffixOf [':', ':'] (unpack s)
      let expectedEmpty = 1 + (if leadingEmpty then 1 else 0) + (if trailingEmpty then 1 else 0)
      if not (isInfixOf [':', ':'] (unpack s)) || emptyCount /= expectedEmpty || length middle >= 8 then bad else Right ()
      let before = takeWhile (/= "") parts
      let after = filter (/= "") (drop (length before) parts)
      a <- maybe bad Right (traverse hexGroup before)
      b <- maybe bad Right (traverse hexGroup after)
      Right (a ++ replicate (8 `minus` length middle) 0 ++ b)
  let n = foldl (\a,b => a*65536+b) 0 groups
  case choose (n >= 0 && n < pow2 128) of
    Left prf => Right (IP6 n prf)
    Right _ => bad
  where
    bad : Either Diagnostic a
    bad = Left (failure "address.invalid-ipv6" at "Expected an IPv6 address in full or compressed hexadecimal notation")

public export
parsePrefix6 : SourceSpan -> String -> Either Diagnostic IPv6Prefix
parsePrefix6 at s = case splitOn '/' s of
  [a,w] => do
    ip <- parseIPv6 at a
    n <- maybe bad Right (decimal w)
    if n > 128 then bad else Right ()
    let width : Nat = cast n
    case choose (width <= 128) of
      Right _ => bad
      Left bounded => case choose (mod ip.number (pow2 (128 `minus` width)) == 0) of
        Left canonical => Right (Prefix6 ip width bounded canonical)
        Right _ => Left (failure "address.noncanonical-prefix" at "IPv6 prefix has host bits set")
  _ => bad
  where
    bad : Either Diagnostic a
    bad = Left (failure "address.invalid-ipv6-prefix" at "Expected IPv6 network prefix with width in 0..128")

private
hex4 : Integer -> String
hex4 n = pack (map (\shift => fromMaybe '0' (lookupAt (cast (mod (div n (pow2 shift)) 16)) (unpack "0123456789abcdef"))) [12,8,4,0])

public export
showIPv6 : IPv6 -> String
showIPv6 ip = join ":" (map (\shift => hex4 (mod (div ip.number (pow2 shift)) 65536)) [112,96,80,64,48,32,16,0])

public export
showPrefix6 : IPv6Prefix -> String
showPrefix6 p = showIPv6 p.address ++ "/" ++ show p.width

public export
record Address4 where
  constructor OnSubnet4
  address : IPv4
  subnet : IPv4Prefix

public export
parseAddress4 : Token -> Either Diagnostic Address4
parseAddress4 t = case splitOn '/' t.text of
  [a,w] => do
    ip <- parseIPv4 t.source a
    n <- maybe (Left (failure "address.invalid-prefix" t.source "Invalid IPv4 prefix width")) Right (decimal w)
    if n > 32 then Left (failure "address.invalid-prefix" t.source "Invalid IPv4 prefix width") else Right ()
    base <- mkIPv4 t.source (div ip.number (pow2 (32 `minus` cast n)) * pow2 (32 `minus` cast n))
    parsedPrefix <- parsePrefix t.source (showIPv4 base ++ "/" ++ w)
    Right (OnSubnet4 ip parsedPrefix)
  _ => Left (failure "address.invalid-prefix" t.source "Expected interface IPv4 address/prefix")

public export
record Address6 where
  constructor OnSubnet6
  address : IPv6
  width : Nat
  0 validWidth : So (width <= 128)

public export
parseAddress6 : Token -> Either Diagnostic Address6
parseAddress6 t = case splitOn '/' t.text of
  [a,w] => do
    ip <- parseIPv6 t.source a
    n <- maybe bad Right (decimal w)
    if n > 128 then bad else Right ()
    let width : Nat = cast n
    case choose (width <= 128) of
      Left prf => Right (OnSubnet6 ip width prf)
      Right _ => bad
  _ => bad
  where
    bad : Either Diagnostic a
    bad = Left (failure "address.invalid-ipv6-prefix" t.source "Expected IPv6 interface address/prefix with width in 0..128")

public export
poolEndpoints : IPv4Prefix -> Statement -> Either Diagnostic (Located IPv4, Located IPv4)
poolEndpoints subnet st = if hasBlock st then bad else do
  (first,last) <- case stmtTokens st of
    [_,a,dots,b] => if dots.text /= ".." then bad else do
      first <- assignment subnet a.source a.text
      last <- assignment subnet b.source b.text
      Right (At a.source first, At b.source last)
    [_,start,a,max,b] => if start.text /= "start" || max.text /= "max" || not (isPrefixOf ['+'] (unpack a.text)) then bad else do
      first <- assignment subnet a.source a.text
      count <- maybe bad Right (decimal b.text)
      if count < 1 then Left (failure "address.invalid-dhcp-count" b.source "DHCP max must be a positive address count") else Right ()
      last <- mkIPv4 b.source (first.number + count - 1)
      if contains subnet last then Right () else Left (failure "address.outside-prefix" b.source "DHCP pool exceeds its subnet")
      Right (At a.source first, At b.source last)
    _ => bad
  if first.value.number > last.value.number then Left (failure "address.inverted-dhcp-range" (stmtSpan st) "DHCP start must not exceed its end") else Right ()
  if subnet.width >= 31 then Left (failure "address.dhcp-prefix-too-small" (stmtSpan st) "DHCP requires broadcast and host space") else Right ()
  Right (first,last)
  where
    bad : Either Diagnostic a
    bad = Left (failure "syntax.dhcp" (stmtSpan st) "Expected dhcp START .. END or dhcp start +OFFSET max COUNT")
