import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  api: {
    setTaskStatus: vi.fn(async () => {}),
    completeWithSubtasks: vi.fn(async () => {}),
    updateTask: vi.fn(async () => {}),
    getTask: vi.fn(),
    parsePreview: vi.fn(async () => ({ plan: null })),
    createFromText: vi.fn(async () => ({ kind: 'task', id: 5 })),
    getDefaultPlan: vi.fn(async () => 1),
    moveTask: vi.fn(async () => {}),
    listSubtasks: vi.fn(),
    listDependencies: vi.fn(async () => []),
  },
  errorMessage: (e: unknown) => String(e),
}));

import { api } from './api';
import { askParentAfterSave, changeStatus, quickAddTask } from './actions';
import { answerComplete, pending } from './complete.svelte';
import { dismiss, runAction, toast, undo } from './toast.svelte';
import type { Task } from './types';

const mk = (id: number, parentId: number | null, done = false, title = `T${id}`, extra: Partial<Task> = {}): Task => ({
  id, calendarId: 1, title, due: null, dueHasTime: false, tz: 'UTC', completedAt: done ? '2026-10-23T10:00:00Z' : null,
  recurring: false, rrule: null, notes: null, color: '#000', alerts: [], status: done ? 'done' : 'todo', labels: [],
  planId: 1, bucketId: 1, sortOrder: id, parentId, priority: 'medium', start: null, startHasTime: false,
  subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0, attachmentCount: 0, blockedBy: 0, ...extra,
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

  it('a To do parent: toast says it is now In progress; Undo restores the subtask and the parent', async () => {
    await changeStatus(sub, 'inProgress', { children: [], siblings: [sub], parent });
    expect(api.setTaskStatus).toHaveBeenCalledWith(2, 'inProgress');
    expect(toast.current?.message).toBe('Ship Rafael is now In progress');
    expect(toast.current?.canUndo).toBe(true);
    await undo();
    expect(api.updateTask).toHaveBeenCalledWith(2, expect.objectContaining({ title: 'T2', status: 'todo' }));
    expect(api.setTaskStatus).toHaveBeenLastCalledWith(1, 'todo');
    // The subtask first, so nothing can start the parent again afterwards.
    expect(vi.mocked(api.updateTask).mock.invocationCallOrder[0])
      .toBeLessThan(vi.mocked(api.setTaskStatus).mock.invocationCallOrder.at(-1)!);
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

describe('askParentAfterSave (a subtask saved as Done from its own form)', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => dismiss());
  const parent = mk(1, null, false, 'Ship Rafael');
  const sub = mk(2, 1, true);

  it('last subtask saved Done: asks; Yes completes the parent', async () => {
    vi.mocked(api.getTask).mockImplementation(async (id: number) => (id === 2 ? sub : parent));
    vi.mocked(api.listSubtasks).mockResolvedValue([sub, mk(3, 1, true)]);
    const p = askParentAfterSave(2, 'todo');
    await tick();
    expect(pending.current).toMatchObject({ kind: 'parent', task: { id: 1 } });
    answerComplete('yes');
    await p;
    expect(api.setTaskStatus).toHaveBeenCalledWith(1, 'done');
  });

  it('No: nothing else happens', async () => {
    vi.mocked(api.getTask).mockImplementation(async (id: number) => (id === 2 ? sub : parent));
    vi.mocked(api.listSubtasks).mockResolvedValue([sub]);
    const p = askParentAfterSave(2, 'inProgress');
    await tick();
    answerComplete('cancel');
    await p;
    expect(api.setTaskStatus).not.toHaveBeenCalled();
  });

  it('no question when it was already done, a sibling is open, or it is not a subtask', async () => {
    vi.mocked(api.getTask).mockImplementation(async (id: number) => (id === 2 ? sub : parent));
    vi.mocked(api.listSubtasks).mockResolvedValue([sub, mk(3, 1)]);
    await askParentAfterSave(2, 'done');
    await askParentAfterSave(2, 'todo');
    vi.mocked(api.getTask).mockResolvedValue(mk(9, null, true));
    await askParentAfterSave(9, 'todo');
    expect(pending.current).toBeNull();
    expect(api.setTaskStatus).not.toHaveBeenCalled();
  });
});

describe('quickAddTask', () => {
  it('reports the task as created before the move into the plan finishes', async () => {
    const order: string[] = [];
    vi.mocked(api.parsePreview).mockResolvedValue({ plan: null } as never);
    vi.mocked(api.createFromText).mockResolvedValue({ kind: 'task', id: 5 } as never);
    vi.mocked(api.getTask).mockResolvedValue(mk(5, null, false, 'T5', { planId: 1 }));
    vi.mocked(api.getDefaultPlan).mockResolvedValue(1 as never);
    vi.mocked(api.moveTask).mockImplementation(async () => { order.push('move'); throw new Error('move failed'); });
    const plan = { id: 2, name: 'P', color: '#000', sort: 0, openCount: 0 };
    const bucket = { id: 20, planId: 2, name: 'b', sort: 0 };
    await expect(quickAddTask('x', plan, bucket, [], () => order.push('created'))).rejects.toThrow('move failed');
    expect(order).toEqual(['created', 'move']);
  });
});

describe('changeStatus on blocked tasks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listDependencies).mockResolvedValue([{ taskId: 1, dependsOn: 9 }]);
    vi.mocked(api.getTask).mockImplementation(async (id: number) => mk(id, null, false, `Blocker ${id}`));
  });
  afterEach(() => dismiss());
  const blocked = mk(1, null, false, 'Blocked', { blockedBy: 1 });

  it('blocked task asks before starting; No changes nothing', async () => {
    const p = changeStatus(blocked, 'inProgress');
    await tick();
    expect(pending.current).toMatchObject({ kind: 'blocked', message: 'Depends on “Blocker 9”, still open. Continue?' });
    answerComplete('cancel');
    await p;
    expect(api.setTaskStatus).not.toHaveBeenCalled();
  });

  it('Yes proceeds', async () => {
    const p = changeStatus(blocked, 'done');
    await tick();
    answerComplete('yes');
    await p;
    expect(api.setTaskStatus).toHaveBeenCalledWith(1, 'done');
  });

  it('unblocked task does not ask', async () => {
    await changeStatus(mk(1, null), 'inProgress');
    expect(pending.current).toBeNull();
    expect(api.setTaskStatus).toHaveBeenCalledWith(1, 'inProgress');
  });

  it('moving a blocked task to todo does not ask', async () => {
    await changeStatus({ ...blocked, status: 'inProgress' }, 'todo');
    expect(pending.current).toBeNull();
    expect(api.setTaskStatus).toHaveBeenCalledWith(1, 'todo');
  });

  it('subtask start auto-starting a blocked parent does not ask for the parent', async () => {
    const parent = mk(1, null, false, 'P', { blockedBy: 1 });
    await changeStatus(mk(2, 1), 'inProgress', { children: [], siblings: [], parent });
    expect(pending.current).toBeNull();
    expect(api.setTaskStatus).toHaveBeenCalledWith(2, 'inProgress');
  });
});
