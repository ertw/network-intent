module ForgedPrefixWidth
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : IPv4Prefix
bad = Prefix4 (IP4 0 Oh) 33 1 Oh Oh Oh
