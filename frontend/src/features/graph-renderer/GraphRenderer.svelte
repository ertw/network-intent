<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    Background,
    Controls,
    MarkerType,
    SvelteFlow,
    type Edge,
    type Node,
    type NodeTypes,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import type { GraphRendererProps } from '../../contracts/presentation';
  import GraphCanvasNode from './GraphCanvasNode.svelte';
  import { createBundledLayout } from './elk-layout';
  import { toFlowInputs, type NodePosition } from './graph-map';
  import { LayoutSession } from './layout-session';
  import { issueMessage, validateGraph } from './validate';

  let { nodes, edges, selectedId, onSelect }: GraphRendererProps = $props();

  const instance = $props.id();
  const headingId = `${instance}-graph-heading`;
  const listId = `${instance}-graph-list`;
  const nodeTypes: NodeTypes = { 'intent-node': GraphCanvasNode };
  const layout = createBundledLayout();

  type Display =
    | { status: 'empty' }
    | { status: 'loading' }
    | { status: 'invalid'; messages: string[] }
    | { status: 'error'; message: string }
    | { status: 'ready'; positions: NodePosition[] };

  let display = $state<Display>({ status: 'loading' });
  let flowNodes = $state.raw<Node[]>([]);
  let flowEdges = $state.raw<Edge[]>([]);

  const suppliedNodes = $derived(nodes);
  const suppliedEdges = $derived(edges);

  const session = new LayoutSession(layout, (result) => {
    display = result.status === 'ready'
      ? { status: 'ready', positions: result.positions }
      : { status: 'error', message: result.message };
  });
  onDestroy(() => session.dispose());

  $effect(() => {
    const currentNodes = suppliedNodes;
    const currentEdges = suppliedEdges;
    session.invalidate();

    if (currentNodes.length === 0 && currentEdges.length === 0) {
      display = { status: 'empty' };
      flowNodes = [];
      flowEdges = [];
      return () => session.invalidate();
    }

    const validity = validateGraph(currentNodes, currentEdges);
    if (!validity.ok) {
      display = {
        status: 'invalid',
        messages: validity.issues.map(issueMessage),
      };
      flowNodes = [];
      flowEdges = [];
      return () => session.invalidate();
    }

    display = { status: 'loading' };
    flowNodes = [];
    flowEdges = [];
    session.request(currentNodes, currentEdges);
    return () => session.invalidate();
  });

  $effect(() => {
    const current = display;
    const sel = selectedId;
    const currentNodes = suppliedNodes;
    const currentEdges = suppliedEdges;
    if (current.status !== 'ready') {
      return;
    }
    const mapped = toFlowInputs(currentNodes, currentEdges, current.positions, sel);
    flowNodes = mapped.nodes.map((node) => ({
      id: node.id,
      type: 'intent-node',
      position: { x: node.x, y: node.y },
      data: {
        id: node.id,
        label: node.label,
        subtitle: node.subtitle,
        layer: node.layer,
        x: node.x,
        y: node.y,
      },
      selected: node.selected,
      draggable: false,
      connectable: false,
      deletable: false,
      style: 'width: 200px; height: 80px;',
    }));
    flowEdges = mapped.edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      sourceHandle: 'out',
      targetHandle: 'in',
      label: edge.label ?? undefined,
      selectable: true,
      deletable: false,
      selected: edge.selected,
      markerEnd: { type: MarkerType.ArrowClosed },
    }));
  });

  function selectSupplied(id: string) {
    onSelect(id);
  }

  function selectBackground() {
    onSelect(null);
  }

  // Flow may update its bound selection even when the parent rejects a request.
  $effect(() => {
    if (flowNodes.some((node) => !!node.selected !== (node.id === selectedId))) {
      flowNodes = flowNodes.map((node) => ({ ...node, selected: node.id === selectedId }));
    }
    if (flowEdges.some((edge) => !!edge.selected !== (edge.id === selectedId))) {
      flowEdges = flowEdges.map((edge) => ({ ...edge, selected: edge.id === selectedId }));
    }
  });

  function selectFromKeyboard(event: KeyboardEvent) {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey ||
        !['Enter', ' ', 'Escape'].includes(event.key)) return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    const element = target.closest('.svelte-flow__node, .svelte-flow__edge');
    const id = element?.getAttribute('data-id');
    if (id === null || id === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    onSelect(event.key === 'Escape' ? null : id);
  }
</script>

<section
  class="renderer"
  data-testid="graph-renderer"
  data-status={display.status}
  aria-labelledby={headingId}
