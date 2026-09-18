<script lang="ts">
  import type { ChangesPanelProps, DisplayField } from '../../contracts/presentation';
  import {
    emptyListLabel,
    emptyValueLabel,
    explanationDisplay,
    kindLabel,
    noChangesLabel,
    noFieldsLabel,
  } from './labels';
  import { applyScrollKey } from './scrollable-value';

  let { changes, selectedId, onSelect }: ChangesPanelProps = $props();

  const instance = $props.id();
  const headingId = `${instance}-changes-heading`;
  const selected = $derived(changes.find((entry) => entry.id === selectedId) ?? null);

  function onScrollableKeydown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.shiftKey || event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) {
      return;
    }
    if (applyScrollKey(target, event.key)) {
      event.preventDefault();
    }
  }

  function fieldKindLabel(field: DisplayField): string | null {
    if (field.kind === 'redacted') {
      return 'Redacted';
    }
    if (field.kind === 'unknown') {
      return 'Unknown';
    }
    return null;
  }
</script>

<section class="panel" data-testid="changes-panel" aria-labelledby={headingId}>
  <h3 id={headingId}>Supplied changes</h3>

  {#if changes.length === 0}
    <p data-testid="changes-empty">{noChangesLabel()}</p>
  {:else}
    <ul class="rows">
      {#each changes as entry (entry.id)}
        <li>
          <button
            type="button"
            class="row"
            class:selected={selectedId === entry.id}
            data-testid="changes-row"
            data-change-id={entry.id}
            data-kind={entry.kind}
            aria-pressed={selectedId === entry.id}
            onclick={() => onSelect(entry.id)}
          >
            <span class="row-label">{entry.label}</span>
            <span class="kind" data-testid="changes-kind">{kindLabel(entry.kind)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <section class="detail" data-testid="changes-detail" aria-label="Change detail">
    {#if selected}
      <h4>Change detail</h4>
      <p>
        <span class="field-label">Kind</span>
        <span class="field-value" data-testid="changes-detail-kind">{kindLabel(selected.kind)}</span>
      </p>
      <p>
        <span class="field-label">Explanation</span>
        <span
          class="field-value explanation"
          data-testid="changes-explanation"
          data-explanation-empty={selected.explanation === null ? 'true' : undefined}
          >{explanationDisplay(selected.explanation)}</span
        >
      </p>

      <div class="sides">
        <section class="side" data-testid="changes-before" aria-label="Before">
          <h5>Before</h5>
          {#if selected.before.length === 0}
            <p data-testid="changes-before-empty">{noFieldsLabel()}</p>
          {:else}
            <ul class="fields">
              {#each selected.before as field (field.id)}
                {@const kindTag = fieldKindLabel(field)}
                <li class="field" data-field data-field-id={field.id} data-kind={field.kind}>
                  <div class="field-heading">
                    <span class="field-name">{field.label}</span>
                    {#if kindTag}
                      <span class="kind-tag">{kindTag}</span>
                    {/if}
                  </div>
                  {#if field.kind === 'scalar'}
                    {#if field.value === ''}
                      <span class="empty-note" data-empty-value-label>{emptyValueLabel()}</span>
                    {/if}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <pre
                      class="value"
                      class:empty={field.value === ''}
                      data-value
                      tabindex="0"
                      onkeydown={onScrollableKeydown}>{field.value}</pre>
                  {:else if field.kind === 'ordered_list'}
                    {#if field.values.length === 0}
                      <p class="empty-note" data-empty-list>{emptyListLabel()}</p>
                    {:else}
                      <ol class="list" data-list>
                        {#each field.values as entry, index (index)}
                          <li>
                            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                            <pre
                              class="value list-value"
                              data-value
                              tabindex="0"
                              onkeydown={onScrollableKeydown}>{entry}</pre>
                          </li>
                        {/each}
                      </ol>
                    {/if}
                  {:else if field.kind === 'unknown'}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <pre
                      class="value unknown-reason"
                      data-unknown-reason
                      tabindex="0"
                      onkeydown={onScrollableKeydown}>{field.reason}</pre>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>

        <section class="side" data-testid="changes-after" aria-label="After">
          <h5>After</h5>
          {#if selected.after.length === 0}
            <p data-testid="changes-after-empty">{noFieldsLabel()}</p>
          {:else}
            <ul class="fields">
              {#each selected.after as field (field.id)}
                {@const kindTag = fieldKindLabel(field)}
                <li class="field" data-field data-field-id={field.id} data-kind={field.kind}>
                  <div class="field-heading">
                    <span class="field-name">{field.label}</span>
                    {#if kindTag}
                      <span class="kind-tag">{kindTag}</span>
                    {/if}
                  </div>
                  {#if field.kind === 'scalar'}
                    {#if field.value === ''}
                      <span class="empty-note" data-empty-value-label>{emptyValueLabel()}</span>
                    {/if}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <pre
                      class="value"
                      class:empty={field.value === ''}
                      data-value
                      tabindex="0"
                      onkeydown={onScrollableKeydown}>{field.value}</pre>
                  {:else if field.kind === 'ordered_list'}
                    {#if field.values.length === 0}
                      <p class="empty-note" data-empty-list>{emptyListLabel()}</p>
                    {:else}
                      <ol class="list" data-list>
                        {#each field.values as entry, index (index)}
                          <li>
                            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                            <pre
                              class="value list-value"
                              data-value
                              tabindex="0"
                              onkeydown={onScrollableKeydown}>{entry}</pre>
                          </li>
                        {/each}
                      </ol>
                    {/if}
                  {:else if field.kind === 'unknown'}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <pre
                      class="value unknown-reason"
                      data-unknown-reason
                      tabindex="0"
                      onkeydown={onScrollableKeydown}>{field.reason}</pre>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>
    {/if}
  </section>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    min-width: 0;
    max-width: 100%;
  }

  h3,
  h4,
  h5 {
    margin: 0;
    font-size: 1.05rem;
    line-height: 1.3;
    font-weight: 650;
  }

  h4 {
    font-size: 1rem;
  }

  h5 {
    font-size: 0.95rem;
  }

  .rows,
  .fields,
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .list {
    gap: 0.25rem;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.35rem 0.75rem;
    width: 100%;
    min-width: 0;
    text-align: left;
    margin: 0;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--panel);
    color: var(--fg);
    font: inherit;
    cursor: pointer;
  }

  .row.selected {
    border-color: var(--fg);
    background: color-mix(in srgb, var(--border) 28%, var(--panel));
  }

  .row-label {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .kind {
    font-family: var(--mono);
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--accent);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-width: 0;
  }

  .detail > p {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    margin: 0;
    min-width: 0;
  }

  .field-label {
    color: var(--fg-muted);
    font-size: 0.78rem;
    letter-spacing: 0.02em;
  }

  .field-value,
  .field-name,
  .empty-note {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .field-value {
    font-family: var(--mono);
    font-size: 0.88rem;
  }

  .explanation {
    white-space: pre-wrap;
  }

  .sides {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
    min-width: 0;
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-width: 0;
    padding: 0.65rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 0.35rem;
    background: var(--bg);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }

  .field-heading {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem 0.65rem;
  }

  .field-name {
    font-weight: 650;
  }

  .kind-tag {
    font-family: var(--mono);
    font-size: 0.78rem;
    letter-spacing: 0.04em;
    font-weight: 700;
    color: var(--danger);
  }

  .field[data-kind='unknown'] .kind-tag {
    color: var(--fg-muted);
  }

  .empty-note {
    color: var(--fg-muted);
    font-family: var(--mono);
    font-size: 0.9rem;
    margin: 0;
  }

  .value {
    display: block;
    margin: 0;
    max-width: 100%;
    min-width: 0;
    min-height: 0;
    max-height: 14rem;
    overflow: auto;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--panel);
    font-family: var(--mono);
    font-size: 0.85rem;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .value:focus-visible {
    outline: 3px solid var(--focus);
    outline-offset: 3px;
  }

  .value.empty {
    min-height: 2.1rem;
  }

  .list-value {
    max-height: 8rem;
  }

  .unknown-reason {
    color: var(--fg-muted);
    font-style: italic;
    background: var(--bg);
  }

  @media (max-width: 700px) {
    .sides {
      grid-template-columns: 1fr;
    }
  }
</style>
