import { describe, expect, it } from 'vitest';
import { movedDue, tasksForDay, TASK_SLOT_MIN } from './calendarTasks';

type T = { id: number; due: string | null; dueHasTime: boolean; completedAt: string | null };
const at = (d: number, h = 0, m = 0) => new Date(2026, 9, d, h, m).toISOString();
const task = (id: number, due: string | null, dueHasTime = false, done = false): T =>
  ({ id, due, dueHasTime, completedAt: done ? at(20, 8) : null });
const now = new Date(2026, 9, 20, 10, 0); // Tue 20 Oct, 10:00
const day = (d: number) => new Date(2026, 9, d);
const ids = (xs: { task: T }[]) => xs.map((x) => x.task.id);

describe('tasksForDay', () => {
  const tasks = [
    task(1, at(20, 18), true),        // today 18:00
    task(2, at(20)),                  // today, date only
    task(3, at(20, 9), true, true),   // done today 09:00
    task(4, at(18)),                  // overdue, date only
    task(5, at(19, 16), true),        // overdue, timed yesterday
    task(6, at(17), false, true),     // done in the past: not overdue
    task(7, null),                    // no date: never on the calendar
    task(8, at(21, 9), true),         // tomorrow
  ];

  it('timed tasks go on the grid, date-only ones in the all-day strip, done ones included', () => {
    const r = tasksForDay(tasks, day(20), now);
    expect(ids(r.timed)).toEqual([3, 1]);
    expect(ids(r.allDay)).toEqual([2, 4, 5]);
  });

  it('today also lists open overdue tasks, marked overdue, in the all-day strip', () => {
    const r = tasksForDay(tasks, day(20), now);
    expect(r.allDay.map((x) => [x.task.id, x.overdue])).toEqual([[2, false], [4, true], [5, true]]);
  });

  it('overdue tasks still show on their own day, flagged overdue', () => {
    const r = tasksForDay(tasks, day(19), now);
    expect(r.timed.map((x) => [x.task.id, x.overdue])).toEqual([[5, true]]);
    expect(tasksForDay(tasks, day(18), now).allDay.map((x) => [x.task.id, x.overdue])).toEqual([[4, true]]);
  });

  it('other days show only what is due on them', () => {
    expect(ids(tasksForDay(tasks, day(21), now).timed)).toEqual([8]);
    expect(ids(tasksForDay(tasks, day(17), now).allDay)).toEqual([6]);
    expect(tasksForDay(tasks, day(22), now)).toEqual({ timed: [], allDay: [] });
  });

  // A row needs ~20px for its mark and title: 25 min at 48px per hour, so it never covers the next item.
  it('a task reserves a 25-minute slot', () => expect(TASK_SLOT_MIN).toBe(25));
});

describe('movedDue', () => {
  it('↑/↓ move a timed task by 15 minutes', () => {
    const t = task(1, at(20, 18), true);
    expect(movedDue(t, 'ArrowUp')).toBe(at(20, 17, 45));
    expect(movedDue(t, 'ArrowDown')).toBe(at(20, 18, 15));
  });

  it('←/→ move a day, keeping the time, across the DST change', () => {
    expect(movedDue(task(1, at(24, 18), true), 'ArrowRight')).toBe(at(25, 18));
    expect(movedDue(task(2, at(20)), 'ArrowLeft')).toBe(at(19));
  });

  it('↑/↓ do nothing for a date-only task; other keys do nothing', () => {
    expect(movedDue(task(2, at(20)), 'ArrowUp')).toBeNull();
    expect(movedDue(task(1, at(20, 18), true), 'a')).toBeNull();
    expect(movedDue(task(7, null), 'ArrowRight')).toBeNull();
  });
});
