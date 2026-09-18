import { describe, expect, it } from 'vitest';
import {
  CRLF_WITH_FINAL,
  FIXTURES,
  LF_WITH_FINAL,
  MIXED_NEWLINES,
  UNICODE_SOURCE,
} from './fixtures';
import { classifyNewlines } from './newlines';

describe('source-editor fixtures', () => {
  it('keeps LF, CRLF, mixed, and Unicode samples distinct and unnormalized', () => {
    expect(LF_WITH_FINAL.endsWith('\n')).toBe(true);
    expect(LF_WITH_FINAL.includes('\r')).toBe(false);
    expect(CRLF_WITH_FINAL.includes('\r\n')).toBe(true);
    expect(CRLF_WITH_FINAL.includes('\n') && !CRLF_WITH_FINAL.replaceAll('\r\n', '').includes('\n')).toBe(
      true,
    );
    expect(MIXED_NEWLINES.includes('\n')).toBe(true);
    expect(MIXED_NEWLINES.includes('\r\n')).toBe(true);
    expect(UNICODE_SOURCE.includes('😀')).toBe(true);
    expect(UNICODE_SOURCE.includes('\uD83D\uDE00')).toBe(true);
  });

  it('classifies fixture sources the way the widget should', () => {
    expect(classifyNewlines(LF_WITH_FINAL).kind).toBe('lf');
    expect(classifyNewlines(CRLF_WITH_FINAL).kind).toBe('crlf');
    expect(classifyNewlines(MIXED_NEWLINES).supported).toBe(false);
    expect(FIXTURES.map((item) => item.key)).toContain('empty');
  });
});
