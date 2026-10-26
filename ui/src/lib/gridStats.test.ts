import { describe, expect, it } from 'vitest';
import { gridStats, toggleChartFilter } from './gridStats';
import type { Bucket, Plan, Priority, TaskStatus } from './types';

type T = {
  id: number; planId: number; bucketId: number; parentId: number | null; status: TaskStatus; priority: Priority;
  due: string | null; dueHasTime: boolean; completedAt: string | null;
};
let n = 0;
const mk = (o: Partial<T> = {}): T => ({
  id: ++n, planId: 1, bucketId: 10, parentId: null, status: 'todo', priority: 'medium',
  due: null, dueHasTime: false, completedAt: null, ...o,
});
const buckets: Bucket[] = [
  { id: 11, planId: 1, name: 'B', sort: 2 }, { id: 10, planId: 1, name: 'A', sort: 1 }, { id: 20, planId: 2, name: 'Z', sort: 1 },
];
const plans: Plan[] = [
  { id: 2, name: 'Two', color: '#222', sort: 2, openCount: 0 }, { id: 1, name: 'One', color: '#111', sort: 1, openCount: 0 },
];
const now = new Date(2026, 9, 20, 10, 0); // Tue 20 Oct 2026
const base = { scope: 1 as const, buckets, plans, now, weekStart: 1 as const };

describe('gridStats tiles', () => {
  it('counts top-level tasks only', () => {
    const tasks = [
      mk(), mk({ status: 'inProgress' }), mk({ status: 'done', completedAt: new Date(2026, 9, 19).toISOString() }),
      mk({ parentId: 1, status: 'inProgress' }),
    ];
    const s = gridStats(tasks, base);
    expect(s.open).toBe(2);
    expect(s.inProgress).toBe(1);
    expect(s.completed7d).toBe(1);
  });
  it('overdue uses isOverdue: date-only today is not overdue', () => {
    const s = gridStats([
      mk({ due: new Date(2026, 9, 20).toISOString() }),
      mk({ due: new Date(2026, 9, 19).toISOString() }),
      mk({ due: new Date(2026, 9, 20, 9).toISOString(), dueHasTime: true }),
    ], base);
    expect(s.overdue).toBe(2);
  });
  it('completed7d boundary is exactly 7x24h', () => {
    const edge = now.getTime() - 7 * 24 * 3600 * 1000;
    const s = gridStats([
      mk({ status: 'done', completedAt: new Date(edge).toISOString() }),
      mk({ status: 'done', completedAt: new Date(edge - 1).toISOString() }),
    ], base);
    expect(s.completed7d).toBe(1);
  });
});

describe('breakdowns', () => {
  it('status and priority in fixed order with zeros', () => {
    const s = gridStats([mk({ priority: 'urgent' }), mk({ status: 'done', priority: 'low' })], base);
    expect(s.byStatus).toEqual([{ key: 'todo', n: 1 }, { key: 'inProgress', n: 0 }, { key: 'done', n: 1 }]);
    expect(s.byPriority.map((x) => x.key)).toEqual(['urgent', 'important', 'medium', 'low']);
    expect(s.byPriority.map((x) => x.n)).toEqual([1, 0, 0, 1]);
  });
  it('byGroup follows bucket order in a plan, plan order for all', () => {
    const tasks = [mk({ bucketId: 11 }), mk({ bucketId: 11 }), mk({ planId: 2, bucketId: 20 })];
    expect(gridStats(tasks, base).byGroup).toEqual([
      { key: 10, label: 'A', n: 0 }, { key: 11, label: 'B', n: 2 },
    ]);
    expect(gridStats(tasks, { ...base, scope: 'all' }).byGroup).toEqual([
      { key: 1, label: 'One', color: '#111', n: 2 }, { key: 2, label: 'Two', color: '#222', n: 1 },
    ]);
  });
});

describe('perWeek', () => {
  const done = (y: number, m: number, d: number) => mk({ status: 'done', completedAt: new Date(y, m, d, 12).toISOString() });
  it('has 8 weeks ending with the current one (Monday start)', () => {
    const s = gridStats([done(2026, 9, 19), done(2026, 9, 20), done(2026, 9, 12)], base);
    expect(s.perWeek).toHaveLength(8);
    expect(s.perWeek[7].weekStart).toEqual(new Date(2026, 9, 19));
    expect(s.perWeek[7].n).toBe(2); // Mon 19 + Tue 20
    expect(s.perWeek[6].weekStart).toEqual(new Date(2026, 9, 12));
    expect(s.perWeek[6].n).toBe(1);
    expect(s.perWeek[0].weekStart).toEqual(new Date(2026, 7, 31));
  });
  it('Sunday start moves Sunday into the next week', () => {
    const s = gridStats([done(2026, 9, 18), done(2026, 9, 19)], { ...base, weekStart: 0 });
    expect(s.perWeek[7].weekStart).toEqual(new Date(2026, 9, 18));
    expect(s.perWeek[7].n).toBe(2);
  });
  it('is DST-safe around 29 Mar and 25 Oct', () => {
    const a = gridStats([done(2026, 2, 29), done(2026, 2, 30)], { ...base, now: new Date(2026, 3, 2, 10) });
    expect(a.perWeek.map((w) => w.weekStart.getDay())).toEqual(Array(8).fill(1));
    expect(a.perWeek.every((w) => w.weekStart.getHours() === 0)).toBe(true);
    expect(a.perWeek[6].weekStart).toEqual(new Date(2026, 2, 23));
    expect(a.perWeek[6].n).toBe(1); // Sun 29 Mar (Mon-start) belongs to week of 23 Mar
    expect(a.perWeek[7].n).toBe(1);
    const b = gridStats([done(2026, 9, 25), done(2026, 9, 26)], { ...base, now: new Date(2026, 9, 28, 10), weekStart: 0 });
    expect(b.perWeek[7].weekStart).toEqual(new Date(2026, 9, 25));
    expect(b.perWeek[7].n).toBe(2);
    expect(b.perWeek.every((w) => w.weekStart.getHours() === 0 && w.weekStart.getDay() === 0)).toBe(true);
  });
  it('crosses the 2026/2027 year boundary', () => {
    const s = gridStats([done(2026, 11, 31), done(2027, 0, 1), done(2027, 0, 4)], { ...base, now: new Date(2027, 0, 5, 10) });
    expect(s.perWeek[7].weekStart).toEqual(new Date(2027, 0, 4));
    expect(s.perWeek[7].n).toBe(1);
    expect(s.perWeek[6].weekStart).toEqual(new Date(2026, 11, 28));
    expect(s.perWeek[6].n).toBe(2);
    const t = gridStats([done(2027, 0, 3), done(2027, 0, 4)], { ...base, now: new Date(2027, 0, 5, 10), weekStart: 0 });
    expect(t.perWeek[7].weekStart).toEqual(new Date(2027, 0, 3));
    expect(t.perWeek[7].n).toBe(2);
  });
});

describe('toggleChartFilter', () => {
  it('same filter clears, different replaces', () => {
    const a = { kind: 'status', value: 'todo' } as const;
    expect(toggleChartFilter(a, { kind: 'status', value: 'todo' })).toBeNull();
    expect(toggleChartFilter(a, { kind: 'status', value: 'done' })).toEqual({ kind: 'status', value: 'done' });
    expect(toggleChartFilter(null, a)).toEqual(a);
    expect(toggleChartFilter(a, { kind: 'priority', value: 'low' })).toEqual({ kind: 'priority', value: 'low' });
  });
});
