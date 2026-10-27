import { describe, it, expect, afterEach, vi } from 'vitest';
import {
  DAY_WIDTH, ROW_H, barSpan, dayDate, dayNumber, dayToX, dragResult, headerTicks,
  loadUndatedOpen, loadZoom, placeUndated, saveUndatedOpen, saveZoom, timelineRange, timelineRows, xToDay,
  type TimelineRow,
} from './timeline';
import type { Bucket, Plan, TaskStatus } from './types';

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
  const ms = (s: string | null) => new Date(s!).getTime();
  it('start beyond due takes the due instant', () => {
    const r = dragResult({ ...both, start: iso(2026, 10, 20, 18), due: iso(2026, 10, 24, 9) }, 'start', 10);
    expect(r.start).toBe(iso(2026, 10, 24, 9));
    expect(r.startHasTime).toBe(true);
    expect(ms(r.start)).toBeLessThanOrEqual(ms(r.due));
  });
  it('due before start takes the start instant', () => {
    const r = dragResult({ ...both, start: iso(2026, 10, 20, 18), due: iso(2026, 10, 24, 9) }, 'due', -10);
    expect(r.due).toBe(iso(2026, 10, 20, 18));
    expect(ms(r.start)).toBeLessThanOrEqual(ms(r.due));
  });
  it('due on timed start-only with delta <= 0 takes the start instant and flag', () => {
    const t0 = { start: iso(2026, 10, 20, 18), startHasTime: true, due: null, dueHasTime: false };
    for (const d of [0, -3]) {
      const r = dragResult(t0, 'due', d);
      expect(r.due).toBe(t0.start);
      expect(r.dueHasTime).toBe(true);
    }
  });
  it('timed start dragged onto an all-day due day takes the due instant', () => {
    const r = dragResult({ start: iso(2026, 10, 22, 9), startHasTime: true, due: iso(2026, 10, 24), dueHasTime: false }, 'start', 2);
    expect(r.start).toBe(iso(2026, 10, 24));
    expect(r.startHasTime).toBe(false);
    expect(ms(r.start)).toBeLessThanOrEqual(ms(r.due));
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
    expect(store['taskcraft.planner.zoom']).toBe('months');
    expect(store['taskcraft.planner.undated']).toBe('false');
    expect(loadZoom()).toBe('months');
    expect(loadUndatedOpen()).toBe(false);
  });
});

