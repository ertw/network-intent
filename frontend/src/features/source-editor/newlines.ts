export type SupportedNewlines =
  | { supported: true; kind: 'lf'; separator: '\n' }
  | { supported: true; kind: 'crlf'; separator: '\r\n' };

export type UnsupportedNewlines = {
  supported: false;
  kind: 'unsupported';
  reason: string;
};

export type NewlineClassification = SupportedNewlines | UnsupportedNewlines;

export const UNSUPPORTED_NEWLINE_EXPLANATION =
  'This source uses mixed or unsupported line separators. The original text is shown unchanged and cannot be edited in this widget. Mixed LF/CRLF and standalone CR are not supported.';

/**
 * Classify uniform LF/CRLF. Documents with no line breaks default to LF.
 * Mixed LF/CRLF or any standalone CR is unsupported and must not be normalized.
 */
export function classifyNewlines(source: string): NewlineClassification {
  let hasBareLf = false;
  let hasCrlf = false;
  let hasBareCr = false;

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (character === '\r') {
      if (source[index + 1] === '\n') {
        hasCrlf = true;
        index += 1;
      } else {
        hasBareCr = true;
      }
    } else if (character === '\n') {
      hasBareLf = true;
    }
  }

  if (hasBareCr || (hasCrlf && hasBareLf)) {
    const parts: string[] = [];
    if (hasBareLf) {
      parts.push('LF');
    }
    if (hasCrlf) {
      parts.push('CRLF');
    }
    if (hasBareCr) {
      parts.push('standalone CR');
    }
    const mix = parts.join(' and ');
    return {
      supported: false,
      kind: 'unsupported',
      reason: `${mix} line separators are not supported.`,
    };
  }

  if (hasCrlf) {
    return { supported: true, kind: 'crlf', separator: '\r\n' };
  }

  return { supported: true, kind: 'lf', separator: '\n' };
}
