import { describe, expect, it } from 'vitest';
import { classifyNewlines } from './newlines';

describe('classifyNewlines', () => {
  it('defaults documents without line breaks to LF, including empty source', () => {
    expect(classifyNewlines('')).toEqual({
      supported: true,
      kind: 'lf',
      separator: '\n',
    });
    expect(classifyNewlines('no-breaks\tkeep')).toEqual({
      supported: true,
      kind: 'lf',
      separator: '\n',
    });
  });

  it('accepts uniform LF with and without a final newline', () => {
    expect(classifyNewlines('a\nb')).toMatchObject({ kind: 'lf', supported: true });
    expect(classifyNewlines('a\nb\n')).toMatchObject({ kind: 'lf', supported: true });
  });

  it('accepts uniform CRLF with and without a final CRLF', () => {
    expect(classifyNewlines('a\r\nb')).toEqual({
      supported: true,
      kind: 'crlf',
      separator: '\r\n',
    });
    expect(classifyNewlines('a\r\nb\r\n')).toEqual({
      supported: true,
      kind: 'crlf',
      separator: '\r\n',
    });
  });

  it('rejects mixed LF/CRLF, standalone CR, and mixed CR with LF', () => {
    expect(classifyNewlines('a\nb\r\nc').supported).toBe(false);
    expect(classifyNewlines('a\rb').supported).toBe(false);
    expect(classifyNewlines('a\rb\nc').supported).toBe(false);
    expect(classifyNewlines('a\r\nb\rc').supported).toBe(false);
  });

  it('does not mutate or rewrite the inspected string', () => {
    const mixed = 'keep \t two  spaces\ncrlf\r\n';
    const frozen = Object.freeze(mixed);
    const result = classifyNewlines(frozen);
    expect(result.supported).toBe(false);
    expect(frozen).toBe(mixed);
  });
});
