// Which tasks the Day/Week/Month views show, and how a keyboard move changes a due date.
import { addDays, sameDay, startOfDay } from './layout';
import { isOverdue } from './tasks';

type TaskLike = { id: number; due: string | null; dueHasTime: boolean; completedAt: string | null };

/** A timed task occupies this many minutes in the day's overlap layout (tasks have no duration). */
export const TASK_SLOT_MIN = 15;

export interface DayTask<T> {
  task: T;
  overdue: boolean;
}

const byDue = <T extends TaskLike>(a: DayTask<T>, b: DayTask<T>) =>
  new Date(a.task.due!).getTime() - new Date(b.task.due!).getTime() || a.task.id - b.task.id;

/**
 * Tasks for one day: timed ones go on the grid, date-only ones in the all-day
 * strip. Done tasks stay on their day. Today's strip also lists open tasks
 * that became overdue on an earlier day, so nothing overdue is off-screen.
 */
export function tasksForDay<T extends TaskLike>(tasks: T[], day: Date, now: Date): { timed: DayTask<T>[]; allDay: DayTask<T>[] } {
  const timed: DayTask<T>[] = [];
  const due: DayTask<T>[] = [];
  const carried: DayTask<T>[] = [];
  const isToday = sameDay(day, now);
  for (const task of tasks) {
    if (!task.due) continue;
    const d = new Date(task.due);
    const entry = { task, overdue: isOverdue(task, now) };
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
      return new Date(d.getTime() + (key === 'ArrowUp' ? -1 : 1) * TASK_SLOT_MIN * 60_000).toISOString();
    case 'ArrowLeft':
    case 'ArrowRight':
      return addDays(d, key === 'ArrowLeft' ? -1 : 1).toISOString();
    default:
      return null;
  }
}
