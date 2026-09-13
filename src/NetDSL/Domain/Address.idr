module NetDSL.Domain.Address

import NetDSL.Common
import Data.So
import Data.List
import Data.String
import Data.Nat

%default total

public export
pow2 : Nat -> Integer
pow2 Z = 1
pow2 (S n) = 2 * pow2 n

public export
record IPv4 where
  constructor IP4
  number : Integer
  0 bounded : So (number >= 0 && number < 4294967296)

public export
record IPv4Prefix where
  constructor Prefix4
  address : IPv4
  width : Nat
  size : Integer
  0 validWidth : So (width <= 32)
  0 sizeMatchesWidth : So (size == pow2 (32 `minus` width))
  0 canonical : So (mod address.number size == 0)

public export
record VlanId where
  constructor VID
  number : Integer
  0 usable : So (number >= 1 && number <= 4094)

public export
data AddressFamily = IPv4Family | IPv6Family

-- IPv6 deliberately has its own future allocation semantics; IPv4 pools are
-- never generalized to it. No IPv6 source is silently interpreted as IPv4.
public export
mkVlanId : SourceSpan -> String -> Either Diagnostic VlanId
mkVlanId at s = case decimal s of
  Nothing => Left (failure "vlan.invalid-id" at "VLAN ID must be an integer in 1..4094")
  Just n => case choose (n >= 1 && n <= 4094) of
    Left prf => Right (VID n prf)
    Right _ => Left (failure "vlan.invalid-id" at "VLAN ID must be in 1..4094")

public export
mkIPv4 : SourceSpan -> Integer -> Either Diagnostic IPv4
mkIPv4 at n = case choose (n >= 0 && n < 4294967296) of
  Left prf => Right (IP4 n prf)
  Right _ => Left (failure "address.invalid-ipv4" at "IPv4 address is outside its 32-bit range")

public export
parseIPv4 : SourceSpan -> String -> Either Diagnostic IPv4
parseIPv4 at s = case traverse decimal (splitOn '.' s) of
  Just ns@[a,b,c,d] => if all (\n => n >= 0 && n <= 255) ns
                        then mkIPv4 at (a*16777216+b*65536+c*256+d)
                        else Left (failure "address.invalid-ipv4" at "IPv4 octets must be in 0..255")
  _ => Left (failure "address.invalid-ipv4" at ("Invalid IPv4 address: " ++ s))

public export
parsePrefix : SourceSpan -> String -> Either Diagnostic IPv4Prefix
parsePrefix at s = case splitOn '/' s of
  [ip,bits] => do
    addr <- parseIPv4 at ip
    case decimal bits of
      Nothing => Left (failure "address.invalid-prefix" at "CIDR width must be in 0..32")
      Just w => if w > 32 then Left (failure "address.invalid-prefix" at "IPv4 CIDR width must be in 0..32")
                else let width : Nat = cast w
                         size : Integer = pow2 (32 `minus` width) in
                     case choose (width <= 32) of
                       Right _ => Left (failure "address.invalid-prefix" at "Invalid CIDR width")
                       Left bounded => case choose (mod addr.number size == 0) of
                         Left canonical => case choose (size == pow2 (32 `minus` width)) of
                           Left sized => Right (Prefix4 addr width size bounded sized canonical)
                           Right _ => Left (failure "address.invalid-prefix" at "Prefix width and size disagree")
                         Right _ => Left (failure "address.noncanonical-prefix" at ("Prefix has host bits set: " ++ s))
  _ => Left (failure "address.invalid-prefix" at "Expected an IPv4 network prefix, for example 10.0.20.0/24")

public export
showIPv4 : IPv4 -> String
showIPv4 ip = join "." (map show [mod (div ip.number 16777216) 256, mod (div ip.number 65536) 256, mod (div ip.number 256) 256, mod ip.number 256])

public export
showPrefix : IPv4Prefix -> String
showPrefix p = showIPv4 p.address ++ "/" ++ show p.width

public export
contains : IPv4Prefix -> IPv4 -> Bool
contains p ip = ip.number >= p.address.number && ip.number < p.address.number + p.size

public export
overlaps : IPv4Prefix -> IPv4Prefix -> Bool
overlaps a b = a.address.number < b.address.number + b.size && b.address.number < a.address.number + a.size

public export
assignment : IPv4Prefix -> SourceSpan -> String -> Either Diagnostic IPv4
assignment subnet at s = do
  ip <- case unpack s of
    '+' :: rest => case decimal (pack rest) of
      Nothing => Left (failure "address.invalid-offset" at "Expected a nonnegative relative address offset")
      Just n => if n >= subnet.size then Left (failure "address.offset-outside-prefix" at (s ++ " is outside " ++ showPrefix subnet))
                else mkIPv4 at (subnet.address.number + n)
    _ => parseIPv4 at s
  if not (contains subnet ip) then Left (failure "address.outside-prefix" at (showIPv4 ip ++ " is outside " ++ showPrefix subnet))
     else if subnet.width <= 30 && ip.number == subnet.address.number then Left (failure "address.network-address" at "The network address cannot be assigned to a host")
     else if subnet.width <= 30 && ip.number == subnet.address.number + subnet.size - 1 then Left (failure "address.broadcast-address" at "The broadcast address cannot be assigned to a host")
     else Right ip

public export
netmask : IPv4Prefix -> String
netmask p = let n = 4294967296 - p.size in join "." (map show [mod (div n 16777216) 256, mod (div n 65536) 256, mod (div n 256) 256, mod n 256])
