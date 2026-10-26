// Pure logic for the Grid view: rows, sorting, columns and keyboard movement.
import { dateLocale } from './i18n';
import { groupWithParents } from './subtasks';
import { applyFilters, type StatusFilter } from './tasks';
import type { Bucket, Plan, Priority, TaskLabel, TaskStatus } from './types';

export type GridColumn = 'title' | 'status' | 'priority' | 'bucket' | 'plan' | 'labels' | 'start' | 'due' | 'progress' | 'attachments';
export const ALL_COLUMNS: GridColumn[] = ['title', 'status', 'priority', 'bucket', 'plan', 'labels', 'start', 'due', 'progress', 'attachments'];

export type GridScope = number | 'all';
export type GridSort = { column: GridColumn; dir: 'asc' | 'desc' } | null;
export type ChartFilter =
  | { kind: 'status'; value: TaskStatus }
  | { kind: 'priority'; value: Priority }
  | { kind: 'group'; value: number };

export interface GridRow<T> { task: T; depth: 0 | 1; dimmed: boolean; hasChildren: boolean; collapsed: boolean }

type GridTask = {
  id: number; title: string; status: TaskStatus; priority: Priority; planId: number; bucketId: number; sortOrder: number;
  parentId: number | null; labels: TaskLabel[]; start: string | null; due: string | null; dueHasTime: boolean; completedAt: string | null;
  subtaskOpen: number; subtaskTotal: number; attachmentCount: number;
};

export interface GridOpts {
  scope: GridScope; buckets: Bucket[]; plans: Plan[]; labels: number[]; status: StatusFilter;
  chart: ChartFilter | null; sort: GridSort; collapsed: Set<number>;
}

export function defaultColumns(scope: GridScope): GridColumn[] {
  return scope === 'all' ? [...ALL_COLUMNS] : ALL_COLUMNS.filter((c) => c !== 'plan');
}

export function nextSort(current: GridSort, column: GridColumn): GridSort {
  if (!current || current.column !== column) return { column, dir: 'asc' };
  return current.dir === 'asc' ? { column, dir: 'desc' } : null;
}

const STATUS_RANK: Record<TaskStatus, number> = { todo: 0, inProgress: 1, done: 2 };
const PRIORITY_RANK: Record<Priority, number> = { urgent: 0, important: 1, medium: 2, low: 3 };

export function gridRows<T extends GridTask>(tasks: T[], opts: GridOpts): GridRow<T>[] {
  const planSort = new Map(opts.plans.map((p) => [p.id, p.sort]));
  const bucketSort = new Map(opts.buckets.map((b) => [b.id, b.sort]));
  const inScope = opts.scope === 'all' ? tasks : tasks.filter((x) => x.planId === opts.scope);

  const chart = opts.chart;
  const chartOk = (x: T) =>
    !chart ||
    (chart.kind === 'status' ? x.status === chart.value
      : chart.kind === 'priority' ? x.priority === chart.value
      : (opts.scope === 'all' ? x.planId : x.bucketId) === chart.value);
  const shown = applyFilters(inScope, { status: opts.status, labels: opts.labels }).filter(chartOk);

  const byDefault = (a: T, b: T) =>
    (opts.scope === 'all' ? (planSort.get(a.planId) ?? 0) - (planSort.get(b.planId) ?? 0) : 0) ||
    (bucketSort.get(a.bucketId) ?? 0) - (bucketSort.get(b.bucketId) ?? 0) ||
    a.sortOrder - b.sortOrder || a.id - b.id;

  const sort = opts.sort;
  const key = (x: T): string | number | null => {
    switch (sort?.column) {
      case 'title': return x.title.trim() ? x.title : null;
      case 'status': return STATUS_RANK[x.status];
      case 'priority': return PRIORITY_RANK[x.priority];
      case 'bucket': return bucketSort.get(x.bucketId) ?? null;
      case 'plan': return planSort.get(x.planId) ?? null;
      case 'labels': return x.labels[0]?.name ?? null;
      case 'start': return x.start ? new Date(x.start).getTime() : null;
      case 'due': return x.due ? new Date(x.due).getTime() : null;
      case 'progress': return x.subtaskTotal ? (x.subtaskTotal - x.subtaskOpen) / x.subtaskTotal : null;
      case 'attachments': return x.attachmentCount;
      default: return null;
    }
  };
  const cmp = (a: T, b: T): number => {
    if (sort) {
      const ka = key(a), kb = key(b);
      if (ka !== kb) {
        if (ka === null) return 1;
        if (kb === null) return -1;
        const c = typeof ka === 'string' && typeof kb === 'string'
          ? ka.localeCompare(kb, dateLocale, { sensitivity: 'base' })
          : (ka as number) - (kb as number);
        if (c) return sort.dir === 'asc' ? c : -c;
      }
    }
    return byDefault(a, b);
  };

  const rows: GridRow<T>[] = [];
  const groups = groupWithParents<T>(shown, inScope);
  const top = groups.sort((a, b) => cmp(a.parent, b.parent));
  for (const g of top) {
    const collapsed = opts.collapsed.has(g.parent.id);
    rows.push({ task: g.parent, depth: 0, dimmed: g.dimmed, hasChildren: g.children.length > 0, collapsed });
    if (collapsed) continue;
    for (const c of [...g.children].sort(cmp)) rows.push({ task: c, depth: 1, dimmed: false, hasChildren: false, collapsed: false });
  }
  return rows;
}

const colKey = (scope: GridScope) => `taskcraft.grid.columns.${scope}`;

export function loadColumns(scope: GridScope): GridColumn[] {
  try {
    const raw = localStorage.getItem(colKey(scope));
    if (raw) {
      const parsed: unknown = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        const cols = parsed.filter((c): c is GridColumn => ALL_COLUMNS.includes(c as GridColumn));
        return cols.includes('title') ? cols : ['title', ...cols];
      }
    }
  } catch { /* fall through to defaults */ }
  return defaultColumns(scope);
}

export function saveColumns(scope: GridScope, cols: GridColumn[]): void {
  const clean = cols.filter((c) => ALL_COLUMNS.includes(c));
  try {
    localStorage.setItem(colKey(scope), JSON.stringify(clean.includes('title') ? clean : ['title', ...clean]));
  } catch { /* storage unavailable */ }
}

export function nextCell(pos: { row: number; col: number }, key: string, shape: { rows: number; cols: number }): { row: number; col: number } {
  const clamp = (v: number, n: number) => Math.max(0, Math.min(v, n - 1));
  switch (key) {
    case 'ArrowUp': return { row: clamp(pos.row - 1, shape.rows), col: pos.col };
    case 'ArrowDown': return { row: clamp(pos.row + 1, shape.rows), col: pos.col };
    case 'ArrowLeft': return { row: pos.row, col: clamp(pos.col - 1, shape.cols) };
    case 'ArrowRight': return { row: pos.row, col: clamp(pos.col + 1, shape.cols) };
    case 'Home': return { row: pos.row, col: 0 };
    case 'End': return { row: pos.row, col: Math.max(0, shape.cols - 1) };
    default: return pos;
  }
}
