// Which tasks the Day/Week/Month views show, and how a keyboard move changes a due date.
import { addDays, sameDay, startOfDay } from './layout';
import { isOverdue } from './tasks';

type TaskLike = { id: number; due: string | null; dueHasTime: boolean; completedAt: string | null; start?: string | null };

export type SpanRole = 'start' | 'middle' | 'end';

/**
 * A timed task occupies this many minutes in the day's overlap layout (tasks
 * have no duration): enough for a one-line row (~20px at 48px per hour), so it
 * never covers the next item's title.
 */
export const TASK_SLOT_MIN = 25;
/** Keyboard moves shift a timed task by this many minutes. */
const MOVE_STEP_MIN = 15;

export interface DayTask<T> {
  task: T;
  overdue: boolean;
  /** Where this day sits in a start-to-due span; null for a single-day task. */
  span: SpanRole | null;
}

const byDue = <T extends TaskLike>(a: DayTask<T>, b: DayTask<T>) =>
  new Date(a.task.due!).getTime() - new Date(b.task.due!).getTime() || a.task.id - b.task.id;

/** Where `day` falls in the task's start-to-due span; null without a start or when both are the same day. */
export function spanRole(task: TaskLike, day: Date): SpanRole | null {
  if (!task.start || !task.due) return null;
  const s = startOfDay(new Date(task.start)).getTime();
  const e = startOfDay(new Date(task.due)).getTime();
  if (s >= e) return null;
  const d = startOfDay(day).getTime();
  if (d < s || d > e) return null;
  return d === s ? 'start' : d === e ? 'end' : 'middle';
}

/** The start moved by the same delta as the due date, or null when the task has no start. */
export function shiftStart(task: TaskLike, newDue: string): string | null {
  if (!task.start || !task.due) return null;
  const delta = new Date(newDue).getTime() - new Date(task.due).getTime();
  return new Date(new Date(task.start).getTime() + delta).toISOString();
}

/**
 * Tasks for one day: timed ones go on the grid, date-only ones in the all-day
 * strip. Done tasks stay on their day. Today's strip also lists open tasks
 * that became overdue on an earlier day, so nothing overdue is off-screen.
 * A task with a start before its due day is a span: it sits in the all-day
 * strip on every day it covers, timed or not.
 */
export function tasksForDay<T extends TaskLike>(tasks: T[], day: Date, now: Date): { timed: DayTask<T>[]; allDay: DayTask<T>[] } {
  const timed: DayTask<T>[] = [];
  const due: DayTask<T>[] = [];
  const carried: DayTask<T>[] = [];
  const isToday = sameDay(day, now);
  for (const task of tasks) {
    if (!task.due) continue;
    const d = new Date(task.due);
    const overdue = isOverdue(task, now);
    const role = spanRole(task, day);
    if (role) {
      due.push({ task, overdue, span: role });
      continue;
    }
    const entry = { task, overdue, span: null };
    if (sameDay(d, day)) (task.dueHasTime ? timed : due).push(entry);
    else if (isToday && entry.overdue && d < startOfDay(now)) carried.push(entry);
  }
  return { timed: timed.sort(byDue), allDay: [...due.sort(byDue), ...carried.sort(byDue)] };
}

/**
 * New due date for a keyboard move: ↑/↓ shift a timed task 15 minutes,
 * ←/→ shift any dated task a day at the same local time. Null when the key
 * does not apply.
 */
export function movedDue(task: TaskLike, key: string): string | null {
  if (!task.due) return null;
  const d = new Date(task.due);
  switch (key) {
    case 'ArrowUp':
    case 'ArrowDown':
      if (!task.dueHasTime) return null;
      return new Date(d.getTime() + (key === 'ArrowUp' ? -1 : 1) * MOVE_STEP_MIN * 60_000).toISOString();
    case 'ArrowLeft':
    case 'ArrowRight':
      return addDays(d, key === 'ArrowLeft' ? -1 : 1).toISOString();
    default:
      return null;
  }
}
