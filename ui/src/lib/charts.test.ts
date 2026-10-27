import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { barLengths, loadChartsOpen, saveChartsOpen } from './charts';

describe('barLengths', () => {
  it('scales to the largest value', () => {
    expect(barLengths([0, 5, 10], 200)).toEqual([0, 100, 200]);
  });
  it('is all zero when every value is zero (no division by zero)', () => {
    expect(barLengths([0, 0, 0], 200)).toEqual([0, 0, 0]);
  });
  it('handles an empty list', () => expect(barLengths([], 200)).toEqual([]));
  it('gives a non-zero value at least the minimum visible length', () => {
    expect(barLengths([1, 1000], 200)).toEqual([4, 200]);
  });
  it('never exceeds the available length, even when tiny', () => {
    expect(barLengths([1, 2], 3)).toEqual([3, 3]);
    expect(barLengths([1], 0)).toEqual([0]);
  });
});

describe('charts panel storage', () => {
  let m: Map<string, string>;
  beforeEach(() => {
    m = new Map();
    vi.stubGlobal('localStorage', { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) });
  });
  afterEach(() => vi.unstubAllGlobals());

  it('is open by default', () => expect(loadChartsOpen()).toBe(true));
  it('remembers collapsed and open', () => {
    saveChartsOpen(false);
    expect(m.get('taskcraft.grid.charts')).toBe('collapsed');
    expect(loadChartsOpen()).toBe(false);
    saveChartsOpen(true);
    expect(loadChartsOpen()).toBe(true);
  });
  it('ignores junk', () => {
    m.set('taskcraft.grid.charts', 'nope');
    expect(loadChartsOpen()).toBe(true);
  });
  it('survives storage being unavailable', () => {
    vi.stubGlobal('localStorage', { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } });
    expect(loadChartsOpen()).toBe(true);
    expect(() => saveChartsOpen(false)).not.toThrow();
  });
});
