import { describe, expect, it } from 'vitest';
import type { GraphEdge, GraphNode } from '../../contracts/presentation';
import { cloneEdges, cloneNodes, issueMessage, validateGraph } from './validate';

function node(id: string, label = id): GraphNode {
  return { id, label, layer: 'L3', subtitle: null };
}

function edge(id: string, source: string, target: string): GraphEdge {
  return { id, source, target, label: null, layer: 'L3' };
}

describe('validateGraph', () => {
  it('accepts a simple directed chain', () => {
    expect(
      validateGraph([node('a'), node('b')], [edge('a-b', 'a', 'b')]),
    ).toEqual({ ok: true });
  });

  it('reports duplicate, overlapping, and dangling IDs without dropping input', () => {
    const nodes = [node('dup'), node('dup'), node('shared')];
    const edges = [
      edge('e1', 'dup', 'missing'),
      edge('e1', 'nobody', 'dup'),
      edge('shared', 'dup', 'dup'),
    ];
    const result = validateGraph(nodes, edges);
    expect(result.ok).toBe(false);
    if (result.ok) {
      return;
    }
    expect(result.issues).toEqual([
      { code: 'duplicate-node-id', id: 'dup' },
      { code: 'duplicate-edge-id', id: 'e1' },
      { code: 'overlapping-id', id: 'shared' },
      { code: 'dangling-target', edgeId: 'e1', target: 'missing' },
      { code: 'dangling-source', edgeId: 'e1', source: 'nobody' },
    ]);
    expect(issueMessage(result.issues[0]!)).toContain('dup');
    expect(nodes).toHaveLength(3);
    expect(edges).toHaveLength(3);
  });

  it('does not mutate cloned caller arrays', () => {
    const nodes = Object.freeze([node('a')]);
    const edges = Object.freeze([edge('loop', 'a', 'a')]);
    expect(validateGraph(nodes, edges)).toEqual({ ok: true });
    const copiedNodes = cloneNodes(nodes);
    const copiedEdges = cloneEdges(edges);
    copiedNodes[0]!.label = 'changed';
    copiedEdges[0]!.label = 'changed';
    expect(nodes[0]?.label).toBe('a');
    expect(edges[0]?.label).toBeNull();
  });
});
