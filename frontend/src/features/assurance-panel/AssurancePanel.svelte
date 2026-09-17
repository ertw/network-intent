<script lang="ts">
  import type { AssurancePanelProps, AssuranceRow } from '../../contracts/presentation';
  import { allowsPositiveAccent, outcomeLabel, timestampLabel } from './display';

  let { planId, graphVersionLabel, rows, selectedId, onSelect }: AssurancePanelProps = $props();

  const selectedRow = $derived(rows.find((entry) => entry.id === selectedId) ?? null);

  function missingText(row: AssuranceRow): string {
    return row.provenance.missing.length === 0 ? 'none' : row.provenance.missing.join(', ');
  }

  function dependsOnText(row: AssuranceRow): string {
    return row.dependsOn.length === 0 ? 'none' : row.dependsOn.join(', ');
  }
</script>

<section class="panel" data-testid="assurance-panel" aria-labelledby="assurance-panel-heading">
  <header class="identity">
    <h3 id="assurance-panel-heading">Assurance observations</h3>
    <p>
      <span class="field-label">Plan ID</span>
      <span class="field-value" data-testid="assurance-plan-id">{planId === null ? 'No plan' : planId}</span>
    </p>
    <p>
      <span class="field-label">Graph version</span>
      <span class="field-value" data-testid="assurance-graph-version"
        >{graphVersionLabel === null ? 'Unknown graph version' : graphVersionLabel}</span
      >
    </p>
  </header>

  {#if rows.length === 0}
    <p data-testid="assurance-empty">No assurance observations</p>
  {:else}
    <ul class="rows">
      {#each rows as row (row.id)}
        {@const positiveAccent = allowsPositiveAccent(
          row.outcome,
          row.provenance.freshness,
          row.provenance.completeness,
        )}
        <li class="row-item">
          <button
            type="button"
            class="row"
            class:selected={selectedId === row.id}
            class:positive-accent={positiveAccent}
            data-testid="assurance-row"
            data-row-id={row.id}
            data-outcome={row.outcome === null ? 'none' : row.outcome}
            data-freshness={row.provenance.freshness}
            data-completeness={row.provenance.completeness}
            data-positive-accent={positiveAccent ? 'true' : 'false'}
            aria-pressed={selectedId === row.id}
            onclick={() => onSelect(row.id)}
          >
            <span class="row-label">{row.label}</span>
            <span class="row-meta">
              <span><span class="meta-key">Primitive</span> {row.primitive}</span>
              <span class="outcome"
                ><span class="meta-key">Outcome</span> {outcomeLabel(row.outcome)}</span
              >
              <span><span class="meta-key">Freshness</span> {row.provenance.freshness}</span>
              <span><span class="meta-key">Completeness</span> {row.provenance.completeness}</span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <section class="detail" data-testid="assurance-detail" aria-label="Observation detail">
    {#if selectedRow}
      <h4>Observation detail</h4>
      <p>
        <span class="field-label">Observation source</span>
        <span class="field-value">{selectedRow.sourceLabel}</span>
      </p>
      <p>
        <span class="field-label">Provenance source</span>
        <span class="field-value">{selectedRow.provenance.sourceLabel}</span>
      </p>
      <p>
        <span class="field-label">Endpoint</span>
        <span class="field-value"
          >{selectedRow.endpointLabel === null
            ? 'Device-local / no endpoint supplied'
            : selectedRow.endpointLabel}</span
        >
      </p>
      <p>
        <span class="field-label">Detail</span>
        <span class="field-value detail-text">{selectedRow.detail}</span>
      </p>
      <p>
        <span class="field-label">Outcome</span>
        <span class="field-value">{outcomeLabel(selectedRow.outcome)}</span>
      </p>
      <p>
        <span class="field-label">Freshness</span>
        <span class="field-value">{selectedRow.provenance.freshness}</span>
      </p>
      <p>
        <span class="field-label">Completeness</span>
        <span class="field-value">{selectedRow.provenance.completeness}</span>
      </p>
      <p>
        <span class="field-label">Missing</span>
        <span class="field-value">{missingText(selectedRow)}</span>
      </p>
      <p>
        <span class="field-label">Reason</span>
        <span class="field-value">{selectedRow.provenance.reason ?? 'none'}</span>
      </p>
      <p>
        <span class="field-label">Collected at</span>
        <span class="field-value">{timestampLabel(selectedRow.provenance.collectedAt)}</span>
      </p>
      <p>
        <span class="field-label">Received at</span>
        <span class="field-value">{timestampLabel(selectedRow.provenance.receivedAt)}</span>
      </p>
      <p>
        <span class="field-label">Depends on</span>
        <span class="field-value">{dependsOnText(selectedRow)}</span>
      </p>
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

  .identity,
  .detail,
  .rows {
    min-width: 0;
  }

  h3,
  h4 {
    font-size: 1.05rem;
    line-height: 1.3;
    font-weight: 650;
    margin: 0;
  }

  .identity,
  .detail {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .identity p,
  .detail p {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
    margin: 0;
  }

  .field-label,
  .meta-key {
    color: var(--fg-muted);
    font-size: 0.78rem;
    letter-spacing: 0.02em;
  }

  .field-value,
  .row-label,
  .row-meta {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .field-value {
    font-family: var(--mono);
    font-size: 0.88rem;
  }

  .detail-text {
    white-space: pre-wrap;
  }

  .rows {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .row-item {
    min-width: 0;
  }

  .row {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.35rem;
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

  .row.positive-accent {
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .row.positive-accent .outcome {
    color: var(--accent);
    font-weight: 650;
  }

  .row-label {
    font-size: 0.95rem;
    line-height: 1.35;
  }

  .row-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem 0.85rem;
    font-family: var(--mono);
    font-size: 0.8rem;
  }

  .row-meta span {
    min-width: 0;
  }
</style>
