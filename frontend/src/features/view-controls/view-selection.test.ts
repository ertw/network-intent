import { describe, expect, it } from 'vitest';
import type { ViewSelection } from '../../contracts/presentation';
import {
  CANONICAL_LAYERS,
  copySelection,
  emitSelection,
  freezeSelection,
  normalizeOverlays,
  overlayChoices,
  selectionWithOverlay,
  selectionWithPrimary,
} from './view-selection';

const base: ViewSelection = {
  primaryLayer: 'L3',
  overlays: ['L2', 'L1'],
  combinedOverview: false,
  stateView: 'intent',
  assuranceOverlay: false,
  editMode: 'view',
};

describe('canonical layers', () => {
  it('lists L1/L2/L3/L4/L7 and excludes L5/L6', () => {
    expect(CANONICAL_LAYERS).toEqual(['L1', 'L2', 'L3', 'L4', 'L7']);
    expect(CANONICAL_LAYERS).not.toContain('L5');
    expect(CANONICAL_LAYERS).not.toContain('L6');
  });
});

describe('overlayChoices', () => {
  it('excludes the current primary and keeps canonical order', () => {
    expect(overlayChoices('L3')).toEqual(['L1', 'L2', 'L4', 'L7']);
    expect(overlayChoices('L1')).toEqual(['L2', 'L3', 'L4', 'L7']);
    expect(overlayChoices('L7')).toEqual(['L1', 'L2', 'L3', 'L4']);
  });
});

describe('normalizeOverlays', () => {
  it('excludes the primary, drops duplicates, and sorts into canonical order', () => {
    expect(normalizeOverlays(['L7', 'L1', 'L7', 'L2', 'L3'], 'L3')).toEqual([
      'L1',
      'L2',
      'L7',
    ]);
  });

  it('does not mutate the input array', () => {
    const overlays = ['L2', 'L1', 'L2'] as ViewSelection['overlays'];
    const frozen = Object.freeze([...overlays]);
    expect(normalizeOverlays(frozen, 'L3')).toEqual(['L1', 'L2']);
    expect(frozen).toEqual(['L2', 'L1', 'L2']);
  });
});

describe('selectionWithPrimary', () => {
  it('removes the new primary from overlays and does not add the previous primary', () => {
    const next = selectionWithPrimary(base, 'L1');
    expect(next.primaryLayer).toBe('L1');
    expect(next.overlays).toEqual(['L2']);
    expect(next.overlays).not.toContain('L3');
    expect(next.stateView).toBe('intent');
    expect(next.editMode).toBe('view');
    expect(next.assuranceOverlay).toBe(false);
    expect(next.combinedOverview).toBe(false);
  });

  it('does not mutate the input selection', () => {
    const frozen = freezeSelection(base);
    expect(() => selectionWithPrimary(frozen, 'L2')).not.toThrow();
    expect(frozen.primaryLayer).toBe('L3');
    expect(frozen.overlays).toEqual(['L2', 'L1']);
  });
});

describe('selectionWithOverlay', () => {
  it('emits a new canonical overlays array when enabling a layer', () => {
    const next = selectionWithOverlay(base, 'L7', true);
    expect(next.overlays).toEqual(['L1', 'L2', 'L7']);
    expect(next.overlays).not.toBe(base.overlays);
    expect(base.overlays).toEqual(['L2', 'L1']);
  });

  it('removes a layer when disabling it', () => {
    const next = selectionWithOverlay(base, 'L2', false);
    expect(next.overlays).toEqual(['L1']);
  });

  it('cannot add the current primary as an overlay', () => {
    const next = selectionWithOverlay(base, 'L3', true);
    expect(next.overlays).toEqual(['L1', 'L2']);
  });
});

describe('copySelection combined overview', () => {
  it('retains primary and overlay values when combined overview is toggled', () => {
    const enabled = copySelection(base, { combinedOverview: true });
    expect(enabled.combinedOverview).toBe(true);
    expect(enabled.primaryLayer).toBe('L3');
    expect(enabled.overlays).toEqual(['L2', 'L1']);
    expect(enabled.overlays).not.toBe(base.overlays);

    const restored = copySelection(enabled, { combinedOverview: false });
    expect(restored.combinedOverview).toBe(false);
    expect(restored.primaryLayer).toBe('L3');
    expect(restored.overlays).toEqual(['L2', 'L1']);
  });

  it('copies overlays when other independent fields change', () => {
    const next = copySelection(base, {
      stateView: 'compare',
      assuranceOverlay: true,
      editMode: 'edit',
    });
    expect(next.primaryLayer).toBe('L3');
    expect(next.overlays).toEqual(['L2', 'L1']);
    expect(next.overlays).not.toBe(base.overlays);
    expect(next.stateView).toBe('compare');
    expect(next.assuranceOverlay).toBe(true);
    expect(next.editMode).toBe('edit');
    expect(base.stateView).toBe('intent');
    expect(base.editMode).toBe('view');
  });
});

describe('emitSelection', () => {
  it('canonicalizes overlays on every emit without changing other fields', () => {
    const messy: ViewSelection = {
      ...base,
      overlays: ['L3', 'L7', 'L1', 'L1'],
    };
    const next = emitSelection(messy, { stateView: 'operational' });
    expect(next.stateView).toBe('operational');
    expect(next.primaryLayer).toBe('L3');
    expect(next.overlays).toEqual(['L1', 'L7']);
    expect(next.assuranceOverlay).toBe(false);
    expect(next.editMode).toBe('view');
    expect(messy.overlays).toEqual(['L3', 'L7', 'L1', 'L1']);
  });

  it('retains overlay membership when combined overview is toggled', () => {
    const next = emitSelection(base, { combinedOverview: true });
    expect(next.combinedOverview).toBe(true);
    expect(next.primaryLayer).toBe('L3');
    expect(next.overlays).toEqual(['L1', 'L2']);
  });
});
