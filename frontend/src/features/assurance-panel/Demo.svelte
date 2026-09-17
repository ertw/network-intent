<script lang="ts">
  import AssurancePanel from './AssurancePanel.svelte';
  import { FIXTURE_KEYS, FIXTURES, MISSING_SELECTED_ID, type FixtureKey } from './fixtures';

  let fixtureKey = $state<FixtureKey>('all-outcomes');
  let selectedId = $state<string | null>(null);
  let selectCount = $state(0);
  let lastSelectId = $state<string | null>(null);

  const fixture = $derived(FIXTURES[fixtureKey]);

  function applyFixture(key: FixtureKey) {
    fixtureKey = key;
    selectedId = FIXTURES[key].initialSelectedId;
  }

  function resetFixture() {
    applyFixture('all-outcomes');
    selectCount = 0;
    lastSelectId = null;
  }

  function selectUnknownId() {
    selectedId = MISSING_SELECTED_ID;
  }

  function onSelect(id: string) {
    selectedId = id;
    lastSelectId = id;
    selectCount += 1;
  }
</script>

<section class="fixture" aria-labelledby="assurance-demo-title">
  <p class="banner">DEVELOPMENT FIXTURE — NOT LIVE DATA</p>
  <h2 id="assurance-demo-title">Assurance panel — synthetic observations</h2>
  <p class="lede">
    Isolated read-only display of supplied assurance rows. This fixture is not live
    device data, does not execute probes, and does not admit or deploy anything.
  </p>

  <div class="controls" data-testid="assurance-demo-controls">
    <label>
      Synthetic case
      <select
        aria-label="Synthetic case"
        value={fixtureKey}
        onchange={(event) =>
          applyFixture((event.currentTarget as HTMLSelectElement).value as FixtureKey)}
      >
        {#each FIXTURE_KEYS as key (key)}
          <option value={key}>{FIXTURES[key].label}</option>
        {/each}
      </select>
    </label>
    <button type="button" onclick={resetFixture}>Reset fixture</button>
    <button type="button" onclick={selectUnknownId}>Select unknown id</button>
    <p data-testid="assurance-select-count">onSelect calls: {selectCount}</p>
    <p data-testid="assurance-last-select-id">Last onSelect id: {lastSelectId ?? 'none'}</p>
    <p data-testid="assurance-rows-frozen">
      Source rows frozen: {Object.isFrozen(fixture.rows) ? 'yes' : 'no'}
    </p>
  </div>

  <AssurancePanel
    planId={fixture.planId}
    graphVersionLabel={fixture.graphVersionLabel}
    rows={fixture.rows}
    {selectedId}
    {onSelect}
  />
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

  label {
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
