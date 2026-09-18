<script lang="ts">
  import { tick, untrack } from 'svelte';
  import {
    Annotation,
    Compartment,
    EditorSelection as CmSelection,
    EditorState,
    Prec,
    Transaction,
    type Extension,
    type TransactionSpec,
  } from '@codemirror/state';
  import { standardKeymap } from '@codemirror/commands';
  import {
    EditorView,
    drawSelection,
    highlightSpecialChars,
    keymap,
    lineNumbers,
  } from '@codemirror/view';
  import type { EditorSelection, SourceEditorProps } from '../../contracts/presentation';
  import {
    UNSUPPORTED_NEWLINE_EXPLANATION,
    classifyNewlines,
    type NewlineClassification,
  } from './newlines';
  import { clampSelection, selectionsEqual } from './selection';

  let {
    source,
    editable,
    selection,
    onChange,
    onSelectionChange,
    onHistoryRequest,
  }: SourceEditorProps = $props();

  const instance = $props.id();
  const headingId = `${instance}-source-editor-heading`;
  const externalAnnotation = Annotation.define<boolean>();
  const lineSepCompartment = new Compartment();
  const readOnlyCompartment = new Compartment();
  const editableCompartment = new Compartment();

  const live = {
    get source() {
      return source;
    },
    get editable() {
      return editable;
    },
    get selection() {
      return selection;
    },
    get onChange() {
      return onChange;
    },
    get onSelectionChange() {
      return onSelectionChange;
    },
    get onHistoryRequest() {
      return onHistoryRequest;
    },
  };

  const newline = $derived(classifyNewlines(source));
  const supported = $derived(newline.supported);

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let applyingExternal = false;

  function isExternal(transactions: readonly Transaction[]): boolean {
    return transactions.some((transaction) => transaction.annotation(externalAnnotation) === true);
  }

  function emitHistory(direction: 'undo' | 'redo'): boolean {
    if (!live.editable) {
      return true;
    }
    live.onHistoryRequest(direction);
    return true;
  }

  function historyKeymap(): Extension {
    return Prec.highest(
      keymap.of([
        {
          key: 'Mod-z',
          run: () => emitHistory('undo'),
          preventDefault: true,
        },
        {
          key: 'Mod-Shift-z',
          run: () => emitHistory('redo'),
          preventDefault: true,
        },
        {
          key: 'Ctrl-y',
          run: () => emitHistory('redo'),
          preventDefault: true,
        },
        {
          key: 'Ctrl-Shift-z',
          run: () => emitHistory('redo'),
          preventDefault: true,
        },
      ]),
    );
  }

  function preventViewModeEditing(): Extension {
    return [
      EditorState.changeFilter.of((transaction) => {
        if (!transaction.docChanged) {
          return true;
        }
        if (transaction.annotation(externalAnnotation) === true) {
          return true;
        }
        if (transaction.startState.readOnly) {
          return false;
        }
        return true;
      }),
      EditorState.transactionFilter.of((transaction) => {
        if (!transaction.docChanged) {
          return transaction;
        }
        if (transaction.annotation(externalAnnotation) === true) {
          return transaction;
        }
        if (transaction.startState.readOnly) {
          return [];
        }
        return transaction;
      }),
      EditorView.domEventHandlers({
        paste(event, current) {
          if (current.state.readOnly) {
            event.preventDefault();
            return true;
          }
          return false;
        },
        cut(event, current) {
          if (current.state.readOnly) {
            event.preventDefault();
            return true;
          }
          return false;
        },
        drop(event, current) {
          if (current.state.readOnly) {
            event.preventDefault();
            return true;
          }
          return false;
        },
      }),
    ];
  }

  function currentEditorSelection(current: EditorView): EditorSelection {
    const range = current.state.selection.main;
    return { anchor: range.anchor, head: range.head };
  }

  function applyExternal(current: EditorView, spec: TransactionSpec): void {
    applyingExternal = true;
    current.dispatch({
      ...spec,
      annotations: [
        externalAnnotation.of(true),
        ...(Array.isArray(spec.annotations)
          ? spec.annotations
          : spec.annotations
            ? [spec.annotations]
            : []),
      ],
    });
    applyingExternal = false;
  }

  function syncFromProps(current: EditorView): void {
    const classified: NewlineClassification = classifyNewlines(live.source);
    if (!classified.supported) {
      return;
    }

    if (current.state.lineBreak !== classified.separator) {
      applyExternal(current, {
        effects: [lineSepCompartment.reconfigure(EditorState.lineSeparator.of(classified.separator))],
      });
    }

    const modeEffects = [];
    const readOnly = !live.editable;
    if (current.state.readOnly !== readOnly) {
      modeEffects.push(readOnlyCompartment.reconfigure(EditorState.readOnly.of(readOnly)));
      modeEffects.push(editableCompartment.reconfigure(EditorView.editable.of(live.editable)));
    }

    const serialized = current.state.sliceDoc();
    const documentChanged = serialized !== live.source;
    const spec: TransactionSpec = { effects: modeEffects };

    if (documentChanged) {
      spec.changes = {
        from: 0,
        to: current.state.doc.length,
        insert: live.source,
      };
    }

    const nextLength = documentChanged
      ? current.state.toText(live.source).length
      : current.state.doc.length;

    if (live.selection !== null) {
      const clamped = clampSelection(live.selection, nextLength);
      if (clamped) {
        const present = documentChanged
          ? null
          : currentEditorSelection(current);
        if (!present || !selectionsEqual(present, clamped)) {
          spec.selection = CmSelection.create([CmSelection.range(clamped.anchor, clamped.head)]);
        }
      }
    }

    const hasEffects = modeEffects.length > 0;
    if (documentChanged || spec.selection || hasEffects) {
      applyExternal(current, spec);
    }
  }

  function createView(parent: HTMLDivElement, initial: string, canEdit: boolean): EditorView {
    const classified = classifyNewlines(initial);
    const separator = classified.supported ? classified.separator : '\n';
    const readOnly = !canEdit;

    return new EditorView({
      parent,
      state: EditorState.create({
        doc: classified.supported ? initial : '',
        extensions: [
          lineNumbers(),
          drawSelection(),
          highlightSpecialChars(),
          EditorView.lineWrapping,
          lineSepCompartment.of(EditorState.lineSeparator.of(separator)),
          readOnlyCompartment.of(EditorState.readOnly.of(readOnly)),
          editableCompartment.of(EditorView.editable.of(canEdit)),
          preventViewModeEditing(),
          historyKeymap(),
          keymap.of(standardKeymap),
          EditorView.contentAttributes.of({
            'data-testid': 'source-editor-content',
            'aria-labelledby': headingId,
          }),
          EditorView.theme({
            '&': {
              height: '16rem',
              border: '1px solid var(--border)',
              borderRadius: '0.3rem',
              background: 'var(--panel)',
              fontFamily: 'var(--mono)',
            },
            '.cm-scroller': {
              overflow: 'auto',
              fontFamily: 'var(--mono)',
              fontSize: '0.88rem',
              lineHeight: '1.45',
            },
            '.cm-content': {
              caretColor: 'var(--fg)',
            },
            '&.cm-focused': {
              outline: '3px solid var(--focus)',
              outlineOffset: '3px',
            },
          }),
          EditorView.updateListener.of((update) => {
            if (applyingExternal || isExternal(update.transactions)) {
              return;
            }
            if (update.docChanged) {
              live.onChange(update.state.sliceDoc());
            }
            if (update.selectionSet) {
              const range = update.state.selection.main;
              live.onSelectionChange({ anchor: range.anchor, head: range.head });
            }
            if (update.docChanged || (update.selectionSet && live.selection !== null)) {
              void tick().then(() => {
                if (view) {
                  syncFromProps(view);
                }
              });
            }
          }),
        ],
      }),
    });
  }

  $effect(() => {
    const parent = host;
    const canMount = supported;
    if (!parent || !canMount) {
      return;
    }

    const initialSource = untrack(() => source);
    const initialEditable = untrack(() => editable);
    const created = createView(parent, initialSource, initialEditable);
    view = created;
    untrack(() => syncFromProps(created));

    return () => {
      created.destroy();
      if (view === created) {
        view = null;
      }
    };
  });

  $effect(() => {
    source;
    editable;
    selection;
    const current = view;
    if (!current || !supported) {
      return;
    }
    syncFromProps(current);
  });
