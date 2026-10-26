// Pure statistics for the Grid charts panel, plus the chart-filter toggle.
import type { ChartFilter, GridScope } from './grid';
import { addDays, startOfDay } from './layout';
import { isOverdue } from './tasks';
import { PRIORITIES, type Bucket, type Plan, type Priority, type TaskStatus } from './types';

export interface GridStats {
  open: number; inProgress: number; overdue: number; completed7d: number;
  byStatus: { key: TaskStatus; n: number }[];
  byPriority: { key: Priority; n: number }[];
  byGroup: { key: number; label: string; color?: string; n: number }[];
  perWeek: { weekStart: Date; n: number }[];
}

type StatTask = {
  planId: number; bucketId: number; parentId: number | null; status: TaskStatus; priority: Priority;
  id: number; due: string | null; dueHasTime: boolean; completedAt: string | null;
};

export interface GridStatsOpts { scope: GridScope; buckets: Bucket[]; plans: Plan[]; now: Date; weekStart: 0 | 1 }

const STATUSES: TaskStatus[] = ['todo', 'inProgress', 'done'];
const WEEKS = 8;
const DAY_MS = 24 * 3600 * 1000;

/** Local midnight of the first day of the week containing `d`. */
function weekStartOf(d: Date, weekStart: 0 | 1): Date {
  const day = startOfDay(d);
  return addDays(day, -((day.getDay() - weekStart + 7) % 7));
}

export function gridStats(tasks: StatTask[], opts: GridStatsOpts): GridStats {
  const { now } = opts;
  const top = tasks.filter((x) => x.parentId === null && (opts.scope === 'all' || x.planId === opts.scope));

  const cutoff = now.getTime() - 7 * DAY_MS;
  const completedMs = (x: StatTask) => (x.completedAt ? new Date(x.completedAt).getTime() : null);

  const groups = opts.scope === 'all'
    ? [...opts.plans].sort((a, b) => a.sort - b.sort).map((p) => ({ key: p.id, label: p.name, color: p.color as string | undefined }))
    : opts.buckets.filter((b) => b.planId === opts.scope).sort((a, b) => a.sort - b.sort)
        .map((b) => ({ key: b.id, label: b.name, color: undefined as string | undefined }));

  const thisWeek = weekStartOf(now, opts.weekStart);
  const weeks = Array.from({ length: WEEKS }, (_, i) => addDays(thisWeek, (i - (WEEKS - 1)) * 7));
  const perWeek = weeks.map((w) => ({ weekStart: w, n: 0 }));
  for (const x of top) {
    const ms = completedMs(x);
    if (ms === null) continue;
    const start = weekStartOf(new Date(ms), opts.weekStart).getTime();
    const slot = perWeek.find((w) => w.weekStart.getTime() === start);
    if (slot) slot.n++;
  }

  return {
    open: top.filter((x) => x.status !== 'done').length,
    inProgress: top.filter((x) => x.status === 'inProgress').length,
    overdue: top.filter((x) => isOverdue(x, now)).length,
    completed7d: top.filter((x) => { const ms = completedMs(x); return ms !== null && ms >= cutoff && ms <= now.getTime(); }).length,
    byStatus: STATUSES.map((key) => ({ key, n: top.filter((x) => x.status === key).length })),
    byPriority: PRIORITIES.map((key) => ({ key, n: top.filter((x) => x.priority === key).length })),
    byGroup: groups.map((g) => {
      const n = top.filter((x) => (opts.scope === 'all' ? x.planId : x.bucketId) === g.key).length;
      return g.color === undefined ? { key: g.key, label: g.label, n } : { ...g, n };
    }),
    perWeek,
  };
}

const sameFilter = (a: ChartFilter, b: ChartFilter) => a.kind === b.kind && a.value === b.value;

/** Clicking the active segment clears the filter; any other segment replaces it. */
export function toggleChartFilter(current: ChartFilter | null, next: ChartFilter): ChartFilter | null {
  return current && sameFilter(current, next) ? null : next;
}
