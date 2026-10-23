import { describe, expect, it } from 'vitest';
import { movedDue, movedSpan, shiftStart, spanRole, tasksForDay, TASK_SLOT_MIN } from './calendarTasks';

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

describe('spanRole', () => {
  const span = { id: 1, due: at(22), dueHasTime: false, completedAt: null, start: at(20) }; // Tue 20 -> Thu 22
  it('start, middle, end, outside', () => {
    expect(spanRole(span, day(20))).toBe('start');
    expect(spanRole(span, day(21))).toBe('middle');
    expect(spanRole(span, day(22))).toBe('end');
    expect(spanRole(span, day(23))).toBeNull();
    expect(spanRole(span, day(19))).toBeNull();
  });
  it('null without a start or when start and due are on the same day', () => {
    expect(spanRole({ ...span, start: null }, day(21))).toBeNull();
    expect(spanRole({ ...span, due: at(20, 18), dueHasTime: true, start: at(20, 9) }, day(20))).toBeNull();
  });
  it('a timed span across midnight is start then end', () => {
    const s = { ...span, start: at(20, 22), due: at(21, 2), dueHasTime: true };
    expect(spanRole(s, day(20))).toBe('start');
    expect(spanRole(s, day(21))).toBe('end');
  });
});

describe('tasksForDay with spans', () => {
  const s = { id: 1, due: at(23), dueHasTime: false, completedAt: null, start: at(21) };
  it('lists a spanning task once per covered day in the all-day strip with its role', () => {
    const roles = [20, 21, 22, 23, 24].map((d) => tasksForDay([s], day(d), now).allDay.map((x) => x.span));
    expect(roles).toEqual([[], ['start'], ['middle'], ['end'], []]);
    expect(tasksForDay([s], day(22), now).timed).toEqual([]);
  });
  it('non-spanning tasks have span null', () => {
    expect(tasksForDay([task(2, at(20))], day(20), now).allDay[0].span).toBeNull();
  });
});

describe('shiftStart', () => {
  it('moves start by the same delta as due', () => {
    const t = { id: 1, due: at(22, 12), dueHasTime: true, completedAt: null, start: at(20, 12) };
    expect(shiftStart(t, at(23, 12))).toBe(at(21, 12));
    expect(shiftStart({ ...t, start: null }, at(23, 12))).toBeNull();
  });
});

describe('shiftStart across DST (Europe/Rome)', () => {
  const L = (m: number, d: number, h = 0, mi = 0) => new Date(2026, m, d, h, mi).toISOString();
  const span = (start: string, due: string, dueHasTime: boolean) => ({ id: 1, due, dueHasTime, completedAt: null, start });
  it('date-only span over spring-forward (29 Mar) keeps local midnight', () => {
    const t = span(L(2, 28), L(2, 29), false);
    expect(shiftStart(t, L(2, 30))).toBe(L(2, 29));
    expect(shiftStart(t, L(2, 28))).toBe(L(2, 27));
  });
  it('date-only span over fall-back (25 Oct) keeps local midnight', () => {
    const t = span(L(9, 24), L(9, 25), false);
    expect(shiftStart(t, L(9, 26))).toBe(L(9, 25));
    expect(shiftStart(t, L(9, 24))).toBe(L(9, 23));
  });
  it('timed span keeps wall-clock time across spring-forward and fall-back', () => {
    const a = span(L(2, 28, 9), L(2, 29, 18), true);
    expect(shiftStart(a, L(2, 30, 18))).toBe(L(2, 29, 9));
    const b = span(L(9, 24, 9), L(9, 25, 18), true);
    expect(shiftStart(b, L(9, 26, 18))).toBe(L(9, 25, 9));
  });
  it('a 15-minute timed move shifts start by 15 minutes of wall-clock time', () => {
    const t = span(L(9, 24, 9), L(9, 25, 18), true);
    expect(shiftStart(t, L(9, 25, 18, 15))).toBe(L(9, 24, 9, 15));
  });
});

describe('movedSpan', () => {
  const span = { id: 1, due: at(22, 12), dueHasTime: true, completedAt: null, start: at(20, 12) };
  it('movedDue on a span also shifts start', () => {
    expect(movedSpan(span, 'ArrowRight')).toEqual({ due: at(23, 12), start: at(21, 12) });
    expect(movedSpan(span, 'ArrowDown')).toEqual({ due: at(22, 12, 15), start: at(20, 12, 15) });
  });
  it('leaves start null without one, and ignores other keys', () => {
    expect(movedSpan({ ...span, start: null }, 'ArrowLeft')).toEqual({ due: at(21, 12), start: null });
    expect(movedSpan(span, 'a')).toBeNull();
  });
});
