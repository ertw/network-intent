module WrongRefKind
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : Ref VLAN
bad = the (Ref DeviceKind) (Id 0)
