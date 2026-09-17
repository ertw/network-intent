import { describe, expect, it } from 'vitest';
import type { Datastore } from '../../contracts/presentation';
import {
  completenessLabel,
  datastoreLabel,
  emptyGroupsMessage,
  freshnessLabel,
  isIncomplete,
  missingEmptyLabel,
  reasonDisplay,
  timestampDisplay,
} from './labels';

const DATASTORES: readonly Datastore[] = [
  'committed',
  'session_staging',
  'configuration_in_use',
  'operational',
];

describe('datastoreLabel', () => {
  it('maps each datastore to a distinct non-Applied label', () => {
    const labels = DATASTORES.map(datastoreLabel);
    expect(labels).toEqual([
      'Committed configuration',
      'Session staging',
      'Configuration in use',
      'Operational state',
    ]);
    expect(new Set(labels).size).toBe(4);
    expect(labels.some((label) => /applied/i.test(label))).toBe(false);
  });
});

describe('emptyGroupsMessage', () => {
  it('uses operational wording only for the operational datastore', () => {
    expect(emptyGroupsMessage('operational')).toBe('No operational details');
    expect(emptyGroupsMessage('committed')).toBe('No configuration details');
    expect(emptyGroupsMessage('session_staging')).toBe('No configuration details');
    expect(emptyGroupsMessage('configuration_in_use')).toBe('No configuration details');
  });
});

describe('timestampDisplay', () => {
  it('shows Unknown only for null, leaving empty and ISO strings unchanged', () => {
    expect(timestampDisplay(null)).toBe('Unknown');
    expect(timestampDisplay('')).toBe('');
    expect(timestampDisplay('2026-09-17T16:01:02.000Z')).toBe('2026-09-17T16:01:02.000Z');
    expect(timestampDisplay('  2026-09-17T16:01:02.000Z  ')).toBe(
      '  2026-09-17T16:01:02.000Z  ',
    );
  });
});

describe('freshness and completeness labels', () => {
  it('displays supplied tokens without inventing extra states', () => {
    expect(freshnessLabel('fresh')).toBe('Fresh');
    expect(freshnessLabel('stale')).toBe('Stale');
    expect(freshnessLabel('unknown')).toBe('Unknown');
    expect(completenessLabel('complete')).toBe('Complete');
    expect(completenessLabel('partial')).toBe('Partial');
    expect(completenessLabel('unavailable')).toBe('Unavailable');
  });

  it('treats only partial and unavailable as incomplete warnings', () => {
    expect(isIncomplete('complete')).toBe(false);
    expect(isIncomplete('partial')).toBe(true);
    expect(isIncomplete('unavailable')).toBe(true);
  });
});

describe('null optional provenance strings', () => {
  it('keeps placeholders distinct from collector-supplied text', () => {
    expect(reasonDisplay(null)).toBe('None supplied');
    expect(reasonDisplay('Synthetic collector timed out')).toBe(
      'Synthetic collector timed out',
    );
    expect(reasonDisplay('')).toBe('');
    expect(missingEmptyLabel()).toBe('No missing items listed');
  });
});
