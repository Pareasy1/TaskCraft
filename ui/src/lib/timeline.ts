// Planner timeline geometry: pure helpers mapping tasks to day columns and back.
import { monthTitle } from './format';
import { groupWithParents } from './subtasks';
import { applyFilters, type StatusFilter } from './tasks';
import type { Bucket, Plan, TaskStatus } from './types';

export type Zoom = 'days' | 'weeks' | 'months';
export type DragMode = 'move' | 'start' | 'due';

/** Height of every Planner row (name column, bars and arrows share it). */
export const ROW_H = 32;

export const DAY_WIDTH: Record<Zoom, number> = { days: 32, weeks: 14, months: 4 };

const ZOOM_KEY = 'taskcraft.planner.zoom';
const UNDATED_KEY = 'taskcraft.planner.undated';
const ZOOMS: Zoom[] = ['days', 'weeks', 'months'];
const MS_DAY = 86_400_000;

export function loadZoom(): Zoom {
  try {
    const v = localStorage.getItem(ZOOM_KEY);
    if (ZOOMS.includes(v as Zoom)) return v as Zoom;
  } catch { /* storage unavailable */ }
  return 'weeks';
}

export function saveZoom(z: Zoom) {
  try { localStorage.setItem(ZOOM_KEY, z); } catch { /* storage unavailable */ }
}

export function loadUndatedOpen(): boolean {
  try {
    const v = localStorage.getItem(UNDATED_KEY);
    if (v === 'true') return true;
    if (v === 'false') return false;
  } catch { /* storage unavailable */ }
  return true;
}

export function saveUndatedOpen(b: boolean) {
  try { localStorage.setItem(UNDATED_KEY, String(b)); } catch { /* storage unavailable */ }
}

/** Local calendar days since 1970-01-01 (DST-safe: built from local y/m/d). */
export const dayNumber = (d: Date): number => Math.round(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / MS_DAY);

/** Local midnight of a day number. */
export function dayDate(n: number): Date {
  const u = new Date(n * MS_DAY);
  return new Date(u.getUTCFullYear(), u.getUTCMonth(), u.getUTCDate());
}

export function barSpan(t: { start: string | null; due: string | null }): { from: number; to: number } | null {
  const s = t.start ? dayNumber(new Date(t.start)) : null;
  const d = t.due ? dayNumber(new Date(t.due)) : null;
  if (s === null && d === null) return null;
  if (s === null) return { from: d!, to: d! };
  if (d === null) return { from: s, to: s };
  return { from: Math.min(s, d), to: Math.max(s, d) };
}

const MIN_DAYS = 92;

export function timelineRange(
  tasks: { start: string | null; due: string | null }[],
  today: Date,
): { from: number; to: number } {
  const t = dayNumber(today);
  let lo = t, hi = t;
  for (const task of tasks) {
    const s = barSpan(task);
    if (!s) continue;
    lo = Math.min(lo, s.from);
    hi = Math.max(hi, s.to);
  }
  const from = lo - 14;
  let to = hi + 28;
  if (to - from + 1 < MIN_DAYS) to = from + MIN_DAYS - 1;
  return { from, to };
}

export const dayToX = (day: number, range: { from: number }, zoom: Zoom): number =>
  (day - range.from) * DAY_WIDTH[zoom];

export const xToDay = (x: number, range: { from: number }, zoom: Zoom): number =>
  range.from + Math.floor(x / DAY_WIDTH[zoom]);

type Dates = { start: string | null; startHasTime: boolean; due: string | null; dueHasTime: boolean };

/** The same time of day on another local calendar day. */
function onDay(iso: string, day: number): string {
  const d = new Date(iso);
  const t = dayDate(day);
  return new Date(t.getFullYear(), t.getMonth(), t.getDate(), d.getHours(), d.getMinutes(), d.getSeconds(), d.getMilliseconds()).toISOString();
}

const allDay = (day: number) => dayDate(day).toISOString();

export function dragResult(t: Dates, mode: DragMode, deltaDays: number): Dates {
  const out: Dates = { start: t.start, startHasTime: t.startHasTime, due: t.due, dueHasTime: t.dueHasTime };
  const sDay = t.start ? dayNumber(new Date(t.start)) : null;
  const dDay = t.due ? dayNumber(new Date(t.due)) : null;

  if (mode === 'move') {
    if (sDay !== null) out.start = onDay(t.start!, sDay + deltaDays);
    if (dDay !== null) out.due = onDay(t.due!, dDay + deltaDays);
  } else if (mode === 'start') {
    // Candidate start; if it lands after due as an instant, it takes due's exact instant and flag.
    const cand = sDay !== null ? onDay(t.start!, sDay + deltaDays) : dDay !== null ? allDay(dDay + deltaDays) : null;
    if (cand !== null) {
      const over = t.due !== null && new Date(cand).getTime() > new Date(t.due).getTime();
      out.start = over ? t.due : cand;
      out.startHasTime = over ? t.dueHasTime : sDay !== null ? t.startHasTime : false;
    }
  } else {
    // Candidate due; if it lands before start as an instant, it takes start's exact instant and flag.
    const cand = dDay !== null ? onDay(t.due!, dDay + deltaDays) : sDay !== null ? allDay(sDay + deltaDays) : null;
    if (cand !== null) {
      const under = t.start !== null && new Date(cand).getTime() < new Date(t.start).getTime();
      out.due = under ? t.start : cand;
      out.dueHasTime = under ? t.startHasTime : dDay !== null ? t.dueHasTime : false;
    }
  }
  return out;
}

