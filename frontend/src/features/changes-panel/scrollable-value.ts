/** Keyboard movement for a read-only overflow box. Not an editor. */

export function nextScrollTop(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  key: string,
): number | null {
  const maxScroll = scrollHeight - clientHeight;
  if (maxScroll <= 0) {
    return null;
  }

  switch (key) {
    case 'Home':
      return 0;
    case 'End':
      return maxScroll;
    case 'PageDown':
      return Math.min(maxScroll, scrollTop + clientHeight);
    case 'PageUp':
      return Math.max(0, scrollTop - clientHeight);
    case 'ArrowDown':
      return Math.min(maxScroll, scrollTop + 16);
    case 'ArrowUp':
      return Math.max(0, scrollTop - 16);
    default:
      return null;
  }
}

export function applyScrollKey(element: HTMLElement, key: string): boolean {
  const next = nextScrollTop(
    element.scrollTop,
    element.scrollHeight,
    element.clientHeight,
    key,
  );
  if (next === null) {
    return false;
  }
  element.scrollTop = next;
  return true;
}
