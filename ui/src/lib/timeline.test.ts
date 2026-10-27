import { describe, it, expect, afterEach, vi } from 'vitest';
import {
  DAY_WIDTH, barSpan, dayDate, dayNumber, dayToX, dragResult, headerTicks,
  loadUndatedOpen, loadZoom, placeUndated, saveUndatedOpen, saveZoom, timelineRange, xToDay,
} from './timeline';

const local = (y: number, m: number, d: number, hh = 0, mm = 0) => new Date(y, m - 1, d, hh, mm);
const iso = (y: number, m: number, d: number, hh = 0, mm = 0) => local(y, m, d, hh, mm).toISOString();

afterEach(() => vi.unstubAllGlobals());

describe('day numbers', () => {
  it('round-trips across DST changes', () => {
    for (const [y, m, d] of [[2026, 3, 28], [2026, 3, 29], [2026, 3, 30], [2026, 10, 24], [2026, 10, 25], [2026, 10, 26]]) {
      const n = dayNumber(local(y, m, d, 15));
      const back = dayDate(n);
      expect([back.getFullYear(), back.getMonth() + 1, back.getDate(), back.getHours()]).toEqual([y, m, d, 0]);
      expect(dayNumber(back)).toBe(n);
    }
    expect(dayNumber(local(2026, 3, 30)) - dayNumber(local(2026, 3, 29))).toBe(1);
    expect(dayNumber(local(2026, 10, 26)) - dayNumber(local(2026, 10, 25))).toBe(1);
    expect(dayNumber(local(1970, 1, 1, 12))).toBe(0);
  });
});

describe('barSpan', () => {
  const a = iso(2026, 10, 20), b = iso(2026, 10, 24, 18);
  it('uses both dates inclusively', () => {
    expect(barSpan({ start: a, due: b })).toEqual({ from: dayNumber(local(2026, 10, 20)), to: dayNumber(local(2026, 10, 24)) });
  });
  it('one date gives a 1-day span', () => {
    const d = dayNumber(local(2026, 10, 24));
    expect(barSpan({ start: null, due: b })).toEqual({ from: d, to: d });
    const s = dayNumber(local(2026, 10, 20));
    expect(barSpan({ start: a, due: null })).toEqual({ from: s, to: s });
  });
  it('none gives null', () => expect(barSpan({ start: null, due: null })).toBeNull());
});

describe('timelineRange', () => {
  const today = local(2026, 10, 27, 10);
  const t = dayNumber(today);
  it('pads around tasks and today, min 92 days', () => {
    const r = timelineRange([{ start: iso(2026, 10, 20), due: iso(2026, 10, 30) }], today);
    expect(r.from).toBe(dayNumber(local(2026, 10, 20)) - 14);
    expect(r.to - r.from + 1).toBeGreaterThanOrEqual(92);
    expect(r.to).toBeGreaterThanOrEqual(dayNumber(local(2026, 10, 30)) + 28);
  });
  it('includes today when tasks are far away', () => {
    const r = timelineRange([{ start: iso(2025, 1, 1), due: iso(2025, 1, 5) }], today);
    expect(r.to).toBe(t + 28);
    const r2 = timelineRange([{ start: iso(2027, 6, 1), due: iso(2027, 6, 5) }], today);
    expect(r2.from).toBe(t - 14);
  });
  it('handles no tasks', () => {
    const r = timelineRange([], today);
    expect(r.from).toBe(t - 14);
    expect(r.to - r.from + 1).toBe(92);
  });
});

describe('x mapping', () => {
  it('round-trips at each zoom', () => {
    const range = { from: 20000, to: 20100 };
    for (const z of ['days', 'weeks', 'months'] as const) {
      for (const day of [20000, 20001, 20050, 20100]) {
        const x = dayToX(day, range, z);
        expect(x).toBe((day - 20000) * DAY_WIDTH[z]);
        expect(xToDay(x, range, z)).toBe(day);
        expect(xToDay(x + DAY_WIDTH[z] - 1, range, z)).toBe(day);
      }
    }
  });
});

