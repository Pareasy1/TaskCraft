import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ALL_COLUMNS, defaultColumns, gridRows, loadColumns, nextCell, nextSort, saveColumns, type GridSort } from './grid';
import type { Bucket, Plan, Priority, TaskStatus } from './types';

const plans: Plan[] = [
  { id: 1, name: 'A', color: '#000', sort: 2, openCount: 0 },
  { id: 2, name: 'B', color: '#000', sort: 1, openCount: 0 },
];
const buckets: Bucket[] = [
  { id: 10, planId: 1, name: 'x', sort: 2 },
  { id: 11, planId: 1, name: 'y', sort: 1 },
  { id: 20, planId: 2, name: 'z', sort: 1 },
];
let n = 0;
const mk = (o: Partial<{ id: number; title: string; status: TaskStatus; priority: Priority; planId: number; bucketId: number; sortOrder: number; parentId: number | null; due: string | null; labels: { id: number; name: string; color: string }[] }> = {}) => ({
  id: ++n, title: 't', status: 'todo' as TaskStatus, priority: 'medium' as Priority, planId: 1, bucketId: 10, sortOrder: 0, parentId: null as number | null,
  due: null as string | null, start: null as string | null, labels: [] as { id: number; name: string; color: string }[],
  dueHasTime: false, completedAt: null as string | null, subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, attachmentCount: 0, ...o,
});
const base = { scope: 1 as number | 'all', buckets, plans, labels: [] as number[], status: 'any' as const, chart: null, sort: null as GridSort, collapsed: new Set<number>() };
const ids = (rows: { task: { id: number } }[]) => rows.map((r) => r.task.id);

describe('columns and sort', () => {
  it('default columns', () => {
    expect(defaultColumns(1)).toEqual(ALL_COLUMNS.filter((c) => c !== 'plan'));
    expect(defaultColumns('all')).toEqual(ALL_COLUMNS);
  });
  it('nextSort cycles', () => {
    const a = nextSort(null, 'due');
    expect(a).toEqual({ column: 'due', dir: 'asc' });
    const d = nextSort(a, 'due');
    expect(d).toEqual({ column: 'due', dir: 'desc' });
    expect(nextSort(d, 'due')).toBeNull();
    expect(nextSort(d, 'title')).toEqual({ column: 'title', dir: 'asc' });
  });
});

describe('gridRows', () => {
  it('default order: bucket sort then sortOrder', () => {
    const a = mk({ bucketId: 10, sortOrder: 1 }), b = mk({ bucketId: 11, sortOrder: 5 }), c = mk({ bucketId: 10, sortOrder: 0 });
    expect(ids(gridRows([a, b, c], base))).toEqual([b.id, c.id, a.id]);
  });
  it('All: plan, bucket, sortOrder', () => {
    const a = mk({ planId: 1, bucketId: 11 }), b = mk({ planId: 2, bucketId: 20 });
    expect(ids(gridRows([a, b], { ...base, scope: 'all' }))).toEqual([b.id, a.id]);
  });
  it('sorts by due with empties last both ways; third click restores default', () => {
    const a = mk({ due: '2026-01-02T00:00:00Z', sortOrder: 1 }), b = mk({ due: '2026-01-01T00:00:00Z', sortOrder: 2 }), c = mk({ due: null, sortOrder: 0 });
    expect(ids(gridRows([a, b, c], { ...base, sort: { column: 'due', dir: 'asc' } }))).toEqual([b.id, a.id, c.id]);
    expect(ids(gridRows([a, b, c], { ...base, sort: { column: 'due', dir: 'desc' } }))).toEqual([a.id, b.id, c.id]);
    expect(ids(gridRows([a, b, c], { ...base, sort: null }))).toEqual([c.id, a.id, b.id]);
  });
  it('sorts by priority and title', () => {
    const a = mk({ priority: 'low', title: 'b' }), b = mk({ priority: 'urgent', title: 'A' });
    expect(ids(gridRows([a, b], { ...base, sort: { column: 'priority', dir: 'asc' } }))).toEqual([b.id, a.id]);
    expect(ids(gridRows([a, b], { ...base, sort: { column: 'title', dir: 'asc' } }))).toEqual([b.id, a.id]);
  });
  it('subtasks stay under parent and sort among themselves', () => {
    const p = mk({ due: '2026-01-01T00:00:00Z' }), q = mk({ due: '2026-02-01T00:00:00Z' });
    const c1 = mk({ parentId: p.id, due: '2026-03-01T00:00:00Z' }), c2 = mk({ parentId: p.id, due: '2026-02-01T00:00:00Z' });
    const rows = gridRows([c1, q, c2, p], { ...base, sort: { column: 'due', dir: 'asc' } });
    expect(ids(rows)).toEqual([p.id, c2.id, c1.id, q.id]);
    expect(rows.map((r) => r.depth)).toEqual([0, 1, 1, 0]);
    expect(rows[0].hasChildren).toBe(true);
  });
  it('chart filter by status, priority and group', () => {
    const a = mk({ status: 'done', priority: 'low', bucketId: 10 }), b = mk({ status: 'todo', priority: 'urgent', bucketId: 11 });
    expect(ids(gridRows([a, b], { ...base, chart: { kind: 'status', value: 'done' } }))).toEqual([a.id]);
    expect(ids(gridRows([a, b], { ...base, chart: { kind: 'priority', value: 'urgent' } }))).toEqual([b.id]);
    expect(ids(gridRows([a, b], { ...base, chart: { kind: 'group', value: 11 } }))).toEqual([b.id]);
    const c = mk({ planId: 2, bucketId: 20 });
    expect(ids(gridRows([a, c], { ...base, scope: 'all', chart: { kind: 'group', value: 2 } }))).toEqual([c.id]);
  });
  it('collapsed parent hides children', () => {
    const p = mk(), c = mk({ parentId: p.id });
    const rows = gridRows([p, c], { ...base, collapsed: new Set([p.id]) });
    expect(ids(rows)).toEqual([p.id]);
    expect(rows[0].collapsed).toBe(true);
  });
  it('dimmed parent for filtered subtask', () => {
    const p = mk({ status: 'todo' }), c = mk({ parentId: p.id, status: 'done' });
    const rows = gridRows([p, c], { ...base, status: 'done' });
    expect(ids(rows)).toEqual([p.id, c.id]);
    expect(rows[0].dimmed).toBe(true);
    expect(rows[1].dimmed).toBe(false);
  });
  it('label filter', () => {
    const a = mk({ labels: [{ id: 5, name: 'q', color: '' }] }), b = mk();
    expect(ids(gridRows([a, b], { ...base, labels: [5] }))).toEqual([a.id]);
  });
});

