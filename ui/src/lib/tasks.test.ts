import { describe, expect, it } from 'vitest';
import { filterTasks, groupTasks, isOverdue, taskCounts } from './tasks';

type T = { id: number; due: string | null; dueHasTime: boolean; completedAt: string | null };
const now = new Date(2026, 9, 20, 10, 0); // Tue 20 Oct 2026, 10:00 local
const at = (day: number, h = 0, m = 0) => new Date(2026, 9, day, h, m).toISOString();
const task = (id: number, due: string | null, dueHasTime = false, completedAt: string | null = null): T =>
  ({ id, due, dueHasTime, completedAt });

const tasks: T[] = [
  task(1, at(18)),                         // date-only, two days ago → overdue
  task(2, at(20, 9), true),                // today 09:00, already past → overdue
  task(3, at(20)),                         // date-only today → today (not overdue)
  task(4, at(20, 18), true),               // today 18:00 → today
  task(5, at(21)),                         // tomorrow
  task(6, at(25, 8), true),                // later date
  task(7, null),                           // no date
  task(8, at(19), false, at(19, 12, 0)),   // completed
  task(9, null, false, at(20, 8, 0)),      // completed more recently
];

const ids = (xs: T[]) => xs.map((t) => t.id);

describe('isOverdue', () => {
  it('timed tasks are overdue once their time passes; date-only ones from the next day', () => {
    expect(isOverdue(tasks[0], now)).toBe(true);
    expect(isOverdue(tasks[1], now)).toBe(true);
    expect(isOverdue(tasks[2], now)).toBe(false);
    expect(isOverdue(tasks[3], now)).toBe(false);
    expect(isOverdue(tasks[6], now)).toBe(false);
  });
});

describe('filterTasks', () => {
  it('all = every open task', () => expect(ids(filterTasks(tasks, 'all', now)).sort()).toEqual([1, 2, 3, 4, 5, 6, 7]));
  it('today = overdue plus due today', () => expect(ids(filterTasks(tasks, 'today', now)).sort()).toEqual([1, 2, 3, 4]));
  it('upcoming = due after today', () => expect(ids(filterTasks(tasks, 'upcoming', now)).sort()).toEqual([5, 6]));
  it('nodate = open without a due date', () => expect(ids(filterTasks(tasks, 'nodate', now))).toEqual([7]));
  it('completed = most recently completed first', () => expect(ids(filterTasks(tasks, 'completed', now))).toEqual([9, 8]));
});

describe('groupTasks', () => {
  it('groups open tasks: overdue, then each day, then no date', () => {
    const g = groupTasks(tasks, 'all', now);
    expect(g.map((x) => x.kind)).toEqual(['overdue', 'day', 'day', 'day', 'nodate']);
    expect(g.map((x) => ids(x.tasks))).toEqual([[1, 2], [3, 4], [5], [6], [7]]);
    expect(g[1].day?.getDate()).toBe(20);
    expect(g[3].day?.getDate()).toBe(25);
    expect(g[0].day).toBeNull();
  });

  it('completed is a single group', () => {
    const g = groupTasks(tasks, 'completed', now);
    expect(g).toHaveLength(1);
    expect(g[0].kind).toBe('completed');
    expect(ids(g[0].tasks)).toEqual([9, 8]);
  });

  it('no groups when nothing matches', () => expect(groupTasks([], 'all', now)).toEqual([]));
});

describe('taskCounts', () => {
  it('counts each filter', () =>
    expect(taskCounts(tasks, now)).toEqual({ all: 7, today: 4, upcoming: 2, nodate: 1, completed: 2 }));
});

import { applyFilters, boardColumns, nextStatus } from './tasks';

type S = 'todo' | 'inProgress' | 'done';
type B = T & { status: S; labels: { id: number }[] };
const b = (id: number, status: S, labels: number[], due: string | null = null, completedAt: string | null = null): B =>
  ({ id, due, dueHasTime: false, completedAt: status === 'done' ? completedAt ?? at(1) : null, status, labels: labels.map((l) => ({ id: l })) });

describe('applyFilters', () => {
  const xs = [b(1, 'todo', [1]), b(2, 'inProgress', [2]), b(3, 'inProgress', [1, 3]), b(4, 'todo', [])];
  it('status only', () => expect(ids(applyFilters(xs, { status: 'inProgress', labels: [] }))).toEqual([2, 3]));
  it('labels any-of', () => expect(ids(applyFilters(xs, { status: 'any', labels: [1, 2] }))).toEqual([1, 2, 3]));
  it('both', () => expect(ids(applyFilters(xs, { status: 'inProgress', labels: [1] }))).toEqual([3]));
  it('no filters keeps all', () => expect(ids(applyFilters(xs, { status: 'any', labels: [] }))).toEqual([1, 2, 3, 4]));
});

