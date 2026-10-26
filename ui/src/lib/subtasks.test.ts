import { describe, expect, it } from 'vitest';
import {
  checklistLabel, childrenByParent, completionAsk, groupWithParents, offerParentAfter, priorityColumns, progressLabel, progressSegments, topLevel,
} from './subtasks';
import type { Priority } from './types';

type T = { id: number; parentId: number | null; sortOrder: number; completedAt: string | null; priority: Priority };
const task = (id: number, parentId: number | null, sortOrder = id, completedAt: string | null = null, priority: Priority = 'medium'): T =>
  ({ id, parentId, sortOrder, completedAt, priority });
const ids = (xs: { id: number }[]) => xs.map((x) => x.id);

describe('childrenByParent / topLevel', () => {
  it('childrenByParent orders siblings', () => {
    const all = [task(1, null), task(2, 1, 30), task(3, 1, 10), task(4, 1, 20), task(5, null)];
    const m = childrenByParent(all);
    expect(ids(m.get(1)!)).toEqual([3, 4, 2]);
    expect(m.has(5)).toBe(false);
  });
  it('topLevel keeps only tasks without a parent', () => {
    expect(ids(topLevel([task(1, null), task(2, 1), task(3, null)]))).toEqual([1, 3]);
  });
});

describe('groupWithParents', () => {
  const all = [task(1, null), task(2, 1, 10), task(3, 1, 20), task(4, null)];
  it('parent and child shown', () => {
    const g = groupWithParents([all[0], all[1]], all);
    expect(g.map((x) => [x.parent.id, x.dimmed, ids(x.children)])).toEqual([[1, false, [2]]]);
  });
  it('child shown without its parent gets a dimmed parent', () => {
    const g = groupWithParents([all[2]], all);
    expect(g.map((x) => [x.parent.id, x.dimmed, ids(x.children)])).toEqual([[1, true, [3]]]);
  });
  it('child and parent both shown: parent once', () => {
    const g = groupWithParents([all[1], all[0], all[2], all[3]], all);
    expect(g.map((x) => [x.parent.id, x.dimmed, ids(x.children)])).toEqual([[1, false, [2, 3]], [4, false, []]]);
  });
  it('child whose parent is not in `all` (hidden calendar) is a plain row, not dropped', () => {
    const orphan = task(7, 99, 10);
    const g = groupWithParents([all[3], orphan], [...all, orphan]);
    expect(g.map((x) => [x.parent.id, x.dimmed, ids(x.children)])).toEqual([[4, false, []], [7, false, []]]);
  });
});

describe('labels', () => {
  it('progressLabel', () => {
    expect(progressLabel({ subtaskOpen: 0, subtaskTotal: 0 })).toBeNull();
    expect(progressLabel({ subtaskOpen: 2, subtaskTotal: 4 })).toBe('2 of 4');
  });
  it('progressSegments: done, in progress and to do counts', () => {
    expect(progressSegments({ subtaskTotal: 4, subtaskOpen: 2, subtaskInProgress: 1 })).toEqual({ done: 2, inProgress: 1, todo: 1 });
    expect(progressSegments({ subtaskTotal: 3, subtaskOpen: 3, subtaskInProgress: 0 })).toEqual({ done: 0, inProgress: 0, todo: 3 });
    expect(progressSegments({ subtaskTotal: 2, subtaskOpen: 0, subtaskInProgress: 0 })).toEqual({ done: 2, inProgress: 0, todo: 0 });
    expect(progressSegments({ subtaskTotal: 0, subtaskOpen: 0, subtaskInProgress: 0 })).toEqual({ done: 0, inProgress: 0, todo: 0 });
    // Never more in progress than open (stale counts can't push the bar past full).
    expect(progressSegments({ subtaskTotal: 2, subtaskOpen: 1, subtaskInProgress: 2 })).toEqual({ done: 1, inProgress: 1, todo: 0 });
  });
  it('checklistLabel', () => {
    expect(checklistLabel({ checklistDone: 0, checklistTotal: 0 })).toBeNull();
    expect(checklistLabel({ checklistDone: 3, checklistTotal: 5 })).toBe('3/5');
  });
});

