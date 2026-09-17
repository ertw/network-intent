/** Frozen presentation contract v1. Not a backend wire schema or authority. */
export type Layer = 'L1' | 'L2' | 'L3' | 'L4' | 'L7';
export type StateView = 'intent' | 'device_configuration' | 'operational' | 'compare';
export type EditMode = 'view' | 'edit';
export type Freshness = 'fresh' | 'stale' | 'unknown';
export type Completeness = 'complete' | 'partial' | 'unavailable';
export type Datastore = 'committed' | 'session_staging' | 'configuration_in_use' | 'operational';
export interface ViewSelection {
  primaryLayer: Layer;
  overlays: readonly Layer[];
  combinedOverview: boolean;
  stateView: StateView;
  assuranceOverlay: boolean;
  editMode: EditMode;
}
export interface ViewControlsProps {
  value: ViewSelection;
  onChange: (next: ViewSelection) => void;
}
/** Labels and ISO timestamps are already supplied by an authoritative adapter. */
export interface Provenance {
  sourceLabel: string;
  collectedAt: string | null;
  receivedAt: string | null;
  freshness: Freshness;
  completeness: Completeness;
  missing: readonly string[];
  reason: string | null;
}
export type DisplayField =
  | { id: string; label: string; kind: 'scalar'; value: string }
  | { id: string; label: string; kind: 'ordered_list'; values: readonly string[] }
  | { id: string; label: string; kind: 'redacted' }
  | { id: string; label: string; kind: 'unknown'; reason: string };
export interface DetailGroup {
  id: string;
  label: string;
  fields: readonly DisplayField[];
}
export interface SnapshotPanelProps {
  title: string;
  snapshotId: string | null;
  datastore: Datastore;
  provenance: Provenance;
  groups: readonly DetailGroup[];
}
export type Primitive = 'tcp_connect' | 'udp_dns' | 'tcp_dns' | 'http' | 'icmp' | 'uci_readback' | 'netifd_state';
export type Outcome = 'success' | 'violation' | 'timeout' | 'refused' | 'unreachable'
  | 'dns_nx_domain' | 'dns_server_failure' | 'tls_failure' | 'malformed_response'
  | 'unsupported' | 'unavailable';
export interface AssuranceRow {
  id: string;
  label: string;
  primitive: Primitive;
  outcome: Outcome | null;
  detail: string;
  provenance: Provenance;
  sourceLabel: string;
  endpointLabel: string | null;
  dependsOn: readonly string[];
}
export interface AssurancePanelProps {
  planId: string | null;
  graphVersionLabel: string | null;
  rows: readonly AssuranceRow[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}
/** CodeMirror offsets are UTF-16 editor positions, NEVER compiler source spans. */
export interface EditorSelection { anchor: number; head: number }
export interface SourceEditorProps {
  source: string;
  editable: boolean;
  selection: EditorSelection | null;
  onChange: (source: string) => void;
  onSelectionChange: (selection: EditorSelection) => void;
  onHistoryRequest: (direction: 'undo' | 'redo') => void;
}
export interface GraphNode {
  id: string;
  label: string;
  layer: Layer;
  subtitle: string | null;
}
export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label: string | null;
  layer: Layer;
}
export interface GraphRendererProps {
  nodes: readonly GraphNode[];
  edges: readonly GraphEdge[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
}
/** Provided changes only: this contract does not authorize calculating a diff. */
export interface ChangeEntry {
  id: string;
  label: string;
  kind: 'added' | 'removed' | 'modified' | 'blocked';
  before: readonly DisplayField[];
  after: readonly DisplayField[];
  explanation: string | null;
}
export interface ChangesPanelProps {
  changes: readonly ChangeEntry[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}
