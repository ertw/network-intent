module ForgedIPv6
import NetDSL.Router.Address
import Data.So
%default total
bad : IPv6
bad = IP6 340282366920938463463374607431768211456 Oh