describe('completionAsk / offerParentAfter', () => {
  it('asks when the task has open children', () => {
    expect(completionAsk(task(1, null), [task(2, 1), task(3, 1, 3, 'x')])).toBe('askChildren');
    expect(completionAsk(task(1, null), [task(3, 1, 3, 'x')])).toBe('none');
    expect(completionAsk(task(1, null), [])).toBe('none');
  });
  it('offers the parent when the last open sibling was completed', () => {
    const parent = task(1, null);
    const done = task(2, 1, 2, 'x');
    expect(offerParentAfter(done, [done, task(3, 1, 3, 'x')], parent)).toBe(true);
    expect(offerParentAfter(done, [done, task(3, 1)], parent)).toBe(false);
    expect(offerParentAfter(done, [done], task(1, null, 1, 'x'))).toBe(false);
  });
});

describe('priorityColumns', () => {
  it('Urgent, Important, Medium, Low, ordered by sortOrder', () => {
    const xs = [task(1, null, 20, null, 'low'), task(2, null, 10, null, 'urgent'), task(3, null, 5, null, 'low'), task(4, null, 1, null, 'medium')];
    const cols = priorityColumns(xs);
    expect(cols.map((c) => c.key)).toEqual(['urgent', 'important', 'medium', 'low']);
    expect(cols.map((c) => ids(c.tasks))).toEqual([[2], [], [4], [3, 1]]);
  });
});

import { parentCandidates, shiftId } from './subtasks';

describe('parentCandidates', () => {
  type P = { id: number; parentId: number | null; sortOrder: number; planId: number; title: string; completedAt: string | null };
  const p = (id: number, title: string, o: Partial<P> = {}): P => ({ id, parentId: null, sortOrder: id, planId: 1, title, completedAt: null, ...o });
  it('lists open top-level tasks of the same plan, without the task itself, by title', () => {
    const all = [p(1, 'Zeta'), p(2, 'Alpha'), p(3, 'Self'), p(4, 'Sub', { parentId: 1 }), p(5, 'Other plan', { planId: 2 }), p(6, 'Done', { completedAt: 'x' })];
    expect(ids(parentCandidates(all, 3, 1, null))).toEqual([2, 1]);
  });
  it('keeps the current parent even when it is completed', () => {
    const all = [p(1, 'Beta'), p(6, 'Alpha', { completedAt: 'x' })];
    expect(ids(parentCandidates(all, 3, 1, 6))).toEqual([6, 1]);
  });
});

describe('shiftId', () => {
  it('moves an id one place up or down', () => {
    expect(shiftId([1, 2, 3], 2, -1)).toEqual([2, 1, 3]);
    expect(shiftId([1, 2, 3], 2, 1)).toEqual([1, 3, 2]);
  });
  it('returns null at either end or when missing', () => {
    expect(shiftId([1, 2, 3], 1, -1)).toBeNull();
    expect(shiftId([1, 2, 3], 3, 1)).toBeNull();
    expect(shiftId([1, 2, 3], 9, 1)).toBeNull();
  });
});

import { focusAfterRemoval } from './subtasks';

describe('focusAfterRemoval', () => {
  it('picks the next row, else the previous one, else none', () => {
    expect(focusAfterRemoval([1, 2, 3], 2)).toBe(3);
    expect(focusAfterRemoval([1, 2, 3], 3)).toBe(2);
    expect(focusAfterRemoval([1], 1)).toBeNull();
    expect(focusAfterRemoval([1, 2], 9)).toBeNull();
  });
});

import { completeUndo, flattenRows, nestGroups, statusCtx } from './subtasks';
import type { TaskInput } from './types';

describe('completeUndo', () => {
  const input = (title: string, status: TaskInput['status'] = 'todo'): TaskInput => ({
    calendarId: 1, title, due: null, dueHasTime: false, tz: 'UTC', rrule: null, notes: null, alerts: [], status, labels: [],
  });
  it('completeUndoRestoresChildren: one update per closed child plus the parent', () => {
    const calls = completeUndo(1, input('Parent', 'inProgress'), [{ id: 2, input: input('A') }, { id: 3, input: input('B', 'inProgress') }]);
    expect(calls.map((c) => [c.id, c.input.title, c.input.status])).toEqual([
      [2, 'A', 'todo'], [3, 'B', 'inProgress'], [1, 'Parent', 'inProgress'],
    ]);
  });
  it('no closed children: only the parent', () => {
    expect(completeUndo(1, input('P'), []).map((c) => c.id)).toEqual([1]);
  });
});

