<script lang="ts">
  import type { Component } from 'svelte';
  import { fixtureModules } from './lib/discover';
  import {
    discoverFixtureNames,
    fixtureNameFromModulePath,
    resolveFixtureRoute,
  } from './lib/fixtures';

  const heading =
    'Component development fixtures — no live device data or admission decisions';
  const fixtureNames = discoverFixtureNames(Object.keys(fixtureModules));
  const loaders = new Map<string, () => Promise<{ default: Component }>>();
  for (const [modulePath, loader] of Object.entries(fixtureModules)) {
    const name = fixtureNameFromModulePath(modulePath);
    if (name) {
      loaders.set(name, loader);
    }
  }

  let hash = $state('');
  let Fixture = $state<Component | null>(null);
  let loadState = $state<'idle' | 'loading' | 'ready' | 'failed'>('idle');

  const resolution = $derived(resolveFixtureRoute(hash, fixtureNames));

  $effect(() => {
    hash = window.location.hash;
    const onHashChange = () => {
      hash = window.location.hash;
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  });

  $effect(() => {
    if (resolution.status !== 'found') {
      Fixture = null;
      loadState = 'idle';
      return;
    }

    const loader = loaders.get(resolution.name);
    if (!loader) {
      Fixture = null;
      loadState = 'failed';
      return;
    }

    let cancelled = false;
    Fixture = null;
    loadState = 'loading';
    void loader()
      .then((module) => {
        if (!cancelled) {
          Fixture = module.default;
          loadState = 'ready';
        }
      })
      .catch(() => {
        if (!cancelled) {
          Fixture = null;
          loadState = 'failed';
        }
      });

    return () => {
      cancelled = true;
    };
  });
</script>

<div class="shell">
  <header class="banner">
    <h1>{heading}</h1>
    <p>
      Isolated component harness. This is not the integrated product and does not
      admit, deploy, or read live devices.
    </p>
  </header>

  <div class="layout">
    <nav class="nav" aria-label="Development fixtures">
      <h2>Fixtures</h2>
      <p class="nav-note">
        Routes are hash paths such as /#/view-controls. The list is discovered
        from Demo.svelte files; imaginary product pages are not listed.
      </p>
      {#if fixtureNames.length === 0}
        <p>No fixtures discovered. Add src/features/name/Demo.svelte.</p>
      {:else}
        <ul>
          {#each fixtureNames as name (name)}
            <li>
              <a
                href={`#/${name}`}
                aria-current={resolution.status === 'found' && resolution.name === name
                  ? 'page'
                  : undefined}>{name}</a
              >
            </li>
          {/each}
        </ul>
      {/if}
    </nav>

    <main class="main">
      {#if resolution.status === 'idle'}
        <p>
          Select a development fixture from the list, or open /#/fixture-name.
          Unknown names stay in memory and do not fetch from the network.
        </p>
      {:else if resolution.status === 'unknown'}
        <h2>Unknown fixture</h2>
        <p>
          No fixture named {resolution.name}. Add src/features/{resolution.name}/Demo.svelte
          to make this route load. This harness does not fetch from the network.
        </p>
      {:else if loadState === 'failed'}
        <h2>Fixture failed to load</h2>
        <p>
          The local module for {resolution.name} was found by glob but did not
          load. No remote request was made.
        </p>
      {:else if Fixture}
        <Fixture />
      {:else}
        <p>Loading local fixture {resolution.name}.</p>
      {/if}
    </main>
  </div>
</div>

<style>
  .shell {
    min-height: 100vh;
    padding: 1.25rem clamp(0.85rem, 2vw, 1.75rem) 1.75rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .banner {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    padding: 1rem 1.15rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  h1 {
    font-size: clamp(1.15rem, 2.2vw, 1.55rem);
    line-height: 1.25;
    font-weight: 650;
    letter-spacing: -0.02em;
  }

  h2 {
    font-size: 1.05rem;
    line-height: 1.3;
    font-weight: 650;
  }

  .banner p,
  .nav-note,
  .main p {
    color: var(--fg-muted);
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(12.5rem, 16rem) minmax(0, 1fr);
    gap: 1.25rem;
    align-items: start;
    flex: 1;
  }

  .nav,
  .main {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    padding: 1rem 1.15rem;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }

  .nav ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .nav a {
    display: inline-block;
    padding: 0.35rem 0.2rem;
    font-family: var(--mono);
    font-size: 0.92rem;
    text-decoration-thickness: 1px;
  }

  .nav a[aria-current='page'] {
    font-weight: 700;
    color: var(--fg);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
  }

  @media (max-width: 700px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
