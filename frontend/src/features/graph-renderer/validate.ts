import type { GraphEdge, GraphNode } from '../../contracts/presentation';

export type GraphIssue =
  | { code: 'duplicate-node-id'; id: string }
  | { code: 'duplicate-edge-id'; id: string }
  | { code: 'overlapping-id'; id: string }
  | { code: 'dangling-source'; edgeId: string; source: string }
  | { code: 'dangling-target'; edgeId: string; target: string };

export function issueMessage(issue: GraphIssue): string {
  switch (issue.code) {
    case 'duplicate-node-id':
      return `Duplicate node ID: ${issue.id}`;
    case 'duplicate-edge-id':
      return `Duplicate edge ID: ${issue.id}`;
    case 'overlapping-id':
      return `Node and edge share ID: ${issue.id}`;
    case 'dangling-source':
      return `Edge ${issue.edgeId} source is missing: ${issue.source}`;
    case 'dangling-target':
      return `Edge ${issue.edgeId} target is missing: ${issue.target}`;
  }
}

export function validateGraph(
  nodes: readonly GraphNode[],
  edges: readonly GraphEdge[],
): { ok: true } | { ok: false; issues: GraphIssue[] } {
  const issues: GraphIssue[] = [];
  const nodeIds = new Set<string>();
  const duplicateNodes = new Set<string>();
  const edgeIds = new Set<string>();
  const duplicateEdges = new Set<string>();

  for (const node of nodes) {
    if (nodeIds.has(node.id)) {
      duplicateNodes.add(node.id);
    }
    nodeIds.add(node.id);
  }
  for (const id of duplicateNodes) {
    issues.push({ code: 'duplicate-node-id', id });
  }

  for (const edge of edges) {
    if (edgeIds.has(edge.id)) {
      duplicateEdges.add(edge.id);
    }
    edgeIds.add(edge.id);
  }
  for (const id of duplicateEdges) {
    issues.push({ code: 'duplicate-edge-id', id });
  }

  for (const id of nodeIds) {
    if (edgeIds.has(id)) {
      issues.push({ code: 'overlapping-id', id });
    }
  }

  for (const edge of edges) {
    if (!nodeIds.has(edge.source)) {
      issues.push({ code: 'dangling-source', edgeId: edge.id, source: edge.source });
    }
    if (!nodeIds.has(edge.target)) {
      issues.push({ code: 'dangling-target', edgeId: edge.id, target: edge.target });
    }
  }

  if (issues.length > 0) {
    return { ok: false, issues };
  }
  return { ok: true };
}

export function cloneNodes(nodes: readonly GraphNode[]): GraphNode[] {
  return nodes.map((node) => ({ ...node }));
}

export function cloneEdges(edges: readonly GraphEdge[]): GraphEdge[] {
  return edges.map((edge) => ({ ...edge }));
}

export function byId<T extends { id: string }>(left: T, right: T): number {
  return left.id < right.id ? -1 : left.id > right.id ? 1 : 0;
}
