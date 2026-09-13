module NetDSL.Graph.DAG

import NetDSL.Common
import Data.List
import Data.So

%default total

public export
rank : Nat -> List Nat -> Maybe Nat
rank x = go 0
  where
    go : Nat -> List Nat -> Maybe Nat
    go _ [] = Nothing
    go i (n :: ns) = if n == x then Just i else go (S i) ns

public export
certificateOK : List Nat -> List (Nat, Nat) -> List Nat -> Bool
certificateOK vertices edges order =
  length (nub vertices) == length vertices && length order == length vertices && length (nub order) == length order &&
  all (\v => elem v order) vertices && all forward edges
  where
    forward : (Nat,Nat) -> Bool
    forward (a,b) = case (rank a order, rank b order) of
      (Just x,Just y) => x < y
      _ => False

public export
record TopologicalCertificate (vertices : List Nat) (edges : List (Nat,Nat)) where
  constructor Topological
  order : List Nat
  0 checked : So (certificateOK vertices edges order)

public export
record CycleWitness where
  constructor Cycle
  path : List Nat

public export
data GraphFailure = InvalidGraph String | DirectedCycle CycleWitness

private
cyclePath : Nat -> List (Nat,Nat) -> List Nat -> Nat -> List Nat
cyclePath Z edges seen current = reverse (current :: seen)
cyclePath (S fuel) edges seen current =
  if elem current seen then current :: takeWhile (/= current) seen ++ [current]
  else case find (\(_,b) => b == current) edges of
    Nothing => reverse (current :: seen)
    Just (a,_) => cyclePath fuel edges (current :: seen) a

private
kahn : Nat -> List Nat -> List (Nat,Nat) -> List Nat -> Either CycleWitness (List Nat)
kahn _ [] edges acc = Right (reverse acc)
kahn Z pending edges acc = Left (Cycle pending)
kahn (S fuel) pending edges acc =
  case find (\v => not (any (\(a,b) => b == v && elem a pending) edges)) pending of
    Just v => kahn fuel (filter (/= v) pending) edges (v :: acc)
    Nothing => case pending of
      [] => Right (reverse acc)
      x :: _ => Left (Cycle (cyclePath (S (length pending)) (filter (\(a,b) => elem a pending && elem b pending) edges) [] x))

public export
certifyDAG : (vertices : List Nat) -> (edges : List (Nat,Nat)) -> Either GraphFailure (TopologicalCertificate vertices edges)
certifyDAG vertices edges = do
  if length (nub vertices) == length vertices && all (\(a,b) => elem a vertices && elem b vertices) edges then Right ()
    else Left (InvalidGraph "Vertex IDs must be unique and every edge endpoint must exist")
  order <- either (Left . DirectedCycle) Right (kahn (length vertices) vertices edges [])
  case choose (certificateOK vertices edges order) of
    Left prf => Right (Topological order prf)
    Right _ => Left (InvalidGraph "Topological ordering failed certificate verification")
