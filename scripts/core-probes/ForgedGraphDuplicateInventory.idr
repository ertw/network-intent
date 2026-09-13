module ForgedGraphDuplicateInventory
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : TopologicalCertificate [0,0] []
bad = Topological [0,1] Oh
