module ForgedIPv4
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : IPv4
bad = IP4 4294967296 Oh
