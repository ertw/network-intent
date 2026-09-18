import { describe, expect, it } from 'vitest';
import type { ElkNode } from 'elkjs/lib/elk-api';
import type { GraphNode } from '../../contracts/presentation';
import { LayoutSession, type LayoutResult } from './layout-session';

function node(id: string): GraphNode {
  return { id, label: id, layer: 'L3', subtitle: null };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

describe('LayoutSession', () => {
  it('keeps only the latest generation when B finishes before A', async () => {
    const first = deferred<ElkNode>();
    const second = deferred<ElkNode>();
    let call = 0;
    const emitted: LayoutResult[] = [];
    const session = new LayoutSession(
      () => {
        call += 1;
        return call === 1 ? first.promise : second.promise;
      },
      (result) => {
        emitted.push(result);
      },
    );

    session.request([node('a')], []);
    session.request([node('b')], []);
    second.resolve({ id: 'root', children: [{ id: 'b', x: 2, y: 3 }] });
    await Promise.resolve();
    first.resolve({ id: 'root', children: [{ id: 'a', x: 9, y: 9 }] });
    await Promise.resolve();

    expect(emitted).toEqual([
      { status: 'ready', generation: 2, positions: [{ id: 'b', x: 2, y: 3 }] },
    ]);
  });

  it('emits an error when the current layout rejects and ignores a later stale success', async () => {
    const first = deferred<ElkNode>();
    const second = deferred<ElkNode>();
    let call = 0;
    const emitted: LayoutResult[] = [];
    const session = new LayoutSession(
      () => {
        call += 1;
        return call === 1 ? first.promise : second.promise;
      },
      (result) => {
        emitted.push(result);
      },
    );

    session.request([node('a')], []);
    session.request([node('b')], []);
    second.reject(new Error('synthetic layout failure'));
    await Promise.resolve();
    first.resolve({ id: 'root', children: [{ id: 'a', x: 1, y: 1 }] });
    await Promise.resolve();

    expect(emitted).toEqual([
      { status: 'error', generation: 2, message: 'synthetic layout failure' },
    ]);
  });

  it('does not emit after dispose', async () => {
    const pending = deferred<ElkNode>();
    const emitted: LayoutResult[] = [];
    const session = new LayoutSession(
      () => pending.promise,
      (result) => {
        emitted.push(result);
      },
    );
    session.request([node('a')], []);
    session.dispose();
    pending.resolve({ id: 'root', children: [{ id: 'a', x: 1, y: 1 }] });
    await Promise.resolve();
    expect(emitted).toEqual([]);
  });
});
