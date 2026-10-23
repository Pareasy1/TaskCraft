import { describe, expect, it } from 'vitest';
import { menuStep } from './menu.svelte';

describe('menuStep', () => {
  it('moves through the items and wraps', () => {
    expect(menuStep('ArrowDown', 0, 3)).toBe(1);
    expect(menuStep('ArrowRight', 2, 3)).toBe(0);
    expect(menuStep('ArrowUp', 0, 3)).toBe(2);
    expect(menuStep('ArrowLeft', 1, 3)).toBe(0);
    expect(menuStep('Home', 2, 3)).toBe(0);
    expect(menuStep('End', 0, 3)).toBe(2);
  });
  it('starts from the ends when nothing is focused', () => {
    expect(menuStep('ArrowDown', -1, 3)).toBe(0);
    expect(menuStep('ArrowUp', -1, 3)).toBe(2);
  });
  it('ignores other keys and empty menus', () => {
    expect(menuStep('a', 0, 3)).toBeNull();
    expect(menuStep('ArrowDown', -1, 0)).toBeNull();
  });
});
