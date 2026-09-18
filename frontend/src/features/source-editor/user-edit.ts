import { EditorView } from '@codemirror/view';

/**
 * Fixture/test helper: dispatch a CodeMirror user-edit transaction.
 * Does not expose the EditorView to product parents.
 */
export function tryInsertAsUser(host: HTMLElement, text: string): boolean {
  const view = EditorView.findFromDOM(host);
  if (!view) {
    return false;
  }
  const insertion = view.state.selection.main.head;
  view.dispatch({
    changes: { from: insertion, insert: text },
    userEvent: 'input.type',
  });
  return true;
}