describe('timelineRows', () => {
  const plans: Plan[] = [
    { id: 1, name: 'Alpha', color: '#000', sort: 2, openCount: 0 },
    { id: 2, name: 'Beta', color: '#000', sort: 1, openCount: 0 },
  ];
  const buckets: Bucket[] = [
    { id: 10, planId: 1, name: 'Later', sort: 2 },
    { id: 11, planId: 1, name: 'Now', sort: 1 },
    { id: 20, planId: 2, name: 'Only', sort: 1 },
  ];
  let n = 0;
  type T = ReturnType<typeof mk>;
  const mk = (o: Partial<{ id: number; status: TaskStatus; planId: number; bucketId: number; sortOrder: number; parentId: number | null; start: string | null; due: string | null; labels: { id: number; name: string; color: string }[] }> = {}) => ({
    id: ++n, title: 't', status: 'todo' as TaskStatus, planId: 1, bucketId: 10, sortOrder: 0, parentId: null as number | null,
    start: null as string | null, due: null as string | null, dueHasTime: false, completedAt: null as string | null,
    labels: [] as { id: number; name: string; color: string }[], ...o,
  });
  const base = { scope: 1 as number | 'all', buckets, plans, labels: [] as number[], status: 'any' as const, collapsed: new Set<string | number>() };
  const shape = (rows: TimelineRow<T>[]) => rows.map((r) => (r.kind === 'group' ? r.key : r.task.id));

  it('row height is 32', () => expect(ROW_H).toBe(32));

  it('groups by bucket in sort order, tasks in default order', () => {
    const a = mk({ bucketId: 10, sortOrder: 1 }), b = mk({ bucketId: 11, sortOrder: 5 }), c = mk({ bucketId: 10, sortOrder: 0 });
    const rows = timelineRows([a, b, c], base);
    expect(shape(rows)).toEqual(['b:11', b.id, 'b:10', c.id, a.id]);
    expect(rows[0]).toEqual({ kind: 'group', key: 'b:11', label: 'Now', count: 1, collapsed: false });
    expect(rows[2]).toMatchObject({ kind: 'group', label: 'Later', count: 2 });
  });

  it('groups by plan in All tasks', () => {
    const a = mk({ planId: 1, bucketId: 11 }), b = mk({ planId: 2, bucketId: 20 }), c = mk({ planId: 1, bucketId: 10 });
    const rows = timelineRows([a, b, c], { ...base, scope: 'all' });
    expect(shape(rows)).toEqual(['p:2', b.id, 'p:1', a.id, c.id]);
    expect(rows[0]).toMatchObject({ label: 'Beta' });
  });

  it('only the scope\'s tasks inside a plan', () => {
    const a = mk({ planId: 1 }), b = mk({ planId: 2, bucketId: 20 });
    expect(shape(timelineRows([a, b], base))).toEqual(['b:10', a.id]);
  });

  it('a collapsed group hides its tasks but keeps its header', () => {
    const a = mk({ bucketId: 11 }), b = mk({ bucketId: 10 });
    const rows = timelineRows([a, b], { ...base, collapsed: new Set<string | number>(['b:11']) });
    expect(shape(rows)).toEqual(['b:11', 'b:10', b.id]);
    expect(rows[0]).toMatchObject({ collapsed: true, count: 1 });
  });

  it('subtasks indent under their parent; a collapsed parent hides them', () => {
    const p = mk({ sortOrder: 0 }), c1 = mk({ parentId: 0, sortOrder: 1 }), c2 = mk({ parentId: 0, sortOrder: 0 });
    c1.parentId = p.id; c2.parentId = p.id;
    const rows = timelineRows([p, c1, c2], base);
    expect(shape(rows)).toEqual(['b:10', p.id, c2.id, c1.id]);
    expect(rows[1]).toMatchObject({ kind: 'task', depth: 0, hasChildren: true, collapsed: false, dimmed: false });
    expect(rows[2]).toMatchObject({ kind: 'task', depth: 1, hasChildren: false });
    expect(shape(timelineRows([p, c1, c2], { ...base, collapsed: new Set<string | number>([p.id]) }))).toEqual(['b:10', p.id]);
  });

  it('a matching subtask under a non-matching parent shows a dimmed parent', () => {
    const p = mk({ status: 'done' }), c = mk({ status: 'todo' });
    c.parentId = p.id;
    const rows = timelineRows([p, c], { ...base, status: 'todo' });
    expect(shape(rows)).toEqual(['b:10', p.id, c.id]);
    expect(rows[1]).toMatchObject({ dimmed: true });
    // A dimmed parent never hides its matching subtasks.
    expect(shape(timelineRows([p, c], { ...base, status: 'todo', collapsed: new Set<string | number>([p.id]) }))).toEqual(['b:10', p.id, c.id]);
  });

  it('filters by label', () => {
    const a = mk({ labels: [{ id: 5, name: 'x', color: '#000' }] }), b = mk();
    expect(shape(timelineRows([a, b], { ...base, labels: [5] }))).toEqual(['b:10', a.id]);
  });

  it('undated tasks get rows too', () => {
    const a = mk({ due: iso(2026, 10, 30) }), b = mk({ sortOrder: 1 });
    expect(shape(timelineRows([a, b], base))).toEqual(['b:10', a.id, b.id]);
  });

  it('no tasks, no rows', () => expect(timelineRows([], base)).toEqual([]));
});
