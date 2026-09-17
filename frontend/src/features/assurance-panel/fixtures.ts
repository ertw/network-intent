import type { AssuranceRow, Primitive, Provenance } from '../../contracts/presentation';

export const LARGE_PLAN_ID = '9007199254740993';
export const LARGE_GRAPH_VERSION = 'graph-9007199254740993';
export const LARGE_ROW_ID = '9007199254740993';
export const LONG_DEPENDENCY_ID =
  'dep-synthetic-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
export const HTML_LIKE_DETAIL =
  '<img src=x onerror=alert(1)> synthetic detail — DEVELOPMENT FIXTURE — NOT LIVE DATA';
export const HTML_LIKE_LABEL =
  'synthetic <img src=x onerror=alert(1)> html-like — DEVELOPMENT FIXTURE — NOT LIVE DATA';
export const MISSING_SELECTED_ID = 'missing-observation-id';

const BANNER = 'DEVELOPMENT FIXTURE — NOT LIVE DATA';

function freezeProvenance(provenance: Provenance): Provenance {
  return Object.freeze({
    ...provenance,
    missing: Object.freeze([...provenance.missing]),
  });
}

function freezeRow(row: AssuranceRow): AssuranceRow {
  return Object.freeze({
    ...row,
    dependsOn: Object.freeze([...row.dependsOn]),
    provenance: freezeProvenance(row.provenance),
  });
}

function provenance(overrides: Partial<Provenance> = {}): Provenance {
  return freezeProvenance({
    sourceLabel: `synthetic-collector — ${BANNER}`,
    collectedAt: '2026-09-17T16:00:00.000Z',
    receivedAt: '2026-09-17T16:00:01.000Z',
    freshness: 'fresh',
    completeness: 'complete',
    missing: [],
    reason: null,
    ...overrides,
  });
}

function row(
  id: string,
  label: string,
  primitive: Primitive,
  outcome: AssuranceRow['outcome'],
  extra: {
    detail?: string;
    provenance?: Provenance;
    sourceLabel?: string;
    endpointLabel?: string | null;
    dependsOn?: readonly string[];
  } = {},
): AssuranceRow {
  return freezeRow({
    id,
    label: `${label} — ${BANNER}`,
    primitive,
    outcome,
    detail: extra.detail ?? `synthetic ${id} detail — ${BANNER}`,
    provenance: extra.provenance ?? provenance(),
    sourceLabel: extra.sourceLabel ?? `synthetic-probe-adapter — ${BANNER}`,
    endpointLabel: extra.endpointLabel === undefined ? `endpoint://${id}` : extra.endpointLabel,
    dependsOn: extra.dependsOn ?? [],
  });
}

