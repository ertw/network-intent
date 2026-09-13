module ForgedPrefixSize
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : IPv4Prefix
bad = Prefix4 (IP4 0 Oh) 24 512 Oh Oh Oh
