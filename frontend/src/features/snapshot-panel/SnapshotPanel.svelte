<script lang="ts">
  import type { DisplayField, SnapshotPanelProps } from '../../contracts/presentation';
  import {
    completenessLabel,
    datastoreLabel,
    emptyGroupsMessage,
    freshnessLabel,
    isIncomplete,
    missingEmptyLabel,
    reasonDisplay,
    timestampDisplay,
  } from './labels';
  import { applyScrollKey } from './scrollable-value';

  let { title, snapshotId, datastore, provenance, groups }: SnapshotPanelProps =
    $props();

  function onScrollableKeydown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.shiftKey || event.ctrlKey || event.metaKey || event.altKey) return;
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

<section class="snapshot-panel" data-snapshot-panel aria-labelledby="snapshot-panel-title">
  <header class="header">
    <h2 id="snapshot-panel-title">{title}</h2>
    <p class="datastore" data-datastore-label>{datastoreLabel(datastore)}</p>
  </header>

  <dl class="meta">
    <dt>Snapshot ID</dt>
    <dd data-snapshot-id data-snapshot-missing={snapshotId === null ? 'true' : undefined}>
      {snapshotId === null ? 'No snapshot' : snapshotId}
    </dd>
    <dt>Source</dt>
    <dd data-source>{provenance.sourceLabel}</dd>
    <dt>Collected at</dt>
    <dd data-collected>{timestampDisplay(provenance.collectedAt)}</dd>
    <dt>Received at</dt>
    <dd data-received>{timestampDisplay(provenance.receivedAt)}</dd>
    <dt>Freshness</dt>
    <dd data-freshness={provenance.freshness}>{freshnessLabel(provenance.freshness)}</dd>
    <dt>Completeness</dt>
    <dd data-completeness={provenance.completeness}>
      {completenessLabel(provenance.completeness)}
    </dd>
    <dt>Missing</dt>
    <dd data-missing>
      {#if provenance.missing.length === 0}
        <span data-missing-empty>{missingEmptyLabel()}</span>
      {:else}
        <ul class="missing-list">
          {#each provenance.missing as item, index (index)}
            <li data-missing-item>{item}</li>
          {/each}
        </ul>
      {/if}
    </dd>
    <dt>Unavailability reason</dt>
    <dd data-reason data-reason-empty={provenance.reason === null ? 'true' : undefined}>
      {reasonDisplay(provenance.reason)}
    </dd>
  </dl>

  {#if isIncomplete(provenance.completeness)}
    <p class="warning" data-completeness-warning role="status">
      Reported completeness is {completenessLabel(provenance.completeness)}. Absence of
      groups is not proof of no configuration.
    </p>
  {/if}

  {#if groups.length === 0}
    <p class="empty-groups" data-empty-groups>{emptyGroupsMessage(datastore)}</p>
  {:else}
    <div class="groups">
      {#each groups as group (group.id)}
        <section class="group" data-group data-group-id={group.id}>
          <h3>{group.label}</h3>
          <ul class="fields">
            {#each group.fields as field (field.id)}
              {@const kindLabel = fieldKindLabel(field)}
              <li class="field" data-field data-field-id={field.id} data-kind={field.kind}>
                <div class="field-heading">
                  <span class="field-label">{field.label}</span>
                  {#if kindLabel}
                    <span class="kind-tag" data-kind-tag>{kindLabel}</span>
                  {/if}
                </div>

                {#if field.kind === 'scalar'}
                  {#if field.value === ''}
                    <span class="empty-list" data-empty-value-label>Empty value</span>
                  {/if}
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                  <pre
                    class="value"
                    class:empty={field.value === ''}
                    data-value
                    data-empty={field.value === '' ? 'true' : undefined}
                    aria-label={field.value === '' ? 'Empty value' : field.label}
                    tabindex="0"
                    onkeydown={onScrollableKeydown}>{field.value}</pre>
                {:else if field.kind === 'ordered_list'}
                  {#if field.values.length === 0}
                    <p class="empty-list" data-empty-list>Empty list</p>
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
        </section>
      {/each}
    </div>
  {/if}
</section>

<style>
  .snapshot-panel {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    min-width: 0;
    max-width: 100%;
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-width: 0;
  }

  h2 {
    font-size: 1.15rem;
    line-height: 1.3;
    font-weight: 650;
  }

  h3 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.3;
    font-weight: 650;
  }

  .datastore {
    font-family: var(--mono);
    font-size: 0.92rem;
    font-weight: 700;
    color: var(--accent);
  }

  .meta {
    display: grid;
    grid-template-columns: minmax(0, 11rem) minmax(0, 1fr);
    gap: 0.35rem 0.75rem;
    margin: 0;
    min-width: 0;
  }

  dt {
    color: var(--fg-muted);
    font-size: 0.92rem;
  }

  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .missing-list,
  .fields,
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .list {
    gap: 0.25rem;
  }

  .warning {
    color: var(--danger);
    border: 1px solid var(--danger);
    border-radius: 0.35rem;
    padding: 0.5rem 0.65rem;
    background: var(--bg);
  }

  .empty-groups,
  .empty-list {
    color: var(--fg-muted);
  }

  .empty-list {
    font-family: var(--mono);
    font-size: 0.9rem;
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    min-width: 0;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
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

  .field-label {
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

  .value {
    display: block;
    margin: 0;
    max-width: 100%;
    min-width: 0;
    min-height: 0;
    max-height: 14rem;
    overflow: auto;
    overflow-x: auto;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--panel);
    font-family: var(--mono);
    font-size: 0.85rem;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    text-overflow: clip;
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
    .meta {
      grid-template-columns: 1fr;
      gap: 0.15rem;
    }

    dt {
      font-size: 0.82rem;
    }
  }
</style>
