module NetDSL.State

import NetDSL.Validate
import NetDSL.Backend.AST

%default total

public export
data Evidence a = Unknown | Reported String String a

public export
record AppliedConfig where
  constructor Applied
  target : String
  acceptedAt : String
  evidenceSource : String
  files : List Artifact

public export
record OperationalSnapshot where
  constructor Observed
  target : String
  collectedAt : String
  source : String
  interfaceUp : List (String,Evidence Bool)
  dnsAnswers : Evidence Bool

-- StableNetwork is desired model assurance; Realization is intended config.
-- There is intentionally no pure conversion to applied or observed evidence.
