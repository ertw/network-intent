import { describe, expect, it } from 'vitest';
import { fixtureModules } from './discover';
import {
  discoverFixtureNames,
  fixtureNameFromModulePath,
  parseFixtureRoute,
  resolveFixtureRoute,
} from './fixtures';

const futureNames = [
  'assurance-panel',
  'changes-panel',
  'graph-renderer',
  'snapshot-panel',
  'source-editor',
  'view-controls',
] as const;

describe('fixtureNameFromModulePath', () => {
  it('reads the feature folder from a Vite glob key', () => {
    expect(fixtureNameFromModulePath('../features/view-controls/Demo.svelte')).toBe(
      'view-controls',
    );
    expect(fixtureNameFromModulePath('./features/harness-example/Demo.svelte')).toBe(
      'harness-example',
    );
    expect(
      fixtureNameFromModulePath('/abs/src/features/snapshot-panel/Demo.svelte'),
    ).toBe('snapshot-panel');
  });

  it('rejects nested or non-Demo paths', () => {
    expect(fixtureNameFromModulePath('../features/view-controls/Other.svelte')).toBeNull();
    expect(
      fixtureNameFromModulePath('../features/view-controls/nested/Demo.svelte'),
    ).toBeNull();
    expect(fixtureNameFromModulePath('../features/Demo.svelte')).toBeNull();
  });
});

describe('parseFixtureRoute', () => {
  it('maps /#/folder-name hashes to fixture names', () => {
    expect(parseFixtureRoute('#/view-controls')).toBe('view-controls');
    expect(parseFixtureRoute('#/harness-example')).toBe('harness-example');
  });

  it('treats empty hashes as no selection', () => {
    expect(parseFixtureRoute('')).toBeNull();
    expect(parseFixtureRoute('#')).toBeNull();
    expect(parseFixtureRoute('#/')).toBeNull();
  });

  it('keeps unknown or nested paths as unresolved names', () => {
    expect(parseFixtureRoute('#/not-a-fixture')).toBe('not-a-fixture');
    expect(parseFixtureRoute('#/foo/bar')).toBe('foo/bar');
  });
});

describe('resolveFixtureRoute', () => {
  it('resolves multiple fixture names without a shared registry', () => {
    const names = ['harness-example', 'harness-discovery', ...futureNames];

    expect(resolveFixtureRoute('#/', names)).toEqual({ status: 'idle', name: null });
    expect(resolveFixtureRoute('#/harness-example', names)).toEqual({
      status: 'found',
      name: 'harness-example',
    });
    expect(resolveFixtureRoute('#/view-controls', names)).toEqual({
      status: 'found',
      name: 'view-controls',
    });
    expect(resolveFixtureRoute('#/snapshot-panel', names)).toEqual({
      status: 'found',
      name: 'snapshot-panel',
    });
    expect(resolveFixtureRoute('#/graph-renderer', names)).toEqual({
      status: 'found',
      name: 'graph-renderer',
    });
    expect(resolveFixtureRoute('#/missing-fixture', names)).toEqual({
      status: 'unknown',
      name: 'missing-fixture',
    });
  });

  it('accepts a Map of loaders keyed by fixture name', () => {
    const loaders = new Map<string, () => Promise<void>>([
      ['view-controls', () => Promise.resolve()],
      ['graph-renderer', () => Promise.resolve()],
    ]);
    expect(resolveFixtureRoute('#/graph-renderer', loaders)).toEqual({
      status: 'found',
      name: 'graph-renderer',
    });
    expect(resolveFixtureRoute('#/source-editor', loaders)).toEqual({
      status: 'unknown',
      name: 'source-editor',
    });
  });

  it('accepts a record of loaders keyed by fixture name', () => {
    const loaders = {
      'view-controls': () => Promise.resolve(),
      'changes-panel': () => Promise.resolve(),
    };
    expect(resolveFixtureRoute('#/changes-panel', loaders)).toEqual({
      status: 'found',
      name: 'changes-panel',
    });
    expect(resolveFixtureRoute('#/assurance-panel', loaders)).toEqual({
      status: 'unknown',
      name: 'assurance-panel',
    });
  });
});

describe('Vite glob discovery', () => {
  it('discovers more than one Demo.svelte without editing App.svelte', () => {
    const names = discoverFixtureNames(Object.keys(fixtureModules));
    expect(names).toEqual(expect.arrayContaining(['harness-discovery', 'harness-example']));
    expect(new Set(names).size).toBe(names.length);
  });
});