</script>

<section
  class="editor"
  data-testid="source-editor"
  data-editable={editable ? 'true' : 'false'}
  data-newline={newline.kind}
  aria-labelledby={headingId}
>
  <h3 id={headingId}>Source</h3>

  {#if !newline.supported}
    <div class="unsupported" data-testid="source-editor-unsupported" role="status">
      <p data-testid="source-editor-unsupported-reason">{UNSUPPORTED_NEWLINE_EXPLANATION}</p>
      <p class="detail">{newline.reason}</p>
      <pre class="raw" data-testid="source-editor-raw">{source}</pre>
    </div>
  {:else}
    <div
      bind:this={host}
      class="host"
      data-testid="source-editor-host"
      data-instance={instance}
    ></div>
  {/if}
</section>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    min-width: 0;
    max-width: 100%;
  }

  h3 {
    margin: 0;
    font-size: 1.05rem;
    line-height: 1.3;
    font-weight: 650;
  }

  .host {
    min-width: 0;
    max-width: 100%;
  }

  .unsupported {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-width: 0;
  }

  .unsupported p {
    margin: 0;
  }

  .detail {
    color: var(--fg-muted);
    font-size: 0.9rem;
  }

  .raw {
    margin: 0;
    max-height: 16rem;
    overflow: auto;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--bg);
    font-family: var(--mono);
    font-size: 0.85rem;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  :global(.cm-editor) {
    max-width: 100%;
  }
</style>
