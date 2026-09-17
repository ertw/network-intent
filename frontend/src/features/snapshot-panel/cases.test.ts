import { describe, expect, it } from 'vitest';
import { DEFAULT_SNAPSHOT_CASE, SNAPSHOT_CASES, snapshotCaseById } from './cases';

describe('synthetic snapshot cases', () => {
  it('labels every fixture as synthetic and covers each datastore', () => {
    const datastores = new Set(SNAPSHOT_CASES.map((item) => item.props.datastore));
    expect(datastores).toEqual(
      new Set(['committed', 'session_staging', 'configuration_in_use', 'operational']),
    );
    expect(SNAPSHOT_CASES.length).toBeGreaterThan(0);
    for (const item of SNAPSHOT_CASES) {
      expect(item.label.startsWith('Synthetic:')).toBe(true);
      expect(item.props.title.toLowerCase()).toContain('synthetic');
    }
  });

  it('keeps empty scalar, empty list, redacted, and unknown as distinct kinds', () => {
    const fixture = snapshotCaseById('redacted-unknown');
    const fields = fixture.props.groups.flatMap((group) => group.fields);
    const emptyScalar = fields.find((field) => field.id === 'empty-scalar');
    const emptyList = fields.find((field) => field.id === 'empty-list');
    const redacted = fields.find((field) => field.id === 'community-string');
    const unknown = fields.find((field) => field.id === 'uptime');

    expect(emptyScalar).toEqual({
      id: 'empty-scalar',
      label: 'Empty description',
      kind: 'scalar',
      value: '',
    });
    expect(emptyList).toEqual({
      id: 'empty-list',
      label: 'DNS servers',
      kind: 'ordered_list',
      values: [],
    });
    expect(redacted).toEqual({
      id: 'community-string',
      label: 'Community string',
      kind: 'redacted',
    });
    expect(unknown).toEqual({
      id: 'uptime',
      label: 'Uptime',
      kind: 'unknown',
      reason: 'Synthetic collector did not return uptime',
    });
    expect(redacted).not.toHaveProperty('value');
    expect(unknown).not.toHaveProperty('value');
  });

  it('falls back to the default committed case for an unknown selector id', () => {
    expect(DEFAULT_SNAPSHOT_CASE.id).toBe('committed-complete');
    expect(snapshotCaseById('does-not-exist')).toBe(DEFAULT_SNAPSHOT_CASE);
  });
});
