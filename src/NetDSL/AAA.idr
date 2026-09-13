module NetDSL.AAA

import NetDSL.Common
import Data.List

%default total

-- A pure capability spike. Actual AAA source syntax and target commands are
-- deliberately unavailable in language 1.0.
public export
data AuthenticationResult = Accepted | Rejected | Unavailable

public export
data AuthenticationDecision = Grant | Refuse | UseIndependentRecovery

public export
decideAuthentication : Bool -> AuthenticationResult -> AuthenticationDecision
decideAuthentication recovery Accepted = Grant
decideAuthentication recovery Rejected = Refuse
decideAuthentication True Unavailable = UseIndependentRecovery
decideAuthentication False Unavailable = Refuse

public export
data SecretKind = TACACSSharedKey | WireGuardPrivateKey | PasswordHash

export
data SecretRef : SecretKind -> Type where
  Reference : String -> SecretRef kind

export
secretReference : SourceSpan -> String -> Either Diagnostic (SecretRef kind)
secretReference span uri = if isPrefixOf (unpack "secret://") (unpack uri) && length uri > 9 &&
  all (\c => ord c >= 33 && ord c <= 126) (unpack uri)
  then Right (Reference uri) else Left (failure "secret.invalid-reference" span "Expected an opaque secret:// reference; literal secret values are not accepted")

public export
record AAAContract where
  constructor ManagementShell
  requiredPermissions : List String
  commandAccountingRequired : Bool
  independentRecovery : Bool
  source : SourceSpan

public export
record AAACapabilities where
  constructor AAAProfile
  realizableRoles : List (List String)
  commandAccounting : Bool

public export
checkAAA : AAAContract -> AAACapabilities -> List Diagnostic
checkAAA contract caps =
  (if any (\role => all (\p => elem p role) contract.requiredPermissions && all (\p => elem p contract.requiredPermissions) role) caps.realizableRoles
     then [] else [failure "aaa.authorization-not-realizable" contract.source "No target role has exactly the requested permissions; privilege broadening is forbidden"]) ++
  (if contract.commandAccountingRequired && not caps.commandAccounting then [failure "aaa.accounting-not-realizable" contract.source "Target cannot produce required command-execution accounting"] else []) ++
  (if contract.independentRecovery then [] else [failure "aaa.no-independent-recovery-path" contract.source "Management access requires an independent recovery path"])
