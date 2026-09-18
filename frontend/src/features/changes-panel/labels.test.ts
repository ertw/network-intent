import { describe, expect, it } from 'vitest';
import {
  explanationDisplay,
  kindLabel,
  noChangesLabel,
  noFieldsLabel,
} from './labels';

describe('changes-panel labels', () => {
  it('maps the four supplied kinds without inferring safety', () => {
    expect(kindLabel('added')).toBe('Added');
    expect(kindLabel('removed')).toBe('Removed');
    expect(kindLabel('modified')).toBe('Modified');
    expect(kindLabel('blocked')).toBe('Blocked');
  });

  it('treats a null explanation as not supplied, not as a safety proof', () => {
    expect(explanationDisplay(null)).toBe('No explanation supplied');
    expect(explanationDisplay('blocked by admission')).toBe('blocked by admission');
    expect(noChangesLabel()).toBe('No changes supplied');
    expect(noFieldsLabel()).toBe('No fields supplied');
  });
});
