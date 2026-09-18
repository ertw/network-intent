import type { ChangeEntry } from '../../contracts/presentation';

export const KIND_LABELS = {
  added: 'Added',
  removed: 'Removed',
  modified: 'Modified',
  blocked: 'Blocked',
} as const satisfies Record<ChangeEntry['kind'], string>;

export function kindLabel(kind: ChangeEntry['kind']): string {
  return KIND_LABELS[kind];
}

export function explanationDisplay(explanation: string | null): string {
  return explanation === null ? 'No explanation supplied' : explanation;
}

export function emptyValueLabel(): string {
  return 'Empty value';
}

export function emptyListLabel(): string {
  return 'Empty list';
}

export function noFieldsLabel(): string {
  return 'No fields supplied';
}

export function noChangesLabel(): string {
  return 'No changes supplied';
}
