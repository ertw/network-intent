import { describe, expect, it } from 'vitest';
import { ALL_KIND_IDS, FIXTURES, HTML_LIKE } from './cases';

describe('changes-panel cases', () => {
  it('includes all four kinds in supplied order without calculating a diff', () => {
    expect(ALL_KIND_IDS).toEqual([
      'added-row',
      'removed-row',
      'modified-row',
      'blocked-row',
      'long-row',
    ]);
    expect(FIXTURES['all-kinds'].changes.map((entry) => entry.kind)).toEqual([
      'added',
      'removed',
      'modified',
      'blocked',
      'modified',
    ]);
    expect(Object.isFrozen(FIXTURES['all-kinds'].changes)).toBe(true);
  });

  it('keeps HTML-like text, duplicate list values, and empty distinctions as supplied data', () => {
    const blocked = FIXTURES['all-kinds'].changes.find((entry) => entry.id === 'blocked-row');
    expect(blocked?.label.includes(HTML_LIKE)).toBe(true);
    expect(blocked?.explanation?.includes('<img')).toBe(true);
    const empty = FIXTURES['whitespace-empty'].changes[0];
    expect(empty?.before[0]).toMatchObject({ kind: 'scalar', value: '' });
    expect(empty?.before[1]).toMatchObject({ kind: 'ordered_list', values: [] });
    expect(empty?.after[1]).toMatchObject({
      kind: 'ordered_list',
      values: ['', 'dup', 'dup', ''],
    });
    expect(FIXTURES.empty.changes).toHaveLength(0);
  });
});
