<script lang="ts">
  import { Handle, Position, type NodeProps } from '@xyflow/svelte';

  let { id, data, selected }: NodeProps = $props();
  const label = $derived(typeof data.label === 'string' ? data.label : '');
  const subtitle = $derived(typeof data.subtitle === 'string' ? data.subtitle : null);
  const layer = $derived(typeof data.layer === 'string' ? data.layer : '');
  const x = $derived(typeof data.x === 'number' ? data.x : 0);
  const y = $derived(typeof data.y === 'number' ? data.y : 0);
</script>

<div
  class="node"
  class:selected
  data-testid="graph-canvas-node"
  data-node-id={id}
  data-x={x}
  data-y={y}
>
  <Handle type="target" id="in" position={Position.Left} isConnectable={false} />
  <p class="label">{label}</p>
  {#if subtitle !== null}
    <p class="subtitle">{subtitle}</p>
  {/if}
  <p class="layer">{layer}</p>
  <Handle type="source" id="out" position={Position.Right} isConnectable={false} />
</div>

<style>
  .node {
    width: 200px;
    height: 80px;
    box-sizing: border-box;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--panel);
    color: var(--fg);
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.1rem;
    overflow: hidden;
  }

  .node.selected {
    border-color: var(--fg);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .label,
  .subtitle,
  .layer {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label {
    font-size: 0.82rem;
    font-weight: 650;
  }

  .subtitle,
  .layer {
    font-family: var(--mono);
    font-size: 0.72rem;
    color: var(--fg-muted);
  }
</style>
