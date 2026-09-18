import { describe, expect, it } from 'vitest';
import type { GraphEdge, GraphNode } from '../../contracts/presentation';
import { LAYOUT_OPTIONS, NODE_HEIGHT, NODE_WIDTH, readElkPositions, toElkGraph } from './graph-map';

function node(id: string): GraphNode {
  return { id, label: `label-${id}`, layer: 'L2', subtitle: 'sub' };
}

function edge(id: string, source: string, target: string): GraphEdge {
  return { id, source, target, label: id, layer: 'L2' };
}

describe('toElkGraph', () => {
  it('clones and sorts by ID without mutating caller arrays', () => {
    const nodes = Object.freeze([node('b'), node('a')]);
    const edges = Object.freeze([edge('z', 'b', 'a'), edge('m', 'a', 'b')]);
    const graph = toElkGraph(nodes, edges);
    expect(graph.children?.map((child) => child.id)).toEqual(['a', 'b']);
    expect(graph.edges?.map((item) => item.id)).toEqual(['m', 'z']);
    expect(graph.children?.[0]?.width).toBe(NODE_WIDTH);
    expect(graph.children?.[0]?.height).toBe(NODE_HEIGHT);
    expect(graph.layoutOptions).toMatchObject(LAYOUT_OPTIONS);
    expect(nodes.map((item) => item.id)).toEqual(['b', 'a']);
    expect(edges.map((item) => item.id)).toEqual(['z', 'm']);
  });
});

describe('readElkPositions', () => {
  it('requires finite positions and preserves IDs', () => {
    expect(
      readElkPositions({
        id: 'root',
        children: [
          { id: 'a', x: 10, y: 20 },
          { id: 'b', x: 40, y: 0 },
        ],
      }),
    ).toEqual({
      ok: true,
      positions: [
        { id: 'a', x: 10, y: 20 },
        { id: 'b', x: 40, y: 0 },
      ],
    });
    expect(
      readElkPositions({
        id: 'root',
        children: [{ id: 'bad', x: Number.NaN, y: 1 }],
      }).ok,
    ).toBe(false);
  });
});
