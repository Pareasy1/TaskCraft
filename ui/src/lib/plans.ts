// Pure helpers for plans, buckets and the grouped board.
import { startOfDay } from './layout';
import { isOverdue } from './tasks';
import type { Bucket, Plan, TaskLabel } from './types';

export type GroupBy = 'bucket' | 'status' | 'due' | 'label';

/** One column per bucket in `sort` order (empty ones kept); cards by (sortOrder, id). */
export function bucketColumns<T extends { bucketId: number; sortOrder: number; id: number }>(
  buckets: Bucket[],
  tasks: T[],
): { bucket: Bucket; tasks: T[] }[] {
  return [...buckets]
    .sort((a, b) => a.sort - b.sort || a.id - b.id)
    .map((bucket) => ({
      bucket,
      tasks: tasks.filter((t) => t.bucketId === bucket.id).sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id),
    }));
}

/**
 * Neighbours for a drop. `ids` is the target column's current order (it may contain
 * `movingId`); `index` is the insertion index in that list without the moving card.
 */
export function dropNeighbours(ids: number[], movingId: number, index: number): { before: number | null; after: number | null } {
  const rest = ids.filter((id) => id !== movingId);
  return { before: index > 0 ? rest[index - 1] ?? null : null, after: rest[index] ?? null };
}

export type DueKey = 'overdue' | 'today' | 'thisWeek' | 'later' | 'nodate';
const DUE_KEYS: DueKey[] = ['overdue', 'today', 'thisWeek', 'later', 'nodate'];

type DueLike = { id: number; due: string | null; dueHasTime: boolean; completedAt: string | null };

/** Open tasks by due date; "this week" runs through the coming Sunday. */
export function dueColumns<T extends DueLike>(tasks: T[], now: Date): { key: DueKey; tasks: T[] }[] {
  const today = startOfDay(now);
  const endOfWeek = new Date(today);
  endOfWeek.setDate(today.getDate() + ((7 - today.getDay()) % 7));
  const cols = new Map<DueKey, T[]>(DUE_KEYS.map((k) => [k, []]));
  for (const t of tasks) {
    if (t.completedAt) continue;
    let key: DueKey;
    if (!t.due) key = 'nodate';
    else if (isOverdue(t, now)) key = 'overdue';
    else {
      const day = startOfDay(new Date(t.due));
      key = day.getTime() <= today.getTime() ? 'today' : day.getTime() <= endOfWeek.getTime() ? 'thisWeek' : 'later';
    }
    cols.get(key)!.push(t);
  }
  const byDue = (a: T, b: T) =>
    (a.due ? new Date(a.due).getTime() : Infinity) - (b.due ? new Date(b.due).getTime() : Infinity) || a.id - b.id;
  return DUE_KEYS.map((key) => ({ key, tasks: cols.get(key)!.sort(byDue) }));
}

/** One column per label (a task appears under each of its labels) plus "no label". */
export function labelColumns<T extends { id: number; labels: { id: number }[] }>(
  tasks: T[],
  labels: TaskLabel[],
): { key: string; label: TaskLabel | null; tasks: T[] }[] {
  return [
    ...labels.map((label) => ({ key: String(label.id), label, tasks: tasks.filter((t) => t.labels.some((l) => l.id === label.id)) })),
    { key: 'nolabel', label: null, tasks: tasks.filter((t) => !t.labels.length) },
  ];
}

export const PLAN_COLORS = ['#3b82f6', '#f97316', '#10b981', '#a855f7', '#ef4444', '#eab308', '#06b6d4', '#ec4899'];

/** First palette colour no plan uses, else cycles by plan count. */
export function nextPlanColor(plans: Plan[]): string {
  const used = new Set(plans.map((p) => p.color.toLowerCase()));
  return PLAN_COLORS.find((c) => !used.has(c)) ?? PLAN_COLORS[plans.length % PLAN_COLORS.length];
}
