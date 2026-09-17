<script lang="ts">
  import { DEFAULT_SNAPSHOT_CASE, SNAPSHOT_CASES, snapshotCaseById } from './cases';
  import SnapshotPanel from './SnapshotPanel.svelte';

  let selectedId = $state(DEFAULT_SNAPSHOT_CASE.id);
  let selectionCount = $state(0);
  let resetCount = $state(0);

  const current = $derived(snapshotCaseById(selectedId));

  function onCaseChange(event: Event) {
    const target = event.currentTarget;
    if (!(target instanceof HTMLSelectElement)) {
      return;
    }
    selectedId = target.value;
    selectionCount += 1;
  }

  function resetCase() {
    selectedId = DEFAULT_SNAPSHOT_CASE.id;
    resetCount += 1;
  }
</script>

<section class="demo" aria-labelledby="snapshot-demo-title">
  <p class="label">SYNTHETIC DEVELOPMENT FIXTURE — NOT LIVE DATA</p>
  <h2 id="snapshot-demo-title">Synthetic snapshot panel cases</h2>
  <p class="lede">
    These cases are synthetic lab fixtures. They are not device configuration,
    operational state, or live collector output.
  </p>

  <div class="demo-controls">
    <label for="synthetic-snapshot-case">Synthetic snapshot case</label>
    <select
      id="synthetic-snapshot-case"
      data-demo-case
      value={selectedId}
      onchange={onCaseChange}
    >
      {#each SNAPSHOT_CASES as item (item.id)}
        <option value={item.id}>{item.label}</option>
      {/each}
    </select>
    <button type="button" data-demo-reset onclick={resetCase}>
      Reset synthetic case
    </button>
    <p data-demo-selection-count>Fixture selections: {selectionCount}</p>
    <p data-demo-reset-count>Fixture resets: {resetCount}</p>
  </div>

  <SnapshotPanel
    title={current.props.title}
    snapshotId={current.props.snapshotId}
    datastore={current.props.datastore}
    provenance={current.props.provenance}
    groups={current.props.groups}
  />
</section>

<style>
  .demo {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    min-width: 0;
    max-width: 100%;
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

  .lede,
  .demo-controls p {
    color: var(--fg-muted);
  }

  .demo-controls {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.45rem;
    width: 100%;
    min-width: 0;
    padding: 0.65rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 0.35rem;
    background: var(--bg);
  }

  label {
    font-weight: 650;
  }

  select,
  button {
    max-width: 100%;
    font: inherit;
  }

  select {
    width: 100%;
  }

  button {
    align-self: flex-start;
  }
</style>
