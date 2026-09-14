module NetDSL.Migration

import NetDSL.Common
import NetDSL.Domain.Model
import NetDSL.Validate

%default total

-- Foundation/spike API, not language-3.0 migration syntax or a deployment engine.
-- Only availability debt is representable here. Structural and security
-- invariants cannot be waived, and candidates are recertified on finish.
public export
data DebtKind = KnownViolation | DeferredProof | RequiresObservation

public export
data AvailabilityObligation = DNSAvailable (Ref VLAN)

public export
record Debt where
  constructor Owed
  obligation : AvailabilityObligation
  kind : DebtKind
  reason : String

public export
record MigrationState (outstanding : List Debt) where
  constructor Migrating
  candidate : StableNetwork
  previous : StableNetwork

public export
begin : StableNetwork -> MigrationState []
begin stable = Migrating stable stable

public export
introduce : (debt : Debt) -> MigrationState debts -> MigrationState (debt :: debts)
introduce debt state = Migrating state.candidate state.previous

public export
replaceCandidate : Network V3 -> MigrationState debts -> Either (List Diagnostic) (MigrationState debts)
replaceCandidate n state = do
  certified <- certify n
  Right (Migrating certified state.previous)

public export
data DischargeEvidence : Debt -> Type where
  -- These constructors are assertions supplied at a future integration boundary,
  -- not proofs of live DNS health. Observation evidence remains visibly distinct.
  ModelEvidence : (detail : String) -> DischargeEvidence (Owed o DeferredProof why)
  RepairEvidence : (detail : String) -> DischargeEvidence (Owed o KnownViolation why)
  ObservationEvidence : (timestamp : String) -> (source : String) -> (detail : String) ->
    DischargeEvidence (Owed o RequiresObservation why)

public export
discharge : DischargeEvidence debt -> MigrationState (debt :: debts) -> MigrationState debts
discharge evidence state = Migrating state.candidate state.previous

public export
finish : MigrationState [] -> Either (List Diagnostic) StableNetwork
finish state = certify state.candidate.model
