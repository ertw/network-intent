import type { Completeness, Datastore, Freshness } from '../../contracts/presentation';

const DATASTORE_LABELS = {
  committed: 'Committed configuration',
  session_staging: 'Session staging',
  configuration_in_use: 'Configuration in use',
  operational: 'Operational state',
} as const satisfies Record<Datastore, string>;

const FRESHNESS_LABELS = {
  fresh: 'Fresh',
  stale: 'Stale',
  unknown: 'Unknown',
} as const satisfies Record<Freshness, string>;

const COMPLETENESS_LABELS = {
  complete: 'Complete',
  partial: 'Partial',
  unavailable: 'Unavailable',
} as const satisfies Record<Completeness, string>;

export function datastoreLabel(datastore: Datastore): string {
  return DATASTORE_LABELS[datastore];
}

export function emptyGroupsMessage(datastore: Datastore): string {
  return datastore === 'operational'
    ? 'No operational details'
    : 'No configuration details';
}

export function timestampDisplay(value: string | null): string {
  return value === null ? 'Unknown' : value;
}

export function freshnessLabel(freshness: Freshness): string {
  return FRESHNESS_LABELS[freshness];
}

export function completenessLabel(completeness: Completeness): string {
  return COMPLETENESS_LABELS[completeness];
}

export function isIncomplete(completeness: Completeness): boolean {
  return completeness === 'partial' || completeness === 'unavailable';
}

export function reasonDisplay(reason: string | null): string {
  return reason === null ? 'None supplied' : reason;
}

export function missingEmptyLabel(): string {
  return 'No missing items listed';
}
