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
