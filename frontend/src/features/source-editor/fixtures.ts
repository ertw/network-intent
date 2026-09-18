export const BANNER = 'DEVELOPMENT FIXTURE — NOT LIVE DATA';

export const LF_WITH_FINAL = `alpha  two-spaces\n// comment\n"quoted string"\tkeep-tab\n`;
export const LF_WITHOUT_FINAL = 'alpha\nbeta';
export const CRLF_WITH_FINAL = 'alpha\r\nbeta\r\n';
export const CRLF_WITHOUT_FINAL = 'alpha\r\nbeta';
export const MIXED_NEWLINES = 'alpha\nbeta\r\ngamma';
export const STANDALONE_CR = 'alpha\rbeta';
export const UNICODE_SOURCE = `emoji 😀 cafe\u0301 surrogate \uD83D\uDE00 — ${BANNER}`;
export const EMPTY_SOURCE = '';

export type FixtureKey =
  | 'lf-final'
  | 'lf-no-final'
  | 'crlf-final'
  | 'crlf-no-final'
  | 'mixed'
  | 'standalone-cr'
  | 'unicode'
  | 'empty';

export interface SourceFixture {
  key: FixtureKey;
  label: string;
  source: string;
  editable: boolean;
}

export const FIXTURES: readonly SourceFixture[] = Object.freeze([
  {
    key: 'lf-final',
    label: 'Synthetic: uniform LF with final newline',
    source: LF_WITH_FINAL,
    editable: true,
  },
  {
    key: 'lf-no-final',
    label: 'Synthetic: uniform LF without final newline',
    source: LF_WITHOUT_FINAL,
    editable: true,
  },
  {
    key: 'crlf-final',
    label: 'Synthetic: uniform CRLF with final newline',
    source: CRLF_WITH_FINAL,
    editable: true,
  },
  {
    key: 'crlf-no-final',
    label: 'Synthetic: uniform CRLF without final newline',
    source: CRLF_WITHOUT_FINAL,
    editable: true,
  },
  {
    key: 'mixed',
    label: 'Synthetic: mixed LF/CRLF (unsupported)',
    source: MIXED_NEWLINES,
    editable: true,
  },
  {
    key: 'standalone-cr',
    label: 'Synthetic: standalone CR (unsupported)',
    source: STANDALONE_CR,
    editable: true,
  },
  {
    key: 'unicode',
    label: 'Synthetic: Unicode, tabs, comments, quotes',
    source: UNICODE_SOURCE,
    editable: true,
  },
  {
    key: 'empty',
    label: 'Synthetic: empty document',
    source: EMPTY_SOURCE,
    editable: true,
  },
]);

const firstFixture = FIXTURES[0];
if (!firstFixture) {
  throw new Error('source-editor fixtures must not be empty');
}
export const DEFAULT_FIXTURE: SourceFixture = firstFixture;

export function fixtureByKey(key: string): SourceFixture {
  return FIXTURES.find((item) => item.key === key) ?? DEFAULT_FIXTURE;
}