describe('column storage', () => {
  beforeEach(() => {
    const m = new Map<string, string>();
    vi.stubGlobal('localStorage', { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) });
  });
  afterEach(() => vi.unstubAllGlobals());
  it('round-trips and keeps title', () => {
    saveColumns(1, ['due', 'status']);
    expect(localStorage.getItem('taskcraft.grid.columns.1')).not.toBeNull();
    expect(loadColumns(1)).toEqual(['title', 'due', 'status']);
  });
  it('defaults when empty or invalid; drops unknown names', () => {
    expect(loadColumns('all')).toEqual(defaultColumns('all'));
    localStorage.setItem('taskcraft.grid.columns.all', 'nope');
    expect(loadColumns('all')).toEqual(defaultColumns('all'));
    localStorage.setItem('taskcraft.grid.columns.all', JSON.stringify(['title', 'bogus', 'due']));
    expect(loadColumns('all')).toEqual(['title', 'due']);
  });
  it('survives throwing storage', () => {
    vi.stubGlobal('localStorage', { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } });
    expect(loadColumns(1)).toEqual(defaultColumns(1));
    expect(() => saveColumns(1, ['title'])).not.toThrow();
  });
});

describe('nextCell', () => {
  const shape = { rows: 3, cols: 4 };
  it('arrows move and clamp', () => {
    expect(nextCell({ row: 1, col: 1 }, 'ArrowDown', shape)).toEqual({ row: 2, col: 1 });
    expect(nextCell({ row: 2, col: 1 }, 'ArrowDown', shape)).toEqual({ row: 2, col: 1 });
    expect(nextCell({ row: 0, col: 0 }, 'ArrowUp', shape)).toEqual({ row: 0, col: 0 });
    expect(nextCell({ row: 0, col: 0 }, 'ArrowLeft', shape)).toEqual({ row: 0, col: 0 });
    expect(nextCell({ row: 0, col: 3 }, 'ArrowRight', shape)).toEqual({ row: 0, col: 3 });
    expect(nextCell({ row: 0, col: 2 }, 'ArrowRight', shape)).toEqual({ row: 0, col: 3 });
  });
  it('Home/End stay in row; others unchanged', () => {
    expect(nextCell({ row: 1, col: 2 }, 'Home', shape)).toEqual({ row: 1, col: 0 });
    expect(nextCell({ row: 1, col: 2 }, 'End', shape)).toEqual({ row: 1, col: 3 });
    expect(nextCell({ row: 1, col: 2 }, 'x', shape)).toEqual({ row: 1, col: 2 });
  });
});
