// The selected task or event in the main window: a single click selects,
// a double click (or Enter) opens it in the item dialog.

export type Selected = { kind: 'event' | 'task'; id: number; occurrence?: string | null };

export const selection = $state<{ current: Selected | null }>({ current: null });

export function select(item: Selected) {
  selection.current = item;
}

export function clearSelection() {
  selection.current = null;
}

export function isSelected(kind: Selected['kind'], id: number, occurrence: string | null = null): boolean {
  const s = selection.current;
  return !!s && s.kind === kind && s.id === id && (kind === 'task' || (s.occurrence ?? null) === occurrence);
}

/**
 * A click on an item: a mouse click (detail ≥ 1) selects it; a keyboard or
 * screen-reader activation (detail 0) opens it, since there is no double click.
 */
export function activate(e: MouseEvent, open: () => void, pick: () => void) {
  if (e.detail === 0) open();
  else pick();
}
