import { describe, expect, it } from 'vitest';
import { FIXTURES, HTML_LIKE_LABEL } from './fixtures';
import { validateGraph } from './validate';

describe('graph-renderer fixtures', () => {
  it('covers empty, chain, cycle, parallel edges, and all five layer tags', () => {
    expect(FIXTURES.empty.nodes).toHaveLength(0);
    expect(FIXTURES.chain.nodes.map((node) => node.id)).toEqual(['n1', 'n2', 'n3']);
    expect(FIXTURES.cycle.edges).toHaveLength(3);
    expect(FIXTURES.parallel.edges).toHaveLength(2);
    expect(FIXTURES.layers.nodes.map((node) => node.layer)).toEqual([
      'L1',
      'L2',
      'L3',
      'L4',
      'L7',
    ]);
  });

  it('keeps HTML-like labels as literal fixture text and marks invalid ID cases', () => {
    expect(FIXTURES['html-long'].nodes[0]?.label).toBe(HTML_LIKE_LABEL);
    expect(validateGraph(FIXTURES['duplicate-nodes'].nodes, FIXTURES['duplicate-nodes'].edges).ok).toBe(
      false,
    );
    expect(validateGraph(FIXTURES.dangling.nodes, FIXTURES.dangling.edges).ok).toBe(false);
    expect(validateGraph(FIXTURES.overlapping.nodes, FIXTURES.overlapping.edges).ok).toBe(false);
    expect(validateGraph(FIXTURES.chain.nodes, FIXTURES.chain.edges).ok).toBe(true);
  });
});
