import { describe, expect, it, vi } from 'vitest';
import { activate, clearSelection, isSelected, select, selection } from './selection.svelte';

describe('selection', () => {
  it('selects one item at a time; occurrences of a repeating event are distinct', () => {
    select({ kind: 'event', id: 1, occurrence: '2026-10-20T07:30:00Z' });
    expect(isSelected('event', 1, '2026-10-20T07:30:00Z')).toBe(true);
    expect(isSelected('event', 1, '2026-10-27T07:30:00Z')).toBe(false);
    select({ kind: 'task', id: 1 });
    expect(isSelected('event', 1, '2026-10-20T07:30:00Z')).toBe(false);
    expect(isSelected('task', 1)).toBe(true);
    clearSelection();
    expect(selection.current).toBeNull();
  });
});

describe('activate (click on an item)', () => {
  const run = (detail: number) => {
    const open = vi.fn();
    const pick = vi.fn();
    activate({ detail } as MouseEvent, open, pick);
    return { open: open.mock.calls.length, pick: pick.mock.calls.length };
  };
  it('a mouse click selects', () => expect(run(1)).toEqual({ open: 0, pick: 1 }));
  it('a keyboard or screen-reader activation (no click count) opens', () => expect(run(0)).toEqual({ open: 1, pick: 0 }));
});
