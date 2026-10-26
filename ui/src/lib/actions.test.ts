import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  api: {
    setTaskStatus: vi.fn(async () => {}),
    completeWithSubtasks: vi.fn(async () => {}),
    updateTask: vi.fn(async () => {}),
    getTask: vi.fn(),
  },
  errorMessage: (e: unknown) => String(e),
}));

import { api } from './api';
import { changeStatus } from './actions';
import { answerComplete, pending } from './complete.svelte';
import { dismiss, runAction, toast, undo } from './toast.svelte';
import type { Task } from './types';

const mk = (id: number, parentId: number | null, done = false, title = `T${id}`, extra: Partial<Task> = {}): Task => ({
  id, calendarId: 1, title, due: null, dueHasTime: false, tz: 'UTC', completedAt: done ? '2026-10-23T10:00:00Z' : null,
  recurring: false, rrule: null, notes: null, color: '#000', alerts: [], status: done ? 'done' : 'todo', labels: [],
  planId: 1, bucketId: 1, sortOrder: id, parentId, priority: 'medium', start: null, startHasTime: false,
  subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0, attachmentCount: 0, ...extra,
});
const tick = () => new Promise((r) => setTimeout(r, 0));

describe('changeStatus with subtasks', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => dismiss());
  const parent = mk(1, null, false, 'Ship Rafael');
  const open = mk(2, 1);
  const closed = mk(3, 1, true);

  it('without context it just sets the status (old callers)', async () => {
    await changeStatus(parent, 'done');
    expect(api.setTaskStatus).toHaveBeenCalledWith(1, 'done');
  });

  it('asks before completing a parent with open subtasks; "all" completes them, Undo restores both', async () => {
    const p = changeStatus(parent, 'done', { children: [open, closed], siblings: [], parent: null });
    await tick();
    expect(pending.current).toMatchObject({ kind: 'children', openChildren: 1 });
    answerComplete('all');
    await p;
    expect(api.completeWithSubtasks).toHaveBeenCalledWith(1);
    expect(api.setTaskStatus).not.toHaveBeenCalled();
    await undo();
    expect(vi.mocked(api.updateTask).mock.calls.map((c) => c[0])).toEqual([2, 1]);
  });

  it('"only" completes just the parent; "cancel" does nothing', async () => {
    let p = changeStatus(parent, 'done', { children: [open], siblings: [], parent: null });
    await tick();
    answerComplete('only');
    await p;
    expect(api.setTaskStatus).toHaveBeenCalledWith(1, 'done');
    vi.clearAllMocks();
    p = changeStatus(parent, 'done', { children: [open], siblings: [], parent: null });
    await tick();
    answerComplete('cancel');
    await p;
    expect(api.setTaskStatus).not.toHaveBeenCalled();
    expect(api.completeWithSubtasks).not.toHaveBeenCalled();
  });

  it('completing the last open subtask asks to complete the parent; Yes completes it', async () => {
    const p = changeStatus(open, 'done', { children: [], siblings: [open, closed], parent });
    await tick();
    expect(api.setTaskStatus).toHaveBeenCalledWith(2, 'done');
    expect(pending.current).toMatchObject({ kind: 'parent', task: { id: 1 } });
    // The subtask's own toast still offers Undo meanwhile.
    expect(toast.current?.message).toBe('Completed “T2”');
    expect(toast.current?.canUndo).toBe(true);
    answerComplete('yes');
    await p;
    expect(api.setTaskStatus).toHaveBeenLastCalledWith(1, 'done');
    expect(toast.current?.message).toBe('Completed “Ship Rafael”');
    await undo();
    expect(api.updateTask).toHaveBeenLastCalledWith(1, expect.objectContaining({ title: 'Ship Rafael', status: 'todo' }));
  });

  it('Yes on a repeating parent goes through the normal status path (the store advances it)', async () => {
    const rep = mk(5, null, false, 'Weekly', { recurring: true, rrule: 'FREQ=WEEKLY', due: '2026-10-30T10:00:00Z' });
    const sub = mk(6, 5);
    const p = changeStatus(sub, 'done', { children: [], siblings: [sub], parent: rep });
    await tick();
    answerComplete('yes');
    await p;
    expect(api.setTaskStatus).toHaveBeenLastCalledWith(5, 'done');
  });

  it('No leaves the parent open', async () => {
    const p = changeStatus(open, 'done', { children: [], siblings: [open, closed], parent });
    await tick();
    answerComplete('cancel');
    await p;
    expect(api.setTaskStatus).toHaveBeenCalledTimes(1);
    expect(api.setTaskStatus).toHaveBeenCalledWith(2, 'done');
    expect(pending.current).toBeNull();
  });

  it('a completeParent hook replaces the default completion (details dialog)', async () => {
    const completeParent = vi.fn(async () => {});
    const p = changeStatus(open, 'done', { children: [], siblings: [open, closed], parent }, { completeParent });
    await tick();
    answerComplete('yes');
    await p;
    expect(completeParent).toHaveBeenCalledWith(parent);
    expect(api.setTaskStatus).toHaveBeenCalledTimes(1);
  });

  it('no offer while another subtask is still open', async () => {
    await changeStatus(open, 'done', { children: [], siblings: [open, mk(4, 1)], parent });
    expect(pending.current).toBeNull();
    expect(toast.current?.action).toBeUndefined();
  });
});

describe('changeStatus: a subtask in progress starts its parent', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => dismiss());
  const parent = mk(1, null, false, 'Ship Rafael');
  const sub = mk(2, 1);

  it('a To do parent: toast says it is now In progress; Undo restores only the parent', async () => {
    await changeStatus(sub, 'inProgress', { children: [], siblings: [sub], parent });
    expect(api.setTaskStatus).toHaveBeenCalledWith(2, 'inProgress');
    expect(toast.current?.message).toBe('Ship Rafael is now In progress');
    expect(toast.current?.canUndo).toBe(true);
    await undo();
    expect(api.setTaskStatus).toHaveBeenLastCalledWith(1, 'todo');
    expect(api.updateTask).not.toHaveBeenCalled();
  });

  it('without context it fetches the parent', async () => {
    vi.mocked(api.getTask).mockResolvedValueOnce(parent);
    await changeStatus(sub, 'inProgress');
    expect(api.getTask).toHaveBeenCalledWith(1);
    expect(toast.current?.message).toBe('Ship Rafael is now In progress');
  });

  it('a parent already in progress or done: the usual toast', async () => {
    for (const p of [{ ...parent, status: 'inProgress' as const }, mk(1, null, true, 'Ship Rafael')]) {
      await changeStatus(sub, 'inProgress', { children: [], siblings: [sub], parent: p });
      expect(toast.current?.message).toBe('Moved “T2” to In progress');
    }
  });

  it('a top-level task: the usual toast', async () => {
    await changeStatus(parent, 'inProgress', { children: [sub], siblings: [], parent: null });
    expect(api.getTask).not.toHaveBeenCalled();
    expect(toast.current?.message).toBe('Moved “Ship Rafael” to In progress');
  });
});
