<script lang="ts">
  import type { ViewSelection } from '../../contracts/presentation';
  import ViewControls from './ViewControls.svelte';
  import { copySelection, freezeSelection } from './view-selection';

  const title = 'Development fixture only — not live data';
  const body =
    'SYNTHETIC development fixture — NOT LIVE DATA. The demo owns an in-memory ViewSelection and records emitted events. No network fetches, localStorage, IndexedDB, credentials, or live device data.';

  const defaultSelection: ViewSelection = {
    primaryLayer: 'L3',
    overlays: [],
    combinedOverview: false,
    stateView: 'intent',
    assuranceOverlay: false,
    editMode: 'view',
  };

  const outsideReplacement: ViewSelection = {
    primaryLayer: 'L7',
    overlays: ['L1', 'L4'],
    combinedOverview: false,
    stateView: 'compare',
    assuranceOverlay: true,
    editMode: 'edit',
  };

  const overlayOrderingCase: ViewSelection = {
    primaryLayer: 'L3',
    overlays: ['L2', 'L1', 'L1'],
    combinedOverview: false,
    stateView: 'intent',
    assuranceOverlay: false,
    editMode: 'view',
  };

  const combinedOverviewCase: ViewSelection = {
    primaryLayer: 'L2',
    overlays: ['L1', 'L7'],
    combinedOverview: false,
    stateView: 'intent',
    assuranceOverlay: false,
    editMode: 'view',
  };

  const overlaysIncludePrimaryCase: ViewSelection = {
    primaryLayer: 'L4',
    overlays: ['L4', 'L1'],
    combinedOverview: false,
    stateView: 'intent',
    assuranceOverlay: false,
    editMode: 'view',
  };

  let value = $state<ViewSelection>(copySelection(defaultSelection));
  let events = $state<ViewSelection[]>([]);
  let changeCount = $state(0);
  let holdSelection = $state(false);

  const presented = $derived(freezeSelection(value));
  const lastEvent = $derived(events.length === 0 ? null : events[events.length - 1]);
  const currentJson = $derived(JSON.stringify(value, null, 2));
  const lastEventJson = $derived(
    lastEvent === null ? 'None' : JSON.stringify(lastEvent, null, 2),
  );
  const eventsJson = $derived(JSON.stringify(events, null, 2));

  function handleChange(next: ViewSelection) {
    changeCount += 1;
    events = [...events, copySelection(next)];
    if (!holdSelection) value = copySelection(next);
  }

  function resetFixture() {
    value = copySelection(defaultSelection);
    events = [];
    changeCount = 0;
  }

  function replaceFromOutside() {
    value = copySelection(outsideReplacement);
  }

  function loadOverlayOrderingCase() {
    value = copySelection(overlayOrderingCase);
  }

  function loadCombinedOverviewCase() {
    value = copySelection(combinedOverviewCase);
  }

  function loadOverlaysIncludingPrimary() {
    value = copySelection(overlaysIncludePrimaryCase);
  }
</script>

<section class="fixture" aria-labelledby="view-controls-fixture-title">
  <p class="label">DEVELOPMENT FIXTURE — NOT LIVE DATA</p>
  <h2 id="view-controls-fixture-title">{title}</h2>
  <p>{body}</p>

  <ViewControls value={presented} onChange={handleChange} />

  <section class="tools" aria-label="Fixture controls">
    <h3>Fixture controls — not part of ViewControls</h3>
    <p class="tools-note">
      Selectors, reset, and event counters belong to this development fixture. They
      are not production ViewControls UI.
    </p>
    <label><input type="checkbox" bind:checked={holdSelection} />Hold supplied selection (fixture only)</label>
    <div class="actions">
      <button type="button" onclick={resetFixture}>Reset fixture</button>
      <button type="button" onclick={replaceFromOutside}>Replace selection from outside</button>
      <button type="button" onclick={loadOverlayOrderingCase}>Load overlay ordering case</button>
      <button type="button" onclick={loadCombinedOverviewCase}>Load combined overview case</button>
      <button type="button" onclick={loadOverlaysIncludingPrimary}>
        Load overlays including primary
      </button>
    </div>
    <p>Emitted event count</p>
    <p role="status" aria-label="Emitted event count">{changeCount}</p>
    <h4>Current view selection</h4>
    <pre class="payload" role="region" aria-label="Current view selection">{currentJson}</pre>
    <h4>Last emitted view selection</h4>
    <pre class="payload" role="region" aria-label="Last emitted view selection">{lastEventJson}</pre>
    <h4>Emitted events</h4>
    <pre class="payload" role="region" aria-label="Emitted events">{eventsJson}</pre>
  </section>
</section>

<style>
  .fixture {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
    max-width: 100%;
    overflow-wrap: anywhere;
  }

  .label {
    font-family: var(--mono);
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    font-weight: 700;
    color: var(--danger);
  }

  h2 {
    font-size: 1.15rem;
    line-height: 1.3;
  }

  h3,
  h4 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.3;
    font-weight: 650;
  }

  .tools {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    min-width: 0;
    margin-top: 0.4rem;
    padding: 0.85rem 0.9rem;
    border: 1px dashed var(--border);
    border-radius: 0.4rem;
    background: var(--bg);
  }

  .tools-note,
  .fixture > p {
    color: var(--fg-muted);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
  }

  button {
    font: inherit;
    color: var(--fg);
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    padding: 0.35rem 0.65rem;
    max-width: 100%;
    overflow-wrap: anywhere;
  }

  .payload {
    margin: 0;
    max-width: 100%;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--panel);
    font-family: var(--mono);
    font-size: 0.82rem;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
