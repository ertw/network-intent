<script lang="ts">
  import type { EditorSelection } from '../../contracts/presentation';
  import SourceEditor from './SourceEditor.svelte';
  import {
    BANNER,
    DEFAULT_FIXTURE,
    FIXTURES,
    fixtureByKey,
  } from './fixtures';
  import { tryInsertAsUser } from './user-edit';

  let fixtureKey = $state(DEFAULT_FIXTURE.key);
  let source = $state(DEFAULT_FIXTURE.source);
  let editable = $state(true);
  let selection = $state<EditorSelection | null>(null);
  let ignoreChange = $state(false);
  let ignoreSelection = $state(false);
  let mounted = $state(true);
  let secondInstance = $state(false);

  let changeCount = $state(0);
  let selectionCount = $state(0);
  let historyCount = $state(0);
  let lastHistory = $state<'undo' | 'redo' | 'none'>('none');
  let lastSelection = $state<EditorSelection | null>(null);

  let primaryRoot: HTMLDivElement | undefined = $state();

  const fixture = $derived(fixtureByKey(fixtureKey));

  function applyFixture(key: string) {
    const next = fixtureByKey(key);
    fixtureKey = next.key;
    source = next.source;
    editable = next.editable;
    selection = null;
  }

  function resetFixture() {
    applyFixture(DEFAULT_FIXTURE.key);
    ignoreChange = false;
    ignoreSelection = false;
    changeCount = 0;
    selectionCount = 0;
    historyCount = 0;
    lastHistory = 'none';
    lastSelection = null;
    mounted = true;
    secondInstance = false;
  }

  function onChange(next: string) {
    changeCount += 1;
    if (!ignoreChange) {
      source = next;
    }
  }

  function onSelectionChange(next: EditorSelection) {
    selectionCount += 1;
    lastSelection = next;
    if (!ignoreSelection) {
      selection = null;
    }
  }

  function onHistoryRequest(direction: 'undo' | 'redo') {
    historyCount += 1;
    lastHistory = direction;
  }

  function externalReset() {
    source = DEFAULT_FIXTURE.source;
    selection = null;
  }

  function applyExternalSelection() {
    selection = { anchor: 0, head: 5 };
  }

  function applyOutOfRangeSelection() {
    selection = { anchor: -4, head: 9999 };
  }

  function applyNonFiniteSelection() {
    selection = { anchor: Number.NaN, head: 1 };
  }

  function attemptUserInsert() {
    const host = primaryRoot?.querySelector('[data-testid="source-editor-host"]');
    if (host instanceof HTMLElement) {
      tryInsertAsUser(host, 'X');
    }
  }

  const lastSelectionText = $derived(
    lastSelection === null
      ? 'none'
      : `${lastSelection.anchor},${lastSelection.head}`,
  );
</script>

<section class="fixture" aria-labelledby="source-editor-demo-title">
  <p class="banner">{BANNER}</p>
  <h2 id="source-editor-demo-title">Source editor — synthetic document</h2>
  <p class="lede">
    Isolated CodeMirror widget. This fixture keeps text in memory, does not parse NetDSL,
    does not admit or deploy anything, and does not implement semantic Undo/Redo.
  </p>

  <div class="controls" data-testid="source-editor-demo-controls">
    <label>
      Synthetic case
      <select
        aria-label="Synthetic case"
        data-testid="source-editor-case"
        value={fixtureKey}
        onchange={(event) =>
          applyFixture((event.currentTarget as HTMLSelectElement).value)}
      >
        {#each FIXTURES as item (item.key)}
          <option value={item.key}>{item.label}</option>
        {/each}
      </select>
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="source-editor-editable"
        checked={editable}
        onchange={(event) => (editable = event.currentTarget.checked)}
      />
      Edit mode
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="source-editor-ignore-change"
        checked={ignoreChange}
        onchange={(event) => (ignoreChange = event.currentTarget.checked)}
      />
      Ignore onChange (retain parent source)
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="source-editor-ignore-selection"
        checked={ignoreSelection}
        onchange={(event) => (ignoreSelection = event.currentTarget.checked)}
      />
      Ignore onSelectionChange (retain parent selection)
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="source-editor-mounted"
        checked={mounted}
        onchange={(event) => (mounted = event.currentTarget.checked)}
      />
      Mount editor
    </label>
    <label class="choice">
      <input
        type="checkbox"
        data-testid="source-editor-second"
        checked={secondInstance}
        onchange={(event) => (secondInstance = event.currentTarget.checked)}
      />
      Second instance
    </label>
    <button type="button" data-testid="source-editor-reset" onclick={resetFixture}>
      Reset fixture
    </button>
    <button type="button" data-testid="source-editor-external-reset" onclick={externalReset}>
      External source replacement
    </button>
    <button type="button" data-testid="source-editor-set-selection" onclick={applyExternalSelection}>
      Set selection 0–5
    </button>
    <button type="button" data-testid="source-editor-clamp-selection" onclick={applyOutOfRangeSelection}>
      Set out-of-range selection
    </button>
    <button
      type="button"
      data-testid="source-editor-nonfinite-selection"
      onclick={applyNonFiniteSelection}
    >
      Set non-finite selection
    </button>
    <button type="button" data-testid="source-editor-user-command" onclick={attemptUserInsert}>
      Attempt user-edit command
    </button>
    <p data-testid="source-editor-change-count">onChange calls: {changeCount}</p>
    <p data-testid="source-editor-selection-count">onSelectionChange calls: {selectionCount}</p>
    <p data-testid="source-editor-history-count">onHistoryRequest calls: {historyCount}</p>
    <p data-testid="source-editor-last-history">Last history request: {lastHistory}</p>
    <p data-testid="source-editor-last-selection">Last selection: {lastSelectionText}</p>
    <p data-testid="source-editor-history-note">
      History requests are fixture events only. This demo has no undo stack.
    </p>
  </div>

  <p class="field-label">Parent source (authoritative)</p>
  <pre class="parent-source" data-testid="source-editor-parent-source">{source}</pre>

  {#if mounted}
    <div bind:this={primaryRoot} data-testid="source-editor-primary">
      <SourceEditor
        {source}
        {editable}
        {selection}
        {onChange}
        {onSelectionChange}
        {onHistoryRequest}
      />
    </div>
  {/if}

  {#if secondInstance}
    <div data-testid="source-editor-secondary">
      <SourceEditor
        {source}
        {editable}
        {selection}
        {onChange}
        {onSelectionChange}
        {onHistoryRequest}
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

  .lede,
  .field-label {
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

  .parent-source {
    margin: 0;
    max-height: 8rem;
    overflow: auto;
    padding: 0.45rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--bg);
    font-family: var(--mono);
    font-size: 0.82rem;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
