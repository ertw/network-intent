import { describe, expect, it } from 'vitest';
import {
  ALL_OUTCOME_ROW_IDS,
  ALL_OUTCOME_ROWS,
  HTML_LIKE_LABEL,
  LARGE_PLAN_ID,
  LARGE_ROW_ID,
} from './fixtures';

describe('synthetic assurance fixtures', () => {
  it('keeps large-looking plan and row IDs as exact decimal strings', () => {
    expect(LARGE_PLAN_ID).toBe('9007199254740993');
    expect(LARGE_ROW_ID).toBe('9007199254740993');
    expect(Number(LARGE_PLAN_ID).toString()).not.toBe(LARGE_PLAN_ID);
  });

  it('freezes supplied rows and preserves order', () => {
    expect(Object.isFrozen(ALL_OUTCOME_ROWS)).toBe(true);
    expect(ALL_OUTCOME_ROWS[0]?.id).toBe(LARGE_ROW_ID);
    expect(ALL_OUTCOME_ROWS.map((row) => row.id)).toEqual([...ALL_OUTCOME_ROW_IDS]);
    expect(Object.isFrozen(ALL_OUTCOME_ROWS[0])).toBe(true);
    expect(Object.isFrozen(ALL_OUTCOME_ROWS[0]?.dependsOn)).toBe(true);
    expect(Object.isFrozen(ALL_OUTCOME_ROWS[0]?.provenance.missing)).toBe(true);
  });

  it('keeps HTML-like labels as literal text data', () => {
    const htmlLike = ALL_OUTCOME_ROWS.find((row) => row.id === 'html-like');
    expect(htmlLike?.label).toBe(HTML_LIKE_LABEL);
    expect(htmlLike?.detail.includes('<img')).toBe(true);
  });
});
