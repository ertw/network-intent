module ForgedGraphReverse
import NetDSL.Domain.Address
import NetDSL.Domain.Model
import NetDSL.Graph.DAG
import Data.So
%default total
bad : TopologicalCertificate [0,1] [(0,1)]
bad = Topological [1,0] Oh
