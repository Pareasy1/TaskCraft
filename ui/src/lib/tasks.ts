// Filtering and grouping for the Tasks view.
import { sameDay, startOfDay } from './layout';
import type { TaskStatus } from './types';
import { t } from './i18n';

export type TaskView = 'all' | 'today' | 'upcoming' | 'nodate' | 'completed';
export const TASK_VIEWS: TaskView[] = ['all', 'today', 'upcoming', 'nodate', 'completed'];

type TaskLike = { id: number; due: string | null; dueHasTime: boolean; completedAt: string | null; start?: string | null };

/** True when the task has a start date after today (local): it is not started yet. */
const notStarted = (t: TaskLike, now: Date) => !!t.start && startOfDay(new Date(t.start)) > startOfDay(now);

export interface TaskGroup<T> {
  key: string;
  kind: 'overdue' | 'day' | 'nodate' | 'completed';
  /** The due day for `kind === 'day'`, otherwise null. */
  day: Date | null;
  tasks: T[];
}

/** Timed tasks are overdue once their time passes; date-only ones from the next day. */
export function isOverdue(t: TaskLike, now: Date): boolean {
  if (!t.due || t.completedAt) return false;
  const due = new Date(t.due);
  return t.dueHasTime ? due < now : startOfDay(due) < startOfDay(now);
}

const byDue = (a: TaskLike, b: TaskLike) =>
  (a.due ? new Date(a.due).getTime() : Infinity) - (b.due ? new Date(b.due).getTime() : Infinity) || a.id - b.id;

export function filterTasks<T extends TaskLike>(tasks: T[], view: TaskView, now: Date): T[] {
  if (view === 'completed') {
    return tasks
      .filter((t) => t.completedAt)
      .sort((a, b) => new Date(b.completedAt!).getTime() - new Date(a.completedAt!).getTime());
  }
  const today = startOfDay(now);
  const keep = (t: T) => {
    if (t.completedAt) return false;
    const due = t.due ? new Date(t.due) : null;
    switch (view) {
      case 'all': return true;
      case 'nodate': return !due;
      case 'today': return !!due && !notStarted(t, now) && (isOverdue(t, now) || sameDay(due, now));
      case 'upcoming': return !!due && startOfDay(due) > today;
    }
  };
  return tasks.filter(keep).sort(byDue);
}

/** Groups a view's tasks: overdue first, then one group per due day, then those without a date. */
export function groupTasks<T extends TaskLike>(tasks: T[], view: TaskView, now: Date): TaskGroup<T>[] {
  const list = filterTasks(tasks, view, now);
  if (view === 'completed') return list.length ? [{ key: 'completed', kind: 'completed', day: null, tasks: list }] : [];
  const overdue: T[] = [];
  const nodate: T[] = [];
  const days: TaskGroup<T>[] = [];
  for (const t of list) {
    if (!t.due) nodate.push(t);
    else if (isOverdue(t, now)) overdue.push(t);
    else {
      const day = startOfDay(new Date(t.due));
      const last = days[days.length - 1];
      if (last && last.day && sameDay(last.day, day)) last.tasks.push(t);
      else days.push({ key: `d${day.getTime()}`, kind: 'day', day, tasks: [t] });
    }
  }
  return [
    ...(overdue.length ? [{ key: 'overdue', kind: 'overdue' as const, day: null, tasks: overdue }] : []),
    ...days,
    ...(nodate.length ? [{ key: 'nodate', kind: 'nodate' as const, day: null, tasks: nodate }] : []),
  ];
}

export function taskCounts(tasks: TaskLike[], now: Date): Record<TaskView, number> {
  return Object.fromEntries(TASK_VIEWS.map((v) => [v, filterTasks(tasks, v, now).length])) as Record<TaskView, number>;
}

// ── Status and labels ────────────────────────────────────────────────

export type StatusFilter = 'any' | 'todo' | 'inProgress' | 'done';
type WithStatus = TaskLike & { status: TaskStatus; labels: { id: number }[] };

export const STATUSES: TaskStatus[] = ['todo', 'inProgress', 'done'];
export const STATUS_LABEL: Record<TaskStatus, string> = { todo: t('status.todo'), inProgress: t('status.inProgress'), done: t('status.done') };
const DONE_LIMIT = 30;

/** Status filter, then labels: a task matches if it has any of `labels` (empty = no label filter). */
export function applyFilters<T extends WithStatus>(tasks: T[], f: { status: StatusFilter; labels: number[] }): T[] {
  return tasks.filter(
    (t) =>
      (f.status === 'any' || t.status === f.status) &&
      (!f.labels.length || t.labels.some((l) => f.labels.includes(l.id))),
  );
}

/** Board columns To do · In progress · Done. Done is newest first and capped unless `showAllDone`. */
export function boardColumns<T extends WithStatus>(
  tasks: T[],
  opts: { showAllDone: boolean },
): { status: TaskStatus; tasks: T[]; hidden: number }[] {
  return STATUSES.map((status) => {
    const col = tasks.filter((t) => t.status === status);
    if (status !== 'done') return { status, tasks: col.sort(byDue), hidden: 0 };
    col.sort((a, b) => new Date(b.completedAt ?? 0).getTime() - new Date(a.completedAt ?? 0).getTime());
    const shown = opts.showAllDone ? col : col.slice(0, DONE_LIMIT);
    return { status, tasks: shown, hidden: col.length - shown.length };
  });
}

export function nextStatus(s: TaskStatus): TaskStatus {
  return STATUSES[(STATUSES.indexOf(s) + 1) % STATUSES.length];
}

/**
 * The Tasks page: open tasks for the date tab, filtered by status and labels.
 * Status "done" lists finished tasks instead (any date, newest first).
 */
export function taskPageGroups<T extends WithStatus>(
  tasks: T[],
  f: { view: TaskView; status: StatusFilter; labels: number[] },
  now: Date,
): TaskGroup<T>[] {
  if (f.status === 'done') return groupTasks(applyFilters(tasks, { status: 'any', labels: f.labels }), 'completed', now);
  return groupTasks(applyFilters(tasks, { status: f.status, labels: f.labels }), f.view, now);
}

const DONE_SHOWN = 30;

/** The sidebar's task groups, using the same rules as the Tasks page. */
export function sidebarSections<T extends TaskLike>(tasks: T[], now: Date): { key: 'overdue' | 'today' | 'upcoming' | 'nodate' | 'completed'; tasks: T[] }[] {
  const open = tasks.filter((t) => !t.completedAt);
  const today = startOfDay(now);
  return [
    { key: 'overdue', tasks: open.filter((t) => isOverdue(t, now)).sort(byDue) },
    { key: 'today', tasks: open.filter((t) => t.due && !notStarted(t, now) && !isOverdue(t, now) && sameDay(new Date(t.due), now)).sort(byDue) },
    { key: 'upcoming', tasks: open.filter((t) => t.due && startOfDay(new Date(t.due)) > today).sort(byDue) },
    { key: 'nodate', tasks: open.filter((t) => !t.due).sort((a, b) => a.id - b.id) },
    { key: 'completed', tasks: filterTasks(tasks, 'completed', now).slice(0, DONE_SHOWN) },
  ];
}
