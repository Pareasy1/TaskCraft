import { beforeEach, describe, expect, it, vi } from 'vitest';

let nextId = 100;
vi.mock('./api', () => ({
  api: {
    getTask: vi.fn(),
    listSubtasks: vi.fn(),
    listChecklist: vi.fn(),
    createTask: vi.fn(async () => nextId++),
    addChecklistItem: vi.fn(async () => nextId++),
    updateChecklistItem: vi.fn(async () => {}),
    reorderSubtasks: vi.fn(async () => {}),
  },
  errorMessage: (e: unknown) => String(e),
}));

import { api } from './api';
import { restoreSubtask, restoreWithSubtasks, restoredOrder, snapshotSubtask, snapshotWithSubtasks } from './restore';
import type { ChecklistItem, Task } from './types';

const mk = (id: number, parentId: number | null, title = `T${id}`): Task => ({
  id, calendarId: 1, title, due: null, dueHasTime: false, tz: 'UTC', completedAt: null,
  recurring: false, rrule: null, notes: null, color: '#000', alerts: [], status: 'todo', labels: [],
  planId: 1, bucketId: 1, sortOrder: id, parentId, priority: 'medium', start: null, startHasTime: false,
  subtaskOpen: 0, subtaskTotal: 0, checklistDone: 0, checklistTotal: 0,
});
const item = (id: number, taskId: number, text: string, done: boolean): ChecklistItem => ({ id, taskId, text, done, sort: id });
const lists: Record<number, ChecklistItem[]> = {
  1: [item(1, 1, 'parent a', false)],
  2: [item(2, 2, 'sub a', true), item(3, 2, 'sub b', false)],
  3: [],
};

/** Every api call in order, as [name, ...args]. */
const calls = () =>
  Object.entries(api)
    .flatMap(([name, fn]) => vi.mocked(fn as (...a: unknown[]) => unknown).mock.calls.map((args, i) => ({
      name, args, order: vi.mocked(fn as (...a: unknown[]) => unknown).mock.invocationCallOrder[i],
    })))
    .sort((a, b) => a.order - b.order)
    .map(({ name, args }) => [name, ...args]);

beforeEach(() => {
  vi.clearAllMocks();
  nextId = 100;
  vi.mocked(api.getTask).mockImplementation(async (id: number) => mk(id, id === 1 ? null : 1));
  vi.mocked(api.listSubtasks).mockImplementation(async (id: number) => (id === 1 ? [mk(2, 1), mk(3, 1)] : []));
  vi.mocked(api.listChecklist).mockImplementation(async (id: number) => lists[id] ?? []);
});

describe('restoredOrder', () => {
  it('puts the new id where the old one was', () => {
    expect(restoredOrder([2, 3, 4], 3, 9, [2, 4, 9])).toEqual([2, 9, 4]);
  });
  it('drops siblings that are gone and appends ones added since', () => {
    expect(restoredOrder([2, 3, 4], 2, 9, [4, 5, 9])).toEqual([9, 4, 5]);
  });
});

describe('parent delete undo', () => {
  it('recreates the parent, its checklist, each subtask and each subtask checklist with done state', async () => {
    const snap = await snapshotWithSubtasks(1);
    vi.clearAllMocks();
    const id = await restoreWithSubtasks(snap);
    expect(id).toBe(100);
    expect(calls()).toEqual([
      ['createTask', expect.objectContaining({ title: 'T1', parentId: null })],
      ['addChecklistItem', 100, 'parent a'],
      ['createTask', expect.objectContaining({ title: 'T2', parentId: 100 })],
      ['addChecklistItem', 102, 'sub a'],
      ['updateChecklistItem', 103, { done: true }],
      ['addChecklistItem', 102, 'sub b'],
      ['createTask', expect.objectContaining({ title: 'T3', parentId: 100 })],
    ]);
  });
});

describe('subtask delete undo', () => {
  it('recreates the subtask with its checklist and puts it back in its place', async () => {
    const snap = await snapshotSubtask(2, [2, 3]);
    vi.clearAllMocks();
    vi.mocked(api.listSubtasks).mockImplementation(async () => [mk(3, 1), mk(100, 1)]);
    await restoreSubtask(snap);
    expect(calls()).toEqual([
      ['createTask', expect.objectContaining({ title: 'T2', parentId: 1 })],
      ['addChecklistItem', 100, 'sub a'],
      ['updateChecklistItem', 101, { done: true }],
      ['addChecklistItem', 100, 'sub b'],
      ['listSubtasks', 1],
      ['reorderSubtasks', 1, [100, 3]],
    ]);
  });
  it('skips the reorder when the order is already right', async () => {
    const snap = await snapshotSubtask(3, [2, 3]);
    vi.clearAllMocks();
    vi.mocked(api.listSubtasks).mockImplementation(async () => [mk(2, 1), mk(100, 1)]);
    await restoreSubtask(snap);
    expect(vi.mocked(api.reorderSubtasks)).not.toHaveBeenCalled();
  });
});
