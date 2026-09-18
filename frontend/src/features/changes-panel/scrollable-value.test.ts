import { describe, expect, it } from 'vitest';
import { nextScrollTop } from './scrollable-value';

describe('nextScrollTop', () => {
  it('does not invent a scroll offset when content fits', () => {
    expect(nextScrollTop(0, 100, 100, 'End')).toBeNull();
  });

  it('maps End/PageDown onto the overflow box, not the page', () => {
    expect(nextScrollTop(0, 400, 100, 'End')).toBe(300);
    expect(nextScrollTop(0, 400, 100, 'PageDown')).toBe(100);
    expect(nextScrollTop(40, 400, 100, 'Escape')).toBeNull();
  });
});