>
  <h3 id={headingId}>Supplied graph</h3>

  {#if display.status === 'empty'}
    <p data-testid="graph-empty">No graph to display</p>
  {:else if display.status === 'invalid'}
    <div class="diagnostic" data-testid="graph-invalid" role="alert">
      <p>Graph input is invalid. Supplied data was not discarded.</p>
      <ul>
        {#each display.messages as message, index (index)}
          <li data-testid="graph-invalid-issue">{message}</li>
        {/each}
      </ul>
    </div>
  {:else if display.status === 'error'}
    <p class="diagnostic" data-testid="graph-error" role="alert">
      Graph layout failed: {display.message}
    </p>
  {:else if display.status === 'loading'}
    <p data-testid="graph-loading">Laying out graph</p>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="canvas-wrap" data-testid="graph-canvas" onkeydowncapture={selectFromKeyboard}>
      <SvelteFlow
        id={instance}
        bind:nodes={flowNodes}
        bind:edges={flowEdges}
        {nodeTypes}
        colorMode="light"
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={true}
        selectionOnDrag={false}
        deleteKey={null}
        clickConnect={false}
        panOnDrag={true}
        zoomOnScroll={true}
        zoomOnPinch={true}
        zoomOnDoubleClick={true}
        fitView={true}
        minZoom={0.15}
        maxZoom={2}
        proOptions={{ hideAttribution: false }}
        isValidConnection={() => false}
        onbeforeconnect={() => false}
        onbeforedelete={async () => false}
        onnodeclick={({ node }) => selectSupplied(node.id)}
        onedgeclick={({ edge }) => selectSupplied(edge.id)}
        onpaneclick={selectBackground}
      >
        <Background />
        <Controls showLock={false} showZoom={true} showFitView={true} />
      </SvelteFlow>
    </div>
  {/if}

  {#if suppliedNodes.length > 0 || suppliedEdges.length > 0}
    <div class="lists">
      <h4 id={`${listId}-nodes`}>Nodes</h4>
      <ul class="select-list" aria-labelledby={`${listId}-nodes`} data-testid="graph-node-list">
        {#each suppliedNodes as node, index (index)}
          <li>
            <button
              type="button"
              class="item"
              class:selected={selectedId === node.id}
              data-testid="graph-node-item"
              data-node-id={node.id}
              data-layer={node.layer}
              aria-pressed={selectedId === node.id}
              onclick={() => selectSupplied(node.id)}
            >
              <span class="item-label">{node.label}</span>
              {#if node.subtitle !== null}
                <span class="item-sub">{node.subtitle}</span>
              {/if}
              <span class="item-meta">{node.layer} · {node.id}</span>
            </button>
          </li>
        {/each}
      </ul>
      <h4 id={`${listId}-edges`}>Edges</h4>
      <ul class="select-list" aria-labelledby={`${listId}-edges`} data-testid="graph-edge-list">
        {#each suppliedEdges as edge, index (index)}
          <li>
            <button
              type="button"
              class="item"
              class:selected={selectedId === edge.id}
              data-testid="graph-edge-item"
              data-edge-id={edge.id}
              data-layer={edge.layer}
              aria-pressed={selectedId === edge.id}
              onclick={() => selectSupplied(edge.id)}
            >
              <span class="item-label">{edge.label === null ? 'No edge label' : edge.label}</span>
              <span class="item-meta">{edge.source} → {edge.target} · {edge.layer} · {edge.id}</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</section>

<style>
  .renderer {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
    max-width: 100%;
  }

  h3,
  h4 {
    margin: 0;
    font-size: 1.05rem;
    line-height: 1.3;
    font-weight: 650;
  }

  h4 {
    font-size: 0.95rem;
  }

  .canvas-wrap {
    width: 100%;
    min-width: 0;
    height: 20rem;
    max-height: 40vh;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    overflow: hidden;
    background: var(--bg);
  }

  .canvas-wrap :global(.svelte-flow) {
    width: 100%;
    height: 100%;
  }

  .diagnostic {
    color: var(--danger);
    border: 1px solid var(--danger);
    border-radius: 0.3rem;
    padding: 0.55rem 0.7rem;
    background: var(--bg);
    margin: 0;
  }

  .diagnostic ul {
    margin: 0.35rem 0 0;
    padding-left: 1.1rem;
  }

  .lists {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-width: 0;
  }

  .select-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .item {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.15rem;
    width: 100%;
    min-width: 0;
    text-align: left;
    margin: 0;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--panel);
    color: var(--fg);
    font: inherit;
    cursor: pointer;
  }

  .item.selected {
    border-color: var(--fg);
    background: color-mix(in srgb, var(--border) 28%, var(--panel));
  }

  .item-label,
  .item-sub,
  .item-meta {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .item-sub,
  .item-meta {
    font-family: var(--mono);
    font-size: 0.78rem;
    color: var(--fg-muted);
  }
</style>
