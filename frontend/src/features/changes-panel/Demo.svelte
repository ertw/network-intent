<script lang="ts">
  import ChangesPanel from './ChangesPanel.svelte';
  import {
    BANNER,
    DEFAULT_FIXTURE,
    FIXTURE_KEYS,
    FIXTURES,
    MISSING_SELECTED_ID,
    type FixtureKey,
  } from './cases';

  let fixtureKey = $state<FixtureKey>(DEFAULT_FIXTURE.key);
  let selectedId = $state<string | null>(DEFAULT_FIXTURE.initialSelectedId);
  let ignoreSelect = $state(false);
  let selectCount = $state(0);
  let lastSelectId = $state<string | null>(null);

  const fixture = $derived(FIXTURES[fixtureKey]);

  function applyFixture(key: FixtureKey) {
    fixtureKey = key;
    selectedId = FIXTURES[key].initialSelectedId;
  }

  function resetFixture() {
    applyFixture(DEFAULT_FIXTURE.key);
    ignoreSelect = false;
    selectCount = 0;
    lastSelectId = null;
  }

  function onSelect(id: string) {
    selectCount += 1;
    lastSelectId = id;
    if (!ignoreSelect) {
      selectedId = id;
    }
  }
</script>

<section class="fixture" aria-labelledby="changes-demo-title">
  <p class="banner">{BANNER}</p>
  <h2 id="changes-demo-title">Changes panel — synthetic supplied list</h2>
  <p class="lede">
    Isolated read-only display of an already-supplied change list. This fixture does not
    calculate a diff, admit, apply, or deploy anything.
  </p>

  <div class="controls" data-testid="changes-demo-controls">
    <label>
      Synthetic case
      <select
        aria-label="Synthetic case"
        data-testid="changes-case"
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
        data-testid="changes-ignore-select"
        checked={ignoreSelect}
        onchange={(event) => (ignoreSelect = event.currentTarget.checked)}
      />
      Ignore onSelect (retain parent selectedId)
    </label>
    <button type="button" data-testid="changes-reset" onclick={resetFixture}>Reset fixture</button>
    <button
      type="button"
      data-testid="changes-missing-id"
      onclick={() => (selectedId = MISSING_SELECTED_ID)}
    >
      Select unknown id
    </button>
    <p data-testid="changes-select-count">onSelect calls: {selectCount}</p>
    <p data-testid="changes-last-select">Last onSelect id: {lastSelectId ?? 'none'}</p>
    <p data-testid="changes-frozen">
      Source changes frozen: {Object.isFrozen(fixture.changes) ? 'yes' : 'no'}
    </p>
  </div>

  <ChangesPanel changes={fixture.changes} {selectedId} {onSelect} />
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
