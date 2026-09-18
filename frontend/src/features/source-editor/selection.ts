import type { EditorSelection } from '../../contracts/presentation';

/**
 * Clamp CodeMirror UTF-16 document offsets. Non-finite values are ignored.
 * Offsets are editor positions, never compiler source spans.
 */
export function clampSelection(
  selection: EditorSelection,
  documentLength: number,
): EditorSelection | null {
  const { anchor, head } = selection;
  if (!Number.isFinite(anchor) || !Number.isFinite(head)) {
    return null;
  }
  const length = Math.max(0, documentLength);
  return {
    anchor: Math.min(length, Math.max(0, Math.trunc(anchor))),
    head: Math.min(length, Math.max(0, Math.trunc(head))),
  };
}

export function selectionsEqual(
  left: EditorSelection | null,
  right: EditorSelection | null,
): boolean {
  if (left === null || right === null) {
    return left === right;
  }
  return left.anchor === right.anchor && left.head === right.head;
}