describe('statusCtx', () => {
  const all = [task(1, null), task(2, 1, 20), task(3, 1, 10), task(4, null)];
  it('a parent gets its children in order, no parent or siblings', () => {
    const c = statusCtx(all[0], all);
    expect([ids(c.children), ids(c.siblings), c.parent]).toEqual([[3, 2], [], null]);
  });
  it('a subtask gets its parent and siblings (itself included)', () => {
    const c = statusCtx(all[1], all);
    expect([ids(c.children), ids(c.siblings), c.parent?.id]).toEqual([[], [3, 2], 1]);
  });
  it('a subtask whose parent is not loaded has no parent', () => {
    expect(statusCtx(all[1], [all[1]]).parent).toBeNull();
  });
});

describe('nestGroups (Tasks list)', () => {
  // 1 ← 2, 3 ; 4 ; 5 ← 6
  const all = [task(1, null), task(2, 1, 10), task(3, 1, 20), task(4, null), task(5, null), task(6, 5)];
  const rows = (gs: { key: string; rows: { parent: T; dimmed: boolean; children: T[] }[] }[]) => gs.map((g) => [g.key, g.rows.map((r) => [r.parent.id, r.dimmed, ids(r.children)])]);
  it('a shown parent takes its shown children, whatever group they fell in', () => {
    const gs = nestGroups([{ key: 'a', tasks: [all[0], all[2]] }, { key: 'b', tasks: [all[1], all[3]] }], all);
    expect(rows(gs)).toEqual([['a', [[1, false, [2, 3]]]], ['b', [[4, false, []]]]]);
  });
  it('a subtask without its shown parent sits under a dimmed parent in its own group', () => {
    const gs = nestGroups([{ key: 'a', tasks: [all[3]] }, { key: 'b', tasks: [all[5]] }], all);
    expect(rows(gs)).toEqual([['a', [[4, false, []]]], ['b', [[5, true, [6]]]]]);
  });
  it('a group left with no rows is dropped', () => {
    const gs = nestGroups([{ key: 'a', tasks: [all[0]] }, { key: 'b', tasks: [all[1]] }], all);
    expect(rows(gs)).toEqual([['a', [[1, false, [2]]]]]);
  });
});

describe('flattenRows', () => {
  const all = [task(1, null), task(2, 1, 10), task(3, 1, 20), task(4, null)];
  const groups = groupWithParents(all, all);
  it('lists each parent, then its children unless collapsed', () => {
    expect(flattenRows(groups, new Set()).map((r) => [r.task.id, r.sub])).toEqual([[1, false], [2, true], [3, true], [4, false]]);
  });
  it('a collapsed parent keeps its header row and its child count', () => {
    const rows = flattenRows(groups, new Set([1]));
    expect(rows.map((r) => [r.task.id, r.sub, r.kids])).toEqual([[1, false, 2], [4, false, 0]]);
  });
  it('keeps dimmed parents dimmed', () => {
    const rows = flattenRows(groupWithParents([all[1]], all), new Set());
    expect(rows.map((r) => [r.task.id, r.dimmed])).toEqual([[1, true], [2, false]]);
  });
});

import { completedBySave } from './subtasks';
describe('completedBySave', () => {
  it('true only when a subtask went from open to done', () => {
    expect(completedBySave({ status: 'todo', parentId: 1 }, { status: 'done', parentId: 1 })).toBe(true);
    expect(completedBySave({ status: 'inProgress', parentId: 1 }, { status: 'done', parentId: 1 })).toBe(true);
    expect(completedBySave({ status: 'done', parentId: 1 }, { status: 'done', parentId: 1 })).toBe(false);
    expect(completedBySave({ status: 'todo', parentId: 1 }, { status: 'inProgress', parentId: 1 })).toBe(false);
    expect(completedBySave({ status: 'todo', parentId: null }, { status: 'done', parentId: null })).toBe(false);
  });
});
