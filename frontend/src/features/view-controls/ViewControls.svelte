<script lang="ts">
  import { tick } from 'svelte';
  import type {
    EditMode,
    Layer,
    StateView,
    ViewControlsProps,
    ViewSelection,
  } from '../../contracts/presentation';
  import {
    CANONICAL_LAYERS,
    EDIT_MODE_OPTIONS,
    STATE_VIEW_OPTIONS,
    emitSelection,
    overlayChoices,
    selectionWithOverlay,
    selectionWithPrimary,
  } from './view-selection';

  let { value, onChange }: ViewControlsProps = $props();

  const instance = $props.id();
  let controls: HTMLElement | undefined;

  async function requestChange(next: ViewSelection) {
    onChange(next);
    await tick();
    // Native inputs toggle before change fires. A parent may retain its value,
    // in which case Svelte has no changed prop to trigger a DOM update.
    const fields = new Map<string, string | boolean>([
      [`${instance}-primary-layer`, value.primaryLayer],
      [`${instance}-state-view`, value.stateView],
      [`${instance}-edit-mode`, value.editMode],
      [`${instance}-combined-overview`, value.combinedOverview],
      [`${instance}-assurance-overlay`, value.assuranceOverlay],
    ]);
    for (const input of controls?.querySelectorAll('input') ?? []) {
      if (input.name.startsWith(`${instance}-overlay-`)) {
        input.checked = value.overlays.includes(input.value as Layer);
      } else {
        const selected = fields.get(input.name);
        input.checked = typeof selected === 'boolean' ? selected : input.value === selected;
      }
    }
  }

  const overlayLayers = $derived(overlayChoices(value.primaryLayer));
  const layersLocked = $derived(value.combinedOverview);

  function emitPrimary(layer: Layer) {
    void requestChange(selectionWithPrimary(value, layer));
  }

  function emitOverlay(layer: Layer, enabled: boolean) {
    void requestChange(selectionWithOverlay(value, layer, enabled));
  }

  function emitCombined(combinedOverview: boolean) {
    void requestChange(emitSelection(value, { combinedOverview }));
  }

  function emitState(stateView: StateView) {
    void requestChange(emitSelection(value, { stateView }));
  }

  function emitAssurance(assuranceOverlay: boolean) {
    void requestChange(emitSelection(value, { assuranceOverlay }));
  }

  function emitEditMode(editMode: EditMode) {
    void requestChange(emitSelection(value, { editMode }));
  }
</script>

<section bind:this={controls} class="controls" aria-label="View controls">
  <fieldset class="group" disabled={layersLocked}>
    <legend>Primary layer</legend>
    <div class="choices">
      {#each CANONICAL_LAYERS as layer (layer)}
        <label class="choice">
          <input
            type="radio"
            name={`${instance}-primary-layer`}
            value={layer}
            checked={value.primaryLayer === layer}
            onchange={() => emitPrimary(layer)}
          />
          <span>{layer}</span>
        </label>
      {/each}
    </div>
  </fieldset>

  <details class="advanced">
    <summary>Advanced</summary>
    <div class="advanced-body">
      <fieldset class="group" disabled={layersLocked}>
        <legend>Overlays</legend>
        <div class="choices">
          {#each overlayLayers as layer (layer)}
            <label class="choice">
              <input
                type="checkbox"
                name={`${instance}-overlay-${layer}`}
                value={layer}
                checked={value.overlays.includes(layer)}
                onchange={(event) => emitOverlay(layer, event.currentTarget.checked)}
              />
              <span>{layer}</span>
            </label>
          {/each}
        </div>
      </fieldset>

      <label class="choice switch">
        <input
          type="checkbox"
          role="switch"
          name={`${instance}-combined-overview`}
          checked={value.combinedOverview}
          aria-checked={value.combinedOverview}
          onchange={(event) => emitCombined(event.currentTarget.checked)}
        />
        <span>Combined overview</span>
      </label>
    </div>
  </details>

  <fieldset class="group">
    <legend>State view</legend>
    <div class="choices">
      {#each STATE_VIEW_OPTIONS as option (option.value)}
        <label class="choice">
          <input
            type="radio"
            name={`${instance}-state-view`}
            value={option.value}
            checked={value.stateView === option.value}
            onchange={() => emitState(option.value)}
          />
          <span>{option.label}</span>
        </label>
      {/each}
    </div>
  </fieldset>

  <label class="choice switch">
    <input
      type="checkbox"
      role="switch"
      name={`${instance}-assurance-overlay`}
      checked={value.assuranceOverlay}
      aria-checked={value.assuranceOverlay}
      onchange={(event) => emitAssurance(event.currentTarget.checked)}
    />
    <span>Assurance overlay</span>
  </label>

  <fieldset class="group">
    <legend>Edit mode</legend>
    <div class="choices">
      {#each EDIT_MODE_OPTIONS as option (option.value)}
        <label class="choice">
          <input
            type="radio"
            name={`${instance}-edit-mode`}
            value={option.value}
            checked={value.editMode === option.value}
            onchange={() => emitEditMode(option.value)}
          />
          <span>{option.label}</span>
        </label>
      {/each}
    </div>
    <p class="note">
      View/Edit requests a mode change only. It does not authorize compiler, graph,
      source, adoption, or deployment operations.
    </p>
  </fieldset>
</section>

<style>
  .controls {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    min-width: 0;
    max-width: 100%;
    overflow-wrap: anywhere;
  }

  .group,
  .advanced {
    min-width: 0;
    min-inline-size: 0;
    max-width: 100%;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    background: var(--panel);
  }

  .group {
    padding: 0.65rem 0.75rem 0.75rem;
  }

  .group:disabled {
    color: var(--fg-muted);
  }

  legend,
  summary {
    font-weight: 650;
    color: var(--fg);
  }

  legend {
    padding: 0 0.3rem;
  }

  .advanced {
    padding: 0.45rem 0.75rem 0.75rem;
  }

  summary {
    cursor: pointer;
    padding: 0.2rem 0.1rem;
  }

  .advanced-body {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-top: 0.7rem;
    min-width: 0;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem 0.9rem;
  }

  .choice {
    display: inline-flex;
    align-items: flex-start;
    gap: 0.4rem;
    min-width: 0;
    max-width: 100%;
  }

  .switch {
    align-items: center;
  }

  input[type='radio'],
  input[type='checkbox'] {
    width: 1.05rem;
    height: 1.05rem;
    margin: 0.15rem 0 0;
    flex-shrink: 0;
    accent-color: var(--accent);
  }

  .note {
    margin: 0.65rem 0 0;
    color: var(--fg-muted);
    font-size: 0.92rem;
  }
</style>
