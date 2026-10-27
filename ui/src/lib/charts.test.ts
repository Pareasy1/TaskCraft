import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { barLengths, fmtCount, loadChartsOpen, saveChartsOpen, shownTicks } from './charts';
import { dateLocale } from './i18n';

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

describe('shownTicks', () => {
  it('shows every label when there is room', () => {
    expect(shownTicks(8, 400)).toEqual([0, 1, 2, 3, 4, 5, 6, 7]);
  });
  it('thins labels on a narrow chart, always keeping the last (current) week', () => {
    // 8 bands in 220px are 27.5px each; a label needs 36px, so every 2nd.
    expect(shownTicks(8, 220)).toEqual([1, 3, 5, 7]);
  });
  it('keeps the first label when it does not crowd its neighbour', () => {
    // every 3rd from the end: 7, 4, 1; 0 would sit next to 1, so it is dropped
    expect(shownTicks(8, 120)).toEqual([1, 4, 7]);
    // 7 bands, every 2nd from the end: 6, 4, 2, 0, so the first is kept
    expect(shownTicks(7, 150)).toEqual([0, 2, 4, 6]);
  });
  it('shows nothing before the width is known, and nothing for no weeks', () => {
    expect(shownTicks(8, 0)).toEqual([]);
    expect(shownTicks(0, 300)).toEqual([]);
  });
});

describe('fmtCount', () => {
  it('formats a count with grouping in the app locale', () => {
    expect(fmtCount(0)).toBe('0');
    expect(fmtCount(1284)).toBe(new Intl.NumberFormat(dateLocale).format(1284));
  });
});
