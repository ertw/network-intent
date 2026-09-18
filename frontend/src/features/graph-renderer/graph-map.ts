import type { ElkNode } from 'elkjs/lib/elk-api';
import type { GraphEdge, GraphNode, Layer } from '../../contracts/presentation';
import { byId, cloneEdges, cloneNodes } from './validate';

export const NODE_WIDTH = 200;
export const NODE_HEIGHT = 80;

export const LAYOUT_OPTIONS: Readonly<Record<string, string>> = {
  'elk.algorithm': 'layered',
  'elk.direction': 'RIGHT',
  'elk.layered.spacing.nodeNodeBetweenLayers': '80',
  'elk.spacing.nodeNode': '40',
  'elk.layered.mergeEdges': 'false',
};

export type LayoutFunction = (graph: ElkNode) => Promise<ElkNode>;

export interface NodePosition {
  id: string;
  x: number;
  y: number;
}

export function toElkGraph(nodes: readonly GraphNode[], edges: readonly GraphEdge[]): ElkNode {
  const layoutNodes = cloneNodes(nodes).sort(byId);
  const layoutEdges = cloneEdges(edges).sort(byId);
  return {
    id: 'root',
    layoutOptions: { ...LAYOUT_OPTIONS },
    children: layoutNodes.map((node) => ({
      id: node.id,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
      labels: [{ text: node.label }],
      ports: [
        {
          id: `${node.id}#in`,
          layoutOptions: { 'port.side': 'WEST', 'port.index': '0' },
        },
        {
          id: `${node.id}#out`,
          layoutOptions: { 'port.side': 'EAST', 'port.index': '0' },
        },
      ],
    })),
    edges: layoutEdges.map((edge) => ({
      id: edge.id,
      sources: [`${edge.source}#out`],
      targets: [`${edge.target}#in`],
      labels: edge.label === null ? [] : [{ text: edge.label }],
    })),
  };
}

export function readElkPositions(laidOut: ElkNode):
  | { ok: true; positions: NodePosition[] }
  | { ok: false; message: string } {
  const positions: NodePosition[] = [];
  for (const child of laidOut.children ?? []) {
    const x = child.x;
    const y = child.y;
    if (!Number.isFinite(x) || !Number.isFinite(y) || x === undefined || y === undefined) {
      return {
        ok: false,
        message: `Layout returned a non-finite position for node ${child.id}`,
      };
    }
    positions.push({ id: child.id, x, y });
  }
  return { ok: true, positions };
}

export interface FlowNodeInput {
  id: string;
  label: string;
  subtitle: string | null;
  layer: Layer;
  x: number;
  y: number;
  selected: boolean;
}

export interface FlowEdgeInput {
  id: string;
  source: string;
  target: string;
  label: string | null;
  layer: Layer;
  selected: boolean;
}

export function toFlowInputs(
  nodes: readonly GraphNode[],
  edges: readonly GraphEdge[],
  positions: readonly NodePosition[],
  selectedId: string | null,
): { nodes: FlowNodeInput[]; edges: FlowEdgeInput[] } {
  const byNodeId = new Map(positions.map((entry) => [entry.id, entry]));
  return {
    nodes: nodes.map((node) => {
      const position = byNodeId.get(node.id);
      return {
        id: node.id,
        label: node.label,
        subtitle: node.subtitle,
        layer: node.layer,
        x: position?.x ?? 0,
        y: position?.y ?? 0,
        selected: selectedId === node.id,
      };
    }),
    edges: edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      label: edge.label,
      layer: edge.layer,
      selected: selectedId === edge.id,
    })),
  };
}