describe('dragResult', () => {
  const both = { start: iso(2026, 10, 20, 9), startHasTime: true, due: iso(2026, 10, 24, 18), dueHasTime: true };
  it('move +3 keeps time of day', () => {
    const r = dragResult(both, 'move', 3);
    expect(r.start).toBe(iso(2026, 10, 23, 9));
    expect(r.due).toBe(iso(2026, 10, 27, 18));
    expect(r.startHasTime && r.dueHasTime).toBe(true);
  });
  it('move across the Oct 25 DST change keeps 09:00', () => {
    const r = dragResult({ ...both, start: iso(2026, 10, 24, 9) }, 'move', 2);
    const s = new Date(r.start!);
    expect([s.getDate(), s.getHours(), s.getMinutes()]).toEqual([26, 9, 0]);
  });
  it('move across Mar 29 DST change keeps time', () => {
    const r = dragResult({ ...both, start: iso(2026, 3, 28, 9), due: iso(2026, 3, 28, 18) }, 'move', 2);
    expect(new Date(r.start!).getHours()).toBe(9);
    expect(new Date(r.due!).getHours()).toBe(18);
  });
  it('move with only a due shifts it', () => {
    const r = dragResult({ start: null, startHasTime: false, due: iso(2026, 10, 24), dueHasTime: false }, 'move', -1);
    expect(r).toEqual({ start: null, startHasTime: false, due: iso(2026, 10, 23), dueHasTime: false });
  });
  it('start on due-only creates an all-day start', () => {
    const r = dragResult({ start: null, startHasTime: false, due: iso(2026, 10, 24, 18), dueHasTime: true }, 'start', -3);
    expect(r.start).toBe(iso(2026, 10, 21));
    expect(r.startHasTime).toBe(false);
    expect(r.due).toBe(iso(2026, 10, 24, 18));
    expect(r.dueHasTime).toBe(true);
  });
  it('start beyond due clamps to the due day', () => {
    const r = dragResult(both, 'start', 10);
    const s = new Date(r.start!);
    expect([s.getDate(), s.getHours()]).toEqual([24, 9]);
  });
  it('due before start clamps to the start day', () => {
    const r = dragResult(both, 'due', -10);
    const d = new Date(r.due!);
    expect([d.getDate(), d.getHours()]).toEqual([20, 18]);
  });
  it('due on start-only creates an all-day due', () => {
    const r = dragResult({ start: iso(2026, 10, 20), startHasTime: false, due: null, dueHasTime: false }, 'due', 4);
    expect(r.due).toBe(iso(2026, 10, 24));
    expect(r.dueHasTime).toBe(false);
  });
  it('due on start-only clamps negative delta to start day', () => {
    const r = dragResult({ start: iso(2026, 10, 20), startHasTime: false, due: null, dueHasTime: false }, 'due', -4);
    expect(r.due).toBe(iso(2026, 10, 20));
  });
});

describe('placeUndated', () => {
  it('gives local midnight for both', () => {
    const n = dayNumber(local(2026, 10, 25));
    expect(placeUndated(n)).toEqual({ start: iso(2026, 10, 25), startHasTime: false, due: iso(2026, 10, 25), dueHasTime: false });
  });
});

describe('headerTicks', () => {
  const range = { from: dayNumber(local(2026, 10, 1)), to: dayNumber(local(2027, 1, 31)) };
  it('days: every day, major on month start', () => {
    const t = headerTicks(range, 'days', 1);
    expect(t).toHaveLength(range.to - range.from + 1);
    const majors = t.filter((x) => x.major).map((x) => dayDate(x.day).getDate());
    expect(majors.every((d) => d === 1)).toBe(true);
    expect(majors).toHaveLength(4);
  });
  it('weeks honour weekStart', () => {
    for (const ws of [0, 1] as const) {
      const t = headerTicks(range, 'weeks', ws);
      expect(t.length).toBeGreaterThan(10);
      expect(t.every((x) => dayDate(x.day).getDay() === ws)).toBe(true);
      expect(t.every((x) => x.day >= range.from && x.day <= range.to)).toBe(true);
    }
  });
  it('months: each month start', () => {
    const t = headerTicks(range, 'months', 1);
    expect(t.map((x) => dayDate(x.day).getMonth())).toEqual([9, 10, 11, 0]);
    expect(t.every((x) => dayDate(x.day).getDate() === 1)).toBe(true);
  });
});

describe('persistence', () => {
  it('falls back to defaults when storage throws', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => { throw new Error('no'); },
      setItem: () => { throw new Error('no'); },
    });
    expect(loadZoom()).toBe('weeks');
    expect(loadUndatedOpen()).toBe(true);
    expect(() => { saveZoom('days'); saveUndatedOpen(false); }).not.toThrow();
  });
  it('round-trips through storage', () => {
    const store: Record<string, string> = {};
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => store[k] ?? null,
      setItem: (k: string, v: string) => { store[k] = v; },
    });
    saveZoom('months'); saveUndatedOpen(false);
    expect(loadZoom()).toBe('months');
    expect(loadUndatedOpen()).toBe(false);
  });
});