export function placeUndated(day: number): { start: string; startHasTime: false; due: string; dueHasTime: false } {
  const iso = allDay(day);
  return { start: iso, startHasTime: false, due: iso, dueHasTime: false };
}

export function headerTicks(
  range: { from: number; to: number },
  zoom: Zoom,
  weekStart: 0 | 1,
): { day: number; label: string; major: boolean }[] {
  const ticks: { day: number; label: string; major: boolean }[] = [];
  for (let n = range.from; n <= range.to; n++) {
    const d = dayDate(n);
    const monthStart = d.getDate() === 1;
    if (zoom === 'days') {
      ticks.push({ day: n, label: monthStart ? monthTitle(d) : String(d.getDate()), major: monthStart });
    } else if (zoom === 'weeks') {
      if (d.getDay() !== weekStart) continue;
      const major = d.getDate() <= 7;
      ticks.push({ day: n, label: major ? monthTitle(d) : String(d.getDate()), major });
    } else if (monthStart) {
      ticks.push({ day: n, label: monthTitle(d), major: d.getMonth() === 0 });
    }
  }
  return ticks;
}

// ── Rows ──

export type TimelineRow<T> =
  | { kind: 'group'; key: string; label: string; count: number; collapsed: boolean }
  | { kind: 'task'; task: T; depth: 0 | 1; dimmed: boolean; hasChildren: boolean; collapsed: boolean };

type RowTask = {
  id: number; status: TaskStatus; planId: number; bucketId: number; sortOrder: number; parentId: number | null;
  labels: { id: number }[]; due: string | null; dueHasTime: boolean; completedAt: string | null;
};

export interface TimelineOpts {
  scope: number | 'all'; buckets: Bucket[]; plans: Plan[]; labels: number[]; status: StatusFilter;
  /** Collapsed group keys (`b:<id>` / `p:<id>`) and parent task ids. */
  collapsed: Set<string | number>;
}

/**
 * Planner rows: top-level tasks grouped by bucket (by plan in All tasks), groups and tasks in their
 * default order, subtasks under their parent. Dated and undated tasks both get rows. A group's
 * count is its matching tasks (dimmed parents excluded); empty groups are left out.
 */
export function timelineRows<T extends RowTask>(tasks: T[], opts: TimelineOpts): TimelineRow<T>[] {
  const all = opts.scope === 'all';
  const planSort = new Map(opts.plans.map((p) => [p.id, p.sort]));
  const bucketSort = new Map(opts.buckets.map((b) => [b.id, b.sort]));
  const inScope = all ? tasks : tasks.filter((x) => x.planId === opts.scope);
  const shown = applyFilters(inScope, { status: opts.status, labels: opts.labels });
  const byDefault = (a: T, b: T) =>
    (all ? (planSort.get(a.planId) ?? 0) - (planSort.get(b.planId) ?? 0) : 0) ||
    (bucketSort.get(a.bucketId) ?? 0) - (bucketSort.get(b.bucketId) ?? 0) ||
    a.sortOrder - b.sortOrder || a.id - b.id;

  const groups = new Map<string, { id: number; sort: number; label: string; items: ReturnType<typeof groupWithParents<T>> }>();
  for (const g of groupWithParents<T>(shown, inScope).sort((a, b) => byDefault(a.parent, b.parent))) {
    const id = all ? g.parent.planId : g.parent.bucketId;
    const key = `${all ? 'p' : 'b'}:${id}`;
    if (!groups.has(key)) {
      const label = all ? opts.plans.find((p) => p.id === id)?.name : opts.buckets.find((b) => b.id === id)?.name;
      groups.set(key, { id, sort: (all ? planSort : bucketSort).get(id) ?? 0, label: label ?? '', items: [] });
    }
    groups.get(key)!.items.push(g);
  }

  const rows: TimelineRow<T>[] = [];
  for (const [key, grp] of [...groups].sort((a, b) => a[1].sort - b[1].sort || a[1].id - b[1].id)) {
    const count = grp.items.reduce((n, g) => n + (g.dimmed ? 0 : 1) + g.children.length, 0);
    const groupCollapsed = opts.collapsed.has(key);
    rows.push({ kind: 'group', key, label: grp.label, count, collapsed: groupCollapsed });
    if (groupCollapsed) continue;
    for (const g of grp.items) {
      // A dimmed parent is only there for its matching subtasks, so it never hides them.
      const collapsed = !g.dimmed && opts.collapsed.has(g.parent.id);
      rows.push({ kind: 'task', task: g.parent, depth: 0, dimmed: g.dimmed, hasChildren: g.children.length > 0, collapsed });
      if (collapsed) continue;
      for (const c of g.children) rows.push({ kind: 'task', task: c, depth: 1, dimmed: false, hasChildren: false, collapsed: false });
    }
  }
  return rows;
}
