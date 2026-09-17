import type { EditMode, Layer, StateView, ViewSelection } from '../../contracts/presentation';

export const CANONICAL_LAYERS: readonly Layer[] = ['L1', 'L2', 'L3', 'L4', 'L7'];

export const STATE_VIEW_OPTIONS: readonly { value: StateView; label: string }[] = [
  { value: 'intent', label: 'Intent' },
  { value: 'device_configuration', label: 'Device configuration' },
  { value: 'operational', label: 'Operational' },
  { value: 'compare', label: 'Compare' },
];

export const EDIT_MODE_OPTIONS: readonly { value: EditMode; label: string }[] = [
  { value: 'view', label: 'View' },
  { value: 'edit', label: 'Edit' },
];

export function overlayChoices(primary: Layer): Layer[] {
  return CANONICAL_LAYERS.filter((layer) => layer !== primary);
}

export function normalizeOverlays(
  overlays: readonly Layer[],
  primary: Layer,
): Layer[] {
  const present = new Set<Layer>(overlays);
  const next: Layer[] = [];
  for (const layer of CANONICAL_LAYERS) {
    if (layer !== primary && present.has(layer)) {
      next.push(layer);
    }
  }
  return next;
}

export function copySelection(
  value: ViewSelection,
  patch: Partial<ViewSelection> = {},
): ViewSelection {
  return {
    primaryLayer: patch.primaryLayer ?? value.primaryLayer,
    overlays:
      patch.overlays !== undefined ? [...patch.overlays] : [...value.overlays],
    combinedOverview: patch.combinedOverview ?? value.combinedOverview,
    stateView: patch.stateView ?? value.stateView,
    assuranceOverlay: patch.assuranceOverlay ?? value.assuranceOverlay,
    editMode: patch.editMode ?? value.editMode,
  };
}

export function selectionWithPrimary(
  value: ViewSelection,
  primary: Layer,
): ViewSelection {
  return {
    primaryLayer: primary,
    overlays: normalizeOverlays(value.overlays, primary),
    combinedOverview: value.combinedOverview,
    stateView: value.stateView,
    assuranceOverlay: value.assuranceOverlay,
    editMode: value.editMode,
  };
}

export function selectionWithOverlay(
  value: ViewSelection,
  layer: Layer,
  enabled: boolean,
): ViewSelection {
  const remaining = value.overlays.filter((item) => item !== layer);
  const nextOverlays = enabled ? [...remaining, layer] : remaining;
  return {
    primaryLayer: value.primaryLayer,
    overlays: normalizeOverlays(nextOverlays, value.primaryLayer),
    combinedOverview: value.combinedOverview,
    stateView: value.stateView,
    assuranceOverlay: value.assuranceOverlay,
    editMode: value.editMode,
  };
}

export function emitSelection(
  value: ViewSelection,
  patch: Partial<ViewSelection> = {},
): ViewSelection {
  const next = copySelection(value, patch);
  return {
    ...next,
    overlays: normalizeOverlays(next.overlays, next.primaryLayer),
  };
}

export function freezeSelection(value: ViewSelection): ViewSelection {
  return Object.freeze({
    primaryLayer: value.primaryLayer,
    overlays: Object.freeze([...value.overlays]),
    combinedOverview: value.combinedOverview,
    stateView: value.stateView,
    assuranceOverlay: value.assuranceOverlay,
    editMode: value.editMode,
  });
}
