import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  api: {
    setTaskStatus: vi.fn(async () => {}),
    completeWithSubtasks: vi.fn(async () => {}),
    updateTask: vi.fn(async () => {}),
  },
  errorMessage: (e: unknown) => String(e),
}));

import { api } from './api';
import { changeStatus } from './actions';
import { answerComplete, pending } from './complete.svelte';
import { dismiss, runAction, toast, undo } from './toast.svelte';
import type { Task } from './types';

const mk = (id: number, parentId: number | null, done = false, title = `T${id}`): Task => ({
  id, calendarId: 1, title, due: null, dueHasTime: false, tz: 'UTC', completedAt: done ? '2026-10-23T10:00:00Z' : null,
  recurring: false, rrule: null, notes: null, color: '#000', alerts: [], status: done ? 'done' : 'todo', labels: [],
  planId: 1, bucketId: 1, sortOrder: id, parentId, priority: 'medium', start: null, startHasTime: false,
  subtaskOpen: 0, subtaskTotal: 0, checklistDone: 0, checklistTotal: 0,
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
    expect(pending.current?.openChildren).toBe(1);
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

  it('completing the last open subtask offers to complete the parent', async () => {
    await changeStatus(open, 'done', { children: [], siblings: [open, closed], parent });
    expect(toast.current?.message).toBe('All subtasks of Ship Rafael are done');
    expect(toast.current?.action).toBe('Complete it');
    expect(toast.current?.canUndo).toBe(true);
    await runAction();
    expect(api.setTaskStatus).toHaveBeenLastCalledWith(1, 'done');
  });

  it('no offer while another subtask is still open', async () => {
    await changeStatus(open, 'done', { children: [], siblings: [open, mk(4, 1)], parent });
    expect(toast.current?.action).toBeUndefined();
  });
});
