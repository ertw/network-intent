import { describe, expect, it } from 'vitest';
import { clampSelection, selectionsEqual } from './selection';

describe('clampSelection', () => {
  it('ignores non-finite endpoints instead of inventing a caret', () => {
    expect(clampSelection({ anchor: Number.NaN, head: 1 }, 10)).toBeNull();
    expect(clampSelection({ anchor: 0, head: Number.POSITIVE_INFINITY }, 10)).toBeNull();
    expect(clampSelection({ anchor: Number.NEGATIVE_INFINITY, head: 2 }, 10)).toBeNull();
  });

  it('truncates finite offsets and clamps them to the internal document length', () => {
    expect(clampSelection({ anchor: -1.8, head: 99.2 }, 4)).toEqual({
      anchor: 0,
      head: 4,
    });
    expect(clampSelection({ anchor: 1.9, head: 2.2 }, 3)).toEqual({
      anchor: 1,
      head: 2,
    });
    expect(clampSelection({ anchor: 0, head: 0 }, 0)).toEqual({
      anchor: 0,
      head: 0,
    });
  });
});

describe('selectionsEqual', () => {
  it('treats null as no external override and compares finite offsets exactly', () => {
    expect(selectionsEqual(null, null)).toBe(true);
    expect(selectionsEqual(null, { anchor: 0, head: 0 })).toBe(false);
    expect(selectionsEqual({ anchor: 1, head: 2 }, { anchor: 1, head: 2 })).toBe(true);
    expect(selectionsEqual({ anchor: 1, head: 2 }, { anchor: 2, head: 1 })).toBe(false);
  });
});
