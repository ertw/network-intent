module ForgedPrefixCanonical
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : IPv4Prefix
bad = Prefix4 (IP4 1 Oh) 24 256 Oh Oh Oh
