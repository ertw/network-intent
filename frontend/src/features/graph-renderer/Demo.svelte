<script lang="ts">
  import GraphRenderer from './GraphRenderer.svelte';
  import {
    BANNER,
    DEFAULT_FIXTURE,
    FIXTURE_KEYS,
    FIXTURES,
    MISSING_SELECTED_ID,
    type FixtureKey,
  } from './fixtures';

  let fixtureKey = $state<FixtureKey>(DEFAULT_FIXTURE.key);
  let selectedId = $state<string | null>(DEFAULT_FIXTURE.initialSelectedId);
  let ignoreSelect = $state(false);
  let secondInstance = $state(false);
  let selectCount = $state(0);
  let lastSelect = $state<string | 'null' | 'none'>('none');

  const fixture = $derived(FIXTURES[fixtureKey]);

  function applyFixture(key: FixtureKey) {
    fixtureKey = key;
    selectedId = FIXTURES[key].initialSelectedId;
  }

  function resetFixture() {
    applyFixture(DEFAULT_FIXTURE.key);
    ignoreSelect = false;
    secondInstance = false;
    selectCount = 0;
    lastSelect = 'none';
  }

  function onSelect(id: string | null) {
    selectCount += 1;
    lastSelect = id === null ? 'null' : id;
    if (!ignoreSelect) {
      selectedId = id;
    }
  }
</script>

<section class="fixture" aria-labelledby="graph-demo-title">
  <p class="banner">{BANNER}</p>
  <h2 id="graph-demo-title">Graph renderer — synthetic projection</h2>
  <p class="lede">
    Isolated read-only renderer of an already-projected graph. Layer tags are display labels
    only. This fixture is not topology, admission, or live device data.
  </p>

  <div class="controls" data-testid="graph-demo-controls">
    <label>
      Synthetic case
      <select
        aria-label="Synthetic case"
        data-testid="graph-case"
        value={fixtureKey}
        onchange={(event) =>
          applyFixture((event.currentTarget as HTMLSelectElement).value as FixtureKey)}
      >
        {#each FIXTURE_KEYS as key (key)}
          <option value={key}>{FIXTURES[key].label}</option>
        {/each}
      </select>
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="graph-ignore-select"
        checked={ignoreSelect}
        onchange={(event) => (ignoreSelect = event.currentTarget.checked)}
      />
      Ignore onSelect (retain parent selectedId)
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="graph-second"
        checked={secondInstance}
        onchange={(event) => (secondInstance = event.currentTarget.checked)}
      />
      Second instance
    </label>
    <button type="button" data-testid="graph-reset" onclick={resetFixture}>Reset fixture</button>
    <button
      type="button"
      data-testid="graph-missing-id"
      onclick={() => (selectedId = MISSING_SELECTED_ID)}
    >
      Select unknown id
    </button>
    <p data-testid="graph-select-count">onSelect calls: {selectCount}</p>
    <p data-testid="graph-last-select">Last onSelect id: {lastSelect}</p>
    <p data-testid="graph-nodes-frozen">
      Source nodes frozen: {Object.isFrozen(fixture.nodes) ? 'yes' : 'no'}
    </p>
  </div>

  <GraphRenderer
    nodes={fixture.nodes}
    edges={fixture.edges}
    {selectedId}
    {onSelect}
  />

  {#if secondInstance}
    <div data-testid="graph-secondary">
      <GraphRenderer
        nodes={fixture.nodes}
        edges={fixture.edges}
        {selectedId}
        {onSelect}
      />
    </div>
  {/if}
</section>

<style>
  .fixture {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
    max-width: 100%;
  }

  .banner {
    font-family: var(--mono);
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    font-weight: 700;
    color: var(--danger);
    margin: 0;
  }

  h2 {
    font-size: 1.15rem;
    line-height: 1.3;
    margin: 0;
  }

  .lede {
    color: var(--fg-muted);
    margin: 0;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem 0.85rem;
    align-items: center;
    min-width: 0;
  }

  .controls p {
    margin: 0;
    font-family: var(--mono);
    font-size: 0.82rem;
  }

  label,
  .choice {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.55rem;
    align-items: center;
    font-size: 0.9rem;
  }

  select,
  button {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    padding: 0.35rem 0.55rem;
    max-width: 100%;
  }

  button {
    cursor: pointer;
  }
</style>
