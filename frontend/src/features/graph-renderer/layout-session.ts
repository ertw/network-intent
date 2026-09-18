import type { GraphEdge, GraphNode } from '../../contracts/presentation';
import {
  readElkPositions,
  toElkGraph,
  type LayoutFunction,
  type NodePosition,
} from './graph-map';

export type LayoutResult =
  | { status: 'ready'; generation: number; positions: NodePosition[] }
  | { status: 'error'; generation: number; message: string }
  | { status: 'stale'; generation: number };

export class LayoutSession {
  private generation = 0;
  private disposed = false;
  private readonly layout: LayoutFunction;
  private readonly emit: (result: Exclude<LayoutResult, { status: 'stale' }>) => void;

  constructor(
    layout: LayoutFunction,
    emit: (result: Exclude<LayoutResult, { status: 'stale' }>) => void,
  ) {
    this.layout = layout;
    this.emit = emit;
  }

  invalidate(): number {
    this.generation += 1;
    return this.generation;
  }

  dispose(): void {
    this.disposed = true;
    this.generation += 1;
  }

  isCurrent(generation: number): boolean {
    return !this.disposed && generation === this.generation;
  }

  request(nodes: readonly GraphNode[], edges: readonly GraphEdge[]): number {
    const generation = this.invalidate();
    const graph = toElkGraph(nodes, edges);
    void this.layout(graph).then(
      (laidOut) => {
        if (!this.isCurrent(generation)) {
          return;
        }
        const positions = readElkPositions(laidOut);
        if (!positions.ok) {
          this.emit({ status: 'error', generation, message: positions.message });
          return;
        }
        this.emit({ status: 'ready', generation, positions: positions.positions });
      },
      (error: unknown) => {
        if (!this.isCurrent(generation)) {
          return;
        }
        const message = error instanceof Error ? error.message : 'Graph layout failed';
        this.emit({ status: 'error', generation, message });
      },
    );
    return generation;
  }
}
