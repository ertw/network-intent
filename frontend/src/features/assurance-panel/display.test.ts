import { describe, expect, it } from 'vitest';
import type { Completeness, Freshness, Outcome } from '../../contracts/presentation';
import { allowsPositiveAccent, outcomeLabel, timestampLabel } from './display';

const OUTCOMES: readonly Outcome[] = [
  'success',
  'violation',
  'timeout',
  'refused',
  'unreachable',
  'dns_nx_domain',
  'dns_server_failure',
  'tls_failure',
  'malformed_response',
  'unsupported',
  'unavailable',
];

describe('outcomeLabel', () => {
  it('keeps every supplied outcome on a distinct label', () => {
    const labels = OUTCOMES.map((outcome) => outcomeLabel(outcome));
    expect(labels).toEqual([
      'Success',
      'Violation',
      'Timeout',
      'Refused',
      'Unreachable',
      'DNS NXDOMAIN',
      'DNS server failure',
      'TLS failure',
      'Malformed response',
      'Unsupported',
      'Unavailable',
    ]);
    expect(new Set(labels).size).toBe(OUTCOMES.length);
  });

  it('does not merge timeout with violation or unsupported', () => {
    expect(outcomeLabel('timeout')).toBe('Timeout');
    expect(outcomeLabel('timeout')).not.toBe(outcomeLabel('violation'));
    expect(outcomeLabel('timeout')).not.toBe(outcomeLabel('unsupported'));
  });

  it('renders a null outcome as No observation', () => {
    expect(outcomeLabel(null)).toBe('No observation');
  });
});

describe('allowsPositiveAccent', () => {
  const freshnessValues: readonly Freshness[] = ['fresh', 'stale', 'unknown'];
  const completenessValues: readonly Completeness[] = ['complete', 'partial', 'unavailable'];

  it('allows a positive accent only for fresh and complete success', () => {
    expect(allowsPositiveAccent('success', 'fresh', 'complete')).toBe(true);
  });

  it('keeps stale, unknown, partial, and unavailable success visually neutral', () => {
    expect(allowsPositiveAccent('success', 'stale', 'complete')).toBe(false);
    expect(allowsPositiveAccent('success', 'unknown', 'complete')).toBe(false);
    expect(allowsPositiveAccent('success', 'fresh', 'partial')).toBe(false);
    expect(allowsPositiveAccent('success', 'fresh', 'unavailable')).toBe(false);
    expect(allowsPositiveAccent('success', 'stale', 'partial')).toBe(false);
  });

  it('never treats a non-success outcome as a positive accent', () => {
    for (const outcome of OUTCOMES) {
      if (outcome === 'success') {
        continue;
      }
      for (const freshness of freshnessValues) {
        for (const completeness of completenessValues) {
          expect(allowsPositiveAccent(outcome, freshness, completeness)).toBe(false);
        }
      }
    }
    expect(allowsPositiveAccent(null, 'fresh', 'complete')).toBe(false);
  });
});

describe('timestampLabel', () => {
  it('keeps supplied timestamps verbatim and uses Unknown for null', () => {
    expect(timestampLabel('2026-09-17T16:00:00.000Z')).toBe('2026-09-17T16:00:00.000Z');
    expect(timestampLabel(null)).toBe('Unknown');
  });
});
