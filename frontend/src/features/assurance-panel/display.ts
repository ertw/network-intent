import type { Completeness, Freshness, Outcome } from '../../contracts/presentation';

const OUTCOME_LABELS: Record<Outcome, string> = {
  success: 'Success',
  violation: 'Violation',
  timeout: 'Timeout',
  refused: 'Refused',
  unreachable: 'Unreachable',
  dns_nx_domain: 'DNS NXDOMAIN',
  dns_server_failure: 'DNS server failure',
  tls_failure: 'TLS failure',
  malformed_response: 'Malformed response',
  unsupported: 'Unsupported',
  unavailable: 'Unavailable',
};

export function outcomeLabel(outcome: Outcome | null): string {
  if (outcome === null) {
    return 'No observation';
  }
  return OUTCOME_LABELS[outcome];
}

/**
 * Display styling only. Not a health, admission, or deployment decision.
 * A positive accent is allowed solely when the supplied outcome is success
 * and the supplied provenance is already fresh and complete.
 */
export function allowsPositiveAccent(
  outcome: Outcome | null,
  freshness: Freshness,
  completeness: Completeness,
): boolean {
  return outcome === 'success' && freshness === 'fresh' && completeness === 'complete';
}

export function timestampLabel(value: string | null): string {
  return value === null ? 'Unknown' : value;
}