export const ALL_OUTCOME_ROWS: readonly AssuranceRow[] = Object.freeze([
  row(LARGE_ROW_ID, 'synthetic fresh success', 'tcp_connect', 'success', {
    dependsOn: [LONG_DEPENDENCY_ID, 'dep-upstream-dns'],
    detail: `synthetic fresh success on ${LARGE_PLAN_ID} — ${BANNER}`,
  }),
  row('stale-success', 'synthetic stale success', 'udp_dns', 'success', {
    sourceLabel: 'synthetic-probe-adapter-east — DEVELOPMENT FIXTURE — NOT LIVE DATA',
    provenance: provenance({
      sourceLabel: 'synthetic-lab-collector-west — DEVELOPMENT FIXTURE — NOT LIVE DATA',
      collectedAt: '2026-01-01T00:00:00.000Z',
      receivedAt: '2026-01-02T12:34:56.000Z',
      freshness: 'stale',
      completeness: 'complete',
      reason: 'synthetic stale window supplied by fixture',
    }),
  }),
  row('partial-success', 'synthetic partial success', 'tcp_dns', 'success', {
    provenance: provenance({
      freshness: 'fresh',
      completeness: 'partial',
      missing: ['lease-table', 'neighbor-snapshot'],
      reason: 'synthetic collector omitted optional tables',
    }),
  }),
  row('unknown-freshness-success', 'synthetic unknown-freshness success', 'http', 'success', {
    provenance: provenance({
      freshness: 'unknown',
      completeness: 'complete',
      collectedAt: null,
      receivedAt: null,
      reason: 'synthetic fixture supplied no timestamps',
    }),
  }),
  row('unavailable-completeness-success', 'synthetic completeness-unavailable success', 'icmp', 'success', {
    provenance: provenance({
      freshness: 'fresh',
      completeness: 'unavailable',
      missing: ['icmp-payload'],
      reason: 'synthetic completeness unavailable beside a success outcome',
    }),
  }),
  row('violation', 'synthetic violation', 'uci_readback', 'violation', {
    provenance: provenance({
      freshness: 'fresh',
      completeness: 'complete',
      reason: 'synthetic expected UCI value did not match',
    }),
  }),
  row('timeout', 'synthetic timeout', 'http', 'timeout'),
  row('refused', 'synthetic refused', 'tcp_connect', 'refused'),
  row('unreachable', 'synthetic unreachable', 'icmp', 'unreachable', {
    endpointLabel: null,
  }),
  row('dns-nx-domain', 'synthetic dns nxdomain', 'udp_dns', 'dns_nx_domain'),
  row('dns-server-failure', 'synthetic dns server failure', 'tcp_dns', 'dns_server_failure'),
  row('tls-failure', 'synthetic tls failure', 'http', 'tls_failure'),
  row('malformed-response', 'synthetic malformed response', 'netifd_state', 'malformed_response'),
  row('unsupported', 'synthetic unsupported', 'uci_readback', 'unsupported', {
    provenance: provenance({
      freshness: 'unknown',
      completeness: 'unavailable',
      missing: ['uci-schema'],
      reason: 'synthetic unsupported primitive on this profile',
    }),
  }),
  row('unavailable', 'synthetic unavailable', 'netifd_state', 'unavailable', {
    provenance: provenance({
      freshness: 'unknown',
      completeness: 'unavailable',
      missing: ['netifd.wireless'],
      reason: 'synthetic wireless object not collected',
    }),
  }),
  row('no-observation', 'synthetic null outcome', 'uci_readback', null, {
    endpointLabel: null,
    provenance: provenance({
      freshness: 'unknown',
      completeness: 'unavailable',
      collectedAt: null,
      receivedAt: null,
      missing: ['observation'],
      reason: 'synthetic: no observation stored',
    }),
  }),
  row('html-like', 'synthetic <img src=x onerror=alert(1)> html-like', 'http', 'timeout', {
    detail: HTML_LIKE_DETAIL,
    dependsOn: ['dep-html-like', LONG_DEPENDENCY_ID],
  }),
]);

export const ALL_OUTCOME_ROW_IDS: readonly string[] = Object.freeze(
  ALL_OUTCOME_ROWS.map((entry) => entry.id),
);

export type FixtureKey = 'all-outcomes' | 'empty' | 'no-plan' | 'unknown-selection';

export interface DemoFixture {
  readonly label: string;
  readonly planId: string | null;
  readonly graphVersionLabel: string | null;
  readonly rows: readonly AssuranceRow[];
  readonly initialSelectedId: string | null;
}

export const FIXTURES: Record<FixtureKey, DemoFixture> = {
  'all-outcomes': {
    label: 'All synthetic outcomes',
    planId: LARGE_PLAN_ID,
    graphVersionLabel: LARGE_GRAPH_VERSION,
    rows: ALL_OUTCOME_ROWS,
    initialSelectedId: null,
  },
  empty: {
    label: 'Empty observations',
    planId: LARGE_PLAN_ID,
    graphVersionLabel: LARGE_GRAPH_VERSION,
    rows: Object.freeze([]),
    initialSelectedId: null,
  },
  'no-plan': {
    label: 'Null plan and graph version',
    planId: null,
    graphVersionLabel: null,
    rows: ALL_OUTCOME_ROWS,
    initialSelectedId: null,
  },
  'unknown-selection': {
    label: 'Unknown selected id',
    planId: LARGE_PLAN_ID,
    graphVersionLabel: LARGE_GRAPH_VERSION,
    rows: ALL_OUTCOME_ROWS,
    initialSelectedId: MISSING_SELECTED_ID,
  },
};

export const FIXTURE_KEYS = Object.keys(FIXTURES) as FixtureKey[];
