import { describe, expect, it } from 'vitest';
import {
  checklistLabel, childrenByParent, completionAsk, groupWithParents, offerParentAfter, priorityColumns, progressLabel, topLevel,
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
});

describe('labels', () => {
  it('progressLabel', () => {
    expect(progressLabel({ subtaskOpen: 0, subtaskTotal: 0 })).toBeNull();
    expect(progressLabel({ subtaskOpen: 2, subtaskTotal: 4 })).toBe('2 of 4');
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
