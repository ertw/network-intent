module ForgedPrefix6
import NetDSL.Router.Address
import Data.So
%default total
bad : IPv6Prefix
bad = Prefix6 (IP6 1 Oh) 64 Oh Oh