describe('boardColumns', () => {
  it('three columns in order, open ones sorted by due (none last) then id', () => {
    const cols = boardColumns([b(5, 'todo', [], null), b(2, 'todo', [], at(22)), b(1, 'todo', [], at(21)), b(3, 'inProgress', [])], { showAllDone: false });
    expect(cols.map((c) => c.status)).toEqual(['todo', 'inProgress', 'done']);
    expect(ids(cols[0].tasks)).toEqual([1, 2, 5]);
    expect(ids(cols[1].tasks)).toEqual([3]);
    expect(cols[2].tasks).toEqual([]);
  });

  it('done shows the 30 most recent unless showAllDone', () => {
    const done = Array.from({ length: 35 }, (_, i) => b(i + 1, 'done', [], null, new Date(2026, 9, 1, i).toISOString()));
    const cut = boardColumns(done, { showAllDone: false })[2];
    expect(cut.tasks).toHaveLength(30);
    expect(cut.tasks[0].id).toBe(35);
    expect(cut.hidden).toBe(5);
    const full = boardColumns(done, { showAllDone: true })[2];
    expect(full.tasks).toHaveLength(35);
    expect(full.hidden).toBe(0);
  });
});

describe('nextStatus', () => {
  it('cycles', () => {
    expect(nextStatus('todo')).toBe('inProgress');
    expect(nextStatus('inProgress')).toBe('done');
    expect(nextStatus('done')).toBe('todo');
  });
});

import { sidebarSections, taskPageGroups } from './tasks';

describe('sidebarSections (same rules as the Tasks page)', () => {
  type W = T & { status: S; labels: { id: number }[] };
  const w = (id: number, due: string | null, dueHasTime = false, status: S = 'todo'): W =>
    ({ id, due, dueHasTime, completedAt: status === 'done' ? at(20, 8) : null, status, labels: [] });
  const xs = [w(1, at(18)), w(2, at(20, 18), true), w(3, at(22)), w(4, null), w(5, at(19), false, 'done'), w(6, at(20, 9), true)];
  it('splits into overdue, today, upcoming, no date and completed', () => {
    const s = sidebarSections(xs, now);
    expect(s.map((x) => [x.key, ids(x.tasks)])).toEqual([
      ['overdue', [1, 6]],
      ['today', [2]],
      ['upcoming', [3]],
      ['nodate', [4]],
      ['completed', [5]],
    ]);
  });
});

describe('taskPageGroups (date tabs + status + labels)', () => {
  type W = T & { status: S; labels: { id: number }[] };
  const w = (id: number, due: string | null, status: S = 'todo', lbl: number[] = []): W =>
    ({ id, due, dueHasTime: false, completedAt: status === 'done' ? at(19, 8) : null, status, labels: lbl.map((x) => ({ id: x })) });
  const xs = [w(1, at(20), 'todo', [1]), w(2, at(22), 'inProgress', [2]), w(3, null, 'todo', [1]), w(4, at(19), 'done', [1])];
  const flat = (g: { tasks: W[] }[]) => g.flatMap((x) => ids(x.tasks));
  it('open tasks by date tab, filtered by status and labels', () => {
    expect(flat(taskPageGroups(xs, { view: 'all', status: 'any', labels: [] }, now))).toEqual([1, 2, 3]);
    expect(flat(taskPageGroups(xs, { view: 'all', status: 'inProgress', labels: [] }, now))).toEqual([2]);
    expect(flat(taskPageGroups(xs, { view: 'nodate', status: 'any', labels: [1] }, now))).toEqual([3]);
  });
  it('status Done lists finished tasks (any date), still label-filtered', () => {
    const g = taskPageGroups(xs, { view: 'today', status: 'done', labels: [1] }, now);
    expect(g.map((x) => x.kind)).toEqual(['completed']);
    expect(flat(g)).toEqual([4]);
    expect(taskPageGroups(xs, { view: 'all', status: 'done', labels: [2] }, now)).toEqual([]);
  });
});

describe('start dates', () => {
  type S = T & { start: string | null };
  const xs: S[] = [
    { ...task(1, at(25)), start: at(21) },   // not started yet
    { ...task(2, at(25)), start: at(20) },   // started today but due later
    { ...task(3, at(20)), start: at(18) },   // due today
  ];
  it('today view excludes tasks whose start is after today', () => {
    expect(ids(filterTasks(xs, 'today', now))).toEqual([3]);
    expect(ids(sidebarSections(xs, now).find((s) => s.key === 'today')!.tasks)).toEqual([3]);
  });
});
