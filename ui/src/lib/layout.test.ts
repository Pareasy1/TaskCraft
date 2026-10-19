import { describe, expect, it } from 'vitest';
import { monthGrid } from './layout';

const ymd = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

describe('monthGrid', () => {
  it('October 2026, Monday start', () => {
    const g = monthGrid(2026, 9, 1);
    expect(g.length).toBe(42);
    expect(ymd(g[0])).toBe('2026-09-28');
    expect(ymd(g[41])).toBe('2026-11-08');
  });

  it('Sunday start begins on the preceding Sunday', () => {
    const g = monthGrid(2026, 9, 0);
    expect(ymd(g[0])).toBe('2026-09-27');
  });

  it('survives the DST change inside the grid', () => {
    const g = monthGrid(2026, 9, 1);
    // Oct 25 2026 is the DST end in Europe; every cell must be a distinct day.
    expect(new Set(g.map(ymd)).size).toBe(42);
  });
});
