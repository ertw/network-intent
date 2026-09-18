import type { ElkNode } from 'elkjs/lib/elk-api';
import ELK from 'elkjs/lib/elk.bundled.js';
import type { LayoutFunction } from './graph-map';

export function createBundledLayout(): LayoutFunction {
  const elk = new ELK();
  return (graph: ElkNode) => elk.layout(graph);
}
