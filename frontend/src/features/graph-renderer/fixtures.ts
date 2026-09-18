import type { GraphEdge, GraphNode, Layer } from '../../contracts/presentation';

export const BANNER = 'DEVELOPMENT FIXTURE — NOT LIVE DATA';
export const HTML_LIKE_LABEL =
  '<img src=x onerror=alert(1)> synthetic node — DEVELOPMENT FIXTURE — NOT LIVE DATA';
export const HTML_LIKE_EDGE =
  '<script>document.title="pwned"</script> synthetic edge — DEVELOPMENT FIXTURE — NOT LIVE DATA';
export const LONG_LABEL =
  'synthetic-long-label-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

export type FixtureKey =
  | 'empty'
  | 'single'
  | 'disconnected'
  | 'chain'
  | 'cycle'
  | 'parallel'
  | 'layers'
  | 'html-long'
  | 'duplicate-nodes'
  | 'duplicate-edges'
  | 'dangling'
  | 'overlapping';

function freezeNode(node: GraphNode): GraphNode {
  return Object.freeze({ ...node });
}

function freezeEdge(edge: GraphEdge): GraphEdge {
  return Object.freeze({ ...edge });
}

function node(id: string, label: string, layer: Layer, subtitle: string | null = null): GraphNode {
  return freezeNode({ id, label, layer, subtitle });
}

function edge(
  id: string,
  source: string,
  target: string,
  layer: Layer,
  label: string | null = null,
): GraphEdge {
  return freezeEdge({ id, source, target, label, layer });
}

export interface GraphFixture {
  key: FixtureKey;
  label: string;
  nodes: readonly GraphNode[];
  edges: readonly GraphEdge[];
  initialSelectedId: string | null;
}

export const FIXTURES: Record<FixtureKey, GraphFixture> = {
  empty: {
    key: 'empty',
    label: 'Synthetic: empty graph',
    nodes: Object.freeze([]),
    edges: Object.freeze([]),
    initialSelectedId: null,
  },
  single: {
    key: 'single',
    label: 'Synthetic: single node',
    nodes: Object.freeze([node('only', 'Only node', 'L3', 'single subtitle')]),
    edges: Object.freeze([]),
    initialSelectedId: null,
  },
  disconnected: {
    key: 'disconnected',
    label: 'Synthetic: disconnected nodes',
    nodes: Object.freeze([
      node('left', 'Left island', 'L2'),
      node('right', 'Right island', 'L4'),
    ]),
    edges: Object.freeze([]),
    initialSelectedId: null,
  },
  chain: {
    key: 'chain',
    label: 'Synthetic: directed chain',
    nodes: Object.freeze([
      node('n1', 'Chain one', 'L3'),
      node('n2', 'Chain two', 'L3', 'middle'),
      node('n3', 'Chain three', 'L3'),
    ]),
    edges: Object.freeze([
      edge('e1', 'n1', 'n2', 'L3', 'one to two'),
      edge('e2', 'n2', 'n3', 'L3', 'two to three'),
    ]),
    initialSelectedId: null,
  },
  cycle: {
    key: 'cycle',
    label: 'Synthetic: cycle',
    nodes: Object.freeze([
      node('c1', 'Cycle A', 'L2'),
      node('c2', 'Cycle B', 'L2'),
      node('c3', 'Cycle C', 'L2'),
    ]),
    edges: Object.freeze([
      edge('c-ab', 'c1', 'c2', 'L2', 'A to B'),
      edge('c-bc', 'c2', 'c3', 'L2', 'B to C'),
      edge('c-ca', 'c3', 'c1', 'L2', 'C to A'),
    ]),
    initialSelectedId: null,
  },
  parallel: {
    key: 'parallel',
    label: 'Synthetic: parallel edges',
    nodes: Object.freeze([
      node('p1', 'Parallel source', 'L1'),
      node('p2', 'Parallel target', 'L7'),
    ]),
    edges: Object.freeze([
      edge('p-a', 'p1', 'p2', 'L1', 'parallel A'),
      edge('p-b', 'p1', 'p2', 'L7', 'parallel B'),
    ]),
    initialSelectedId: null,
  },
  layers: {
    key: 'layers',
    label: 'Synthetic: all layer tags',
    nodes: Object.freeze([
      node('l1', 'Layer L1', 'L1'),
      node('l2', 'Layer L2', 'L2'),
      node('l3', 'Layer L3', 'L3'),
      node('l4', 'Layer L4', 'L4'),
      node('l7', 'Layer L7', 'L7'),
    ]),
    edges: Object.freeze([
      edge('l1-l2', 'l1', 'l2', 'L1', 'L1 tag'),
      edge('l2-l3', 'l2', 'l3', 'L2', 'L2 tag'),
      edge('l3-l4', 'l3', 'l4', 'L3', 'L3 tag'),
      edge('l4-l7', 'l4', 'l7', 'L4', 'L4 tag'),
    ]),
    initialSelectedId: null,
  },
  'html-long': {
    key: 'html-long',
    label: 'Synthetic: long and HTML-like labels',
    nodes: Object.freeze([
      node('html', HTML_LIKE_LABEL, 'L4', LONG_LABEL),
      node('long', LONG_LABEL, 'L4', HTML_LIKE_LABEL),
    ]),
    edges: Object.freeze([edge('html-edge', 'html', 'long', 'L4', HTML_LIKE_EDGE)]),
    initialSelectedId: null,
  },
  'duplicate-nodes': {
    key: 'duplicate-nodes',
    label: 'Synthetic: duplicate node IDs',
    nodes: Object.freeze([node('dup', 'First dup', 'L3'), node('dup', 'Second dup', 'L3')]),
    edges: Object.freeze([]),
    initialSelectedId: null,
  },
  'duplicate-edges': {
    key: 'duplicate-edges',
    label: 'Synthetic: duplicate edge IDs',
    nodes: Object.freeze([node('a', 'A', 'L3'), node('b', 'B', 'L3')]),
    edges: Object.freeze([
      edge('same', 'a', 'b', 'L3', 'first'),
      edge('same', 'a', 'b', 'L3', 'second'),
    ]),
    initialSelectedId: null,
  },
  dangling: {
    key: 'dangling',
    label: 'Synthetic: dangling endpoints',
    nodes: Object.freeze([node('present', 'Present', 'L3')]),
    edges: Object.freeze([edge('ghost', 'present', 'absent', 'L3', 'dangling')]),
    initialSelectedId: null,
  },
  overlapping: {
    key: 'overlapping',
    label: 'Synthetic: overlapping node/edge IDs',
    nodes: Object.freeze([node('shared', 'Shared node', 'L3'), node('other', 'Other', 'L3')]),
    edges: Object.freeze([edge('shared', 'shared', 'other', 'L3', 'shared id')]),
    initialSelectedId: null,
  },
};

export const FIXTURE_KEYS = Object.keys(FIXTURES) as FixtureKey[];
export const DEFAULT_FIXTURE = FIXTURES.chain;
export const MISSING_SELECTED_ID = 'missing-graph-id';
