import type { ChangeEntry, DisplayField } from '../../contracts/presentation';

export const BANNER = 'DEVELOPMENT FIXTURE — NOT LIVE DATA';
export const HTML_LIKE =
  '<img src="x" onerror="alert(1)"> <script>document.title="pwned"</script>';
export const LONG_TEXT = Array.from({ length: 28 }, (_, index) => {
  const line = String(index + 1).padStart(2, '0');
  return `synthetic-line-${line}  keep  two  spaces  ${'x'.repeat(20)}`;
}).join('\n');
export const MISSING_SELECTED_ID = 'missing-change-id';

function freezeField(field: DisplayField): DisplayField {
  if (field.kind === 'ordered_list') {
    return Object.freeze({ ...field, values: Object.freeze([...field.values]) });
  }
  return Object.freeze({ ...field });
}

function freezeEntry(entry: ChangeEntry): ChangeEntry {
  return Object.freeze({
    ...entry,
    before: Object.freeze(entry.before.map(freezeField)),
    after: Object.freeze(entry.after.map(freezeField)),
  });
}

export type FixtureKey =
  | 'all-kinds'
  | 'empty'
  | 'unknown-selection'
  | 'whitespace-empty';

export interface ChangesFixture {
  key: FixtureKey;
  label: string;
  changes: readonly ChangeEntry[];
  initialSelectedId: string | null;
}

const ALL_KIND_CHANGES: readonly ChangeEntry[] = Object.freeze([
  freezeEntry({
    id: 'added-row',
    label: 'Added hostname',
    kind: 'added',
    before: [],
    after: [{ id: 'hostname', label: 'Hostname', kind: 'scalar', value: 'synthetic-lab' }],
    explanation: 'Synthetic added row — DEVELOPMENT FIXTURE — NOT LIVE DATA',
  }),
  freezeEntry({
    id: 'removed-row',
    label: 'Removed banner',
    kind: 'removed',
    before: [{ id: 'banner', label: 'Banner', kind: 'scalar', value: 'old-banner' }],
    after: [],
    explanation: null,
  }),
  freezeEntry({
    id: 'modified-row',
    label: 'Modified DNS list',
    kind: 'modified',
    before: [
      {
        id: 'dns',
        label: 'DNS servers',
        kind: 'ordered_list',
        values: ['192.0.2.1', '192.0.2.1', ''],
      },
    ],
    after: [
      {
        id: 'dns',
        label: 'DNS servers',
        kind: 'ordered_list',
        values: ['192.0.2.1', '192.0.2.8', '192.0.2.1'],
      },
    ],
    explanation: 'Synthetic modified ordered list — DEVELOPMENT FIXTURE — NOT LIVE DATA',
  }),
  freezeEntry({
    id: 'blocked-row',
    label: `Blocked ${HTML_LIKE}`,
    kind: 'blocked',
    before: [{ id: 'secret', label: 'Community string', kind: 'redacted' }],
    after: [
      {
        id: 'uptime',
        label: 'Uptime',
        kind: 'unknown',
        reason: `Synthetic unknown ${HTML_LIKE}`,
      },
    ],
    explanation: `Blocked explanation ${HTML_LIKE}`,
  }),
  freezeEntry({
    id: 'long-row',
    label: 'Long text change',
    kind: 'modified',
    before: [{ id: 'banner-before', label: 'Banner', kind: 'scalar', value: LONG_TEXT }],
    after: [{ id: 'banner-after', label: 'Banner', kind: 'scalar', value: LONG_TEXT }],
    explanation: LONG_TEXT,
  }),
]);

export const FIXTURES: Record<FixtureKey, ChangesFixture> = {
  'all-kinds': {
    key: 'all-kinds',
    label: 'Synthetic: added, removed, modified, blocked',
    changes: ALL_KIND_CHANGES,
    initialSelectedId: null,
  },
  empty: {
    key: 'empty',
    label: 'Synthetic: empty change list',
    changes: Object.freeze([]),
    initialSelectedId: null,
  },
  'unknown-selection': {
    key: 'unknown-selection',
    label: 'Synthetic: unknown selected id',
    changes: ALL_KIND_CHANGES,
    initialSelectedId: MISSING_SELECTED_ID,
  },
  'whitespace-empty': {
    key: 'whitespace-empty',
    label: 'Synthetic: empty scalar, empty list, whitespace',
    changes: Object.freeze([
      freezeEntry({
        id: 'empty-fields',
        label: 'Empty and whitespace fields',
        kind: 'modified',
        before: [
          { id: 'empty-scalar', label: 'Empty description', kind: 'scalar', value: '' },
          { id: 'empty-list', label: 'Flags', kind: 'ordered_list', values: [] },
        ],
        after: [
          { id: 'spaces', label: 'Note', kind: 'scalar', value: 'alpha  beta' },
          {
            id: 'dup-list',
            label: 'Tags',
            kind: 'ordered_list',
            values: ['', 'dup', 'dup', ''],
          },
        ],
        explanation: null,
      }),
    ]),
    initialSelectedId: 'empty-fields',
  },
};

export const FIXTURE_KEYS = Object.keys(FIXTURES) as FixtureKey[];
export const DEFAULT_FIXTURE = FIXTURES['all-kinds'];
export const ALL_KIND_IDS = ALL_KIND_CHANGES.map((entry) => entry.id);
