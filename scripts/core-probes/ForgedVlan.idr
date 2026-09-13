module ForgedVlan
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : VlanId
bad = VID 0 Oh
