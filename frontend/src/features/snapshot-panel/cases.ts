import type { SnapshotPanelProps } from '../../contracts/presentation';

export interface SnapshotCase {
  id: string;
  label: string;
  props: SnapshotPanelProps;
}

const HTML_LIKE =
  '<img src="x" onerror="alert(1)"> <script>document.title="pwned"</script>';

const LONG_BANNER = Array.from({ length: 40 }, (_, index) => {
  const line = String(index + 1).padStart(2, '0');
  return `synthetic-line-${line}  keep  two  spaces  ${'x'.repeat(24)}`;
}).join('\n');

export const SNAPSHOT_CASES: readonly SnapshotCase[] = [
  {
    id: 'committed-complete',
    label: 'Synthetic: committed configuration (complete, fresh)',
    props: {
      title: 'Synthetic committed snapshot',
      snapshotId: 'snap-committed-synthetic-001',
      datastore: 'committed',
      provenance: {
        sourceLabel: 'Synthetic lab collector',
        collectedAt: '2026-09-17T16:01:02.000Z',
        receivedAt: '2026-09-17T16:01:03.000Z',
        freshness: 'fresh',
        completeness: 'complete',
        missing: [],
        reason: null,
      },
      groups: [
        {
          id: 'system',
          label: 'System',
          fields: [
            { id: 'hostname', label: 'Hostname', kind: 'scalar', value: 'synthetic-lab' },
            { id: 'metric', label: 'Metric', kind: 'scalar', value: '0' },
            { id: 'disabled', label: 'Disabled', kind: 'scalar', value: 'false' },
            { id: 'vlan', label: 'VLAN', kind: 'scalar', value: '007' },
            { id: 'note', label: 'Note', kind: 'scalar', value: 'alpha  beta' },
          ],
        },
        {
          id: 'network',
          label: 'Network',
          fields: [
            {
              id: 'interfaces',
              label: 'Interfaces',
              kind: 'ordered_list',
              values: ['eth0', 'eth1', 'eth0', 'br-lan'],
            },
            {
              id: 'tags',
              label: 'Tags',
              kind: 'ordered_list',
              values: ['', 'dup', 'dup', ''],
            },
          ],
        },
      ],
    },
  },
  {
    id: 'session-staging-partial',
    label: 'Synthetic: session staging (partial, stale)',
    props: {
      title: 'Synthetic session staging snapshot',
      snapshotId: 'snap-staging-synthetic-002',
      datastore: 'session_staging',
      provenance: {
        sourceLabel: 'Synthetic staging buffer',
        collectedAt: '2026-09-16T08:00:00.000Z',
        receivedAt: '2026-09-16T09:30:00.000Z',
        freshness: 'stale',
        completeness: 'partial',
        missing: ['firewall.defaults', 'dhcp.lan.leasetime'],
        reason: null,
      },
      groups: [
        {
          id: 'session',
          label: 'Session edits',
          fields: [
            {
              id: 'proposed-hostname',
              label: 'Proposed hostname',
              kind: 'scalar',
              value: 'synthetic-staging',
            },
          ],
        },
      ],
    },
  },
  {
    id: 'configuration-in-use-unavailable',
    label: 'Synthetic: configuration in use (unavailable)',
    props: {
      title: 'Synthetic configuration in use snapshot',
      snapshotId: null,
      datastore: 'configuration_in_use',
      provenance: {
        sourceLabel: 'Synthetic running-config probe',
        collectedAt: null,
        receivedAt: null,
        freshness: 'unknown',
        completeness: 'unavailable',
        missing: ['entire configuration_in_use datastore'],
        reason: 'Synthetic collector reported this datastore unavailable',
      },
      groups: [],
    },
  },
  {
    id: 'operational-complete',
    label: 'Synthetic: operational state (complete, fresh)',
    props: {
      title: 'Synthetic operational snapshot',
      snapshotId: 'snap-operational-synthetic-004',
      datastore: 'operational',
      provenance: {
        sourceLabel: 'Synthetic netifd readback',
        collectedAt: '2026-09-17T16:05:00.000Z',
        receivedAt: '2026-09-17T16:05:01.000Z',
        freshness: 'fresh',
        completeness: 'complete',
        missing: [],
        reason: null,
      },
      groups: [
        {
          id: 'link',
          label: 'Link',
          fields: [
            { id: 'carrier', label: 'Carrier', kind: 'scalar', value: 'true' },
            { id: 'speed', label: 'Speed', kind: 'scalar', value: '1000' },
            {
              id: 'addresses',
              label: 'Addresses',
              kind: 'ordered_list',
              values: ['192.0.2.10/24', '192.0.2.10/24'],
            },
          ],
        },
      ],
    },
  },
  {
    id: 'redacted-unknown',
    label: 'Synthetic: redacted, unknown, empty scalar, empty list',
    props: {
      title: 'Synthetic redacted and unknown fields',
      snapshotId: 'snap-redacted-synthetic-005',
      datastore: 'committed',
      provenance: {
        sourceLabel: 'Synthetic redaction fixture',
        collectedAt: '2026-09-17T12:00:00.000Z',
        receivedAt: '2026-09-17T12:00:02.000Z',
        freshness: 'fresh',
        completeness: 'complete',
        missing: [],
        reason: null,
      },
      groups: [
        {
          id: 'secrets-and-gaps',
          label: 'Field kinds',
          fields: [
            { id: 'empty-scalar', label: 'Empty description', kind: 'scalar', value: '' },
            { id: 'empty-list', label: 'DNS servers', kind: 'ordered_list', values: [] },
            { id: 'community-string', label: 'Community string', kind: 'redacted' },
            {
              id: 'uptime',
              label: 'Uptime',
              kind: 'unknown',
              reason: 'Synthetic collector did not return uptime',
            },
          ],
        },
      ],
    },
  },
  {
    id: 'empty-groups-operational-partial',
    label: 'Synthetic: operational empty groups (partial)',
    props: {
      title: 'Synthetic operational snapshot with no groups',
      snapshotId: 'snap-operational-empty-synthetic-006',
      datastore: 'operational',
      provenance: {
        sourceLabel: 'Synthetic operational probe',
        collectedAt: '2026-09-17T16:09:00.000Z',
        receivedAt: '2026-09-17T16:09:01.000Z',
        freshness: 'stale',
        completeness: 'partial',
        missing: ['link.carrier', 'addresses'],
        reason: 'Synthetic collector returned no groups',
      },
      groups: [],
    },
  },
  {
    id: 'empty-groups-committed-complete',
    label: 'Synthetic: committed empty groups (complete)',
    props: {
      title: 'Synthetic committed snapshot with no groups',
      snapshotId: 'snap-committed-empty-synthetic-007',
      datastore: 'committed',
      provenance: {
        sourceLabel: 'Synthetic empty committed export',
        collectedAt: '2026-09-17T11:00:00.000Z',
        receivedAt: '2026-09-17T11:00:00.000Z',
        freshness: 'fresh',
        completeness: 'complete',
        missing: [],
        reason: null,
      },
      groups: [],
    },
  },
  {
    id: 'long-text-html',
    label: 'Synthetic: long text and HTML-like values',
    props: {
      title: 'Synthetic long text and HTML-like values',
      snapshotId: 'snap-long-html-synthetic-008',
      datastore: 'session_staging',
      provenance: {
        sourceLabel: `Synthetic collector ${HTML_LIKE}`,
        collectedAt: '2026-09-17T15:00:00.000Z',
        receivedAt: '2026-09-17T15:00:05.000Z',
        freshness: 'unknown',
        completeness: 'partial',
        missing: [`missing-leaf ${HTML_LIKE}`],
        reason: `Partial synthetic dump ${HTML_LIKE}`,
      },
      groups: [
        {
          id: 'banner',
          label: 'Banner',
          fields: [
            { id: 'long-banner', label: 'Banner text', kind: 'scalar', value: LONG_BANNER },
            { id: 'html-scalar', label: 'HTML-like scalar', kind: 'scalar', value: HTML_LIKE },
            {
              id: 'html-list',
              label: 'HTML-like list',
              kind: 'ordered_list',
              values: [HTML_LIKE, HTML_LIKE],
            },
            {
              id: 'html-unknown',
              label: 'HTML-like unknown',
              kind: 'unknown',
              reason: HTML_LIKE,
            },
          ],
        },
      ],
    },
  },
  {
    id: 'null-snapshot-unknown-time',
    label: 'Synthetic: no snapshot id, unknown timestamps',
    props: {
      title: 'Synthetic snapshot without an identifier',
      snapshotId: null,
      datastore: 'committed',
      provenance: {
        sourceLabel: 'Synthetic unlabeled dump',
        collectedAt: null,
        receivedAt: null,
        freshness: 'unknown',
        completeness: 'complete',
        missing: [],
        reason: null,
      },
      groups: [
        {
          id: 'only',
          label: 'Only group',
          fields: [{ id: 'present', label: 'Present', kind: 'scalar', value: 'yes' }],
        },
      ],
    },
  },
];

const firstSnapshotCase = SNAPSHOT_CASES[0];
if (!firstSnapshotCase) {
  throw new Error('SNAPSHOT_CASES must contain at least one synthetic case');
}
export const DEFAULT_SNAPSHOT_CASE: SnapshotCase = firstSnapshotCase;

export function snapshotCaseById(id: string): SnapshotCase {
  return SNAPSHOT_CASES.find((item) => item.id === id) ?? DEFAULT_SNAPSHOT_CASE;
}
