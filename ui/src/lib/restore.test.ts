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
    listAttachments: vi.fn(),
    restoreFileAttachment: vi.fn(async () => ({ id: nextId++ })),
    addLinkAttachment: vi.fn(async () => ({ id: nextId++ })),
    reorderAttachments: vi.fn(async () => {}),
    setTaskStatus: vi.fn(async () => {}),
  },
  errorMessage: (e: unknown) => String(e),
}));

import { api } from './api';
import { dismiss, toast } from './toast.svelte';
import { restoreRemovedAttachment, restoreSubtask, restoreWithSubtasks, restoredOrder, snapshotSubtask, snapshotWithSubtasks } from './restore';
import type { Attachment, ChecklistItem, Task } from './types';

const mk = (id: number, parentId: number | null, title = `T${id}`): Task => ({
  id, calendarId: 1, title, due: null, dueHasTime: false, tz: 'UTC', completedAt: null,
  recurring: false, rrule: null, notes: null, color: '#000', alerts: [], status: 'todo', labels: [],
  planId: 1, bucketId: 1, sortOrder: id, parentId, priority: 'medium', start: null, startHasTime: false,
  subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0, attachmentCount: 0,
});
const item = (id: number, taskId: number, text: string, done: boolean): ChecklistItem => ({ id, taskId, text, done, sort: id });
const file = (id: number, taskId: number, name: string, stored: string, size: number): Attachment =>
  ({ id, taskId, kind: 'file', name, stored, url: null, size, createdAt: '2026-10-24T10:00:00Z', sort: id });
const link = (id: number, taskId: number, name: string, url: string): Attachment =>
  ({ id, taskId, kind: 'link', name, stored: null, url, size: null, createdAt: '2026-10-24T10:00:00Z', sort: id });
const atts: Record<number, Attachment[]> = {
  1: [link(11, 1, 'Spec', 'https://example.com/spec'), file(12, 1, 'brief.pdf', '1/1-brief.pdf', 2048)],
  2: [file(13, 2, 'photo.jpg', '2/1-photo.jpg', 512)],
};
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
  vi.mocked(api.listAttachments).mockImplementation(async (id: number) => atts[id] ?? []);
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
  it('recreates the parent, its checklist and attachments, then each subtask with its checklist and attachments', async () => {
    const snap = await snapshotWithSubtasks(1);
    vi.clearAllMocks();
    const id = await restoreWithSubtasks(snap);
    expect(id).toBe(100);
    expect(calls()).toEqual([
      ['createTask', expect.objectContaining({ title: 'T1', parentId: null })],
      ['addChecklistItem', 100, 'parent a'],
      // Attachments in their original order: links re-added, files re-linked to the stored copy.
      ['addLinkAttachment', 100, 'https://example.com/spec', 'Spec'],
      ['restoreFileAttachment', 100, 'brief.pdf', '1/1-brief.pdf', 2048],
      ['createTask', expect.objectContaining({ title: 'T2', parentId: 100 })],
      ['addChecklistItem', 104, 'sub a'],
      ['updateChecklistItem', 105, { done: true }],
      ['addChecklistItem', 104, 'sub b'],
      ['restoreFileAttachment', 104, 'photo.jpg', '2/1-photo.jpg', 512],
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
      ['restoreFileAttachment', 100, 'photo.jpg', '2/1-photo.jpg', 512],
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

describe('attachment remove undo', () => {
  it('re-links a removed file and puts it back in its place', async () => {
    const removed = file(12, 1, 'brief.pdf', '1/1-brief.pdf', 2048);
    vi.mocked(api.listAttachments).mockImplementation(async () => [link(11, 1, 'Spec', 'https://x.org'), file(14, 1, 'c.txt', 'c', 1), file(100, 1, 'brief.pdf', '1/1-brief.pdf', 2048)]);
    await restoreRemovedAttachment(removed, [11, 12, 14]);
    expect(calls()).toEqual([
      ['restoreFileAttachment', 1, 'brief.pdf', '1/1-brief.pdf', 2048],
      ['listAttachments', 1],
      ['reorderAttachments', 1, [11, 100, 14]],
    ]);
  });
  it('adds a removed link again and skips the reorder when it was last', async () => {
    const removed = link(11, 1, 'Spec', 'https://example.com/spec');
    vi.mocked(api.listAttachments).mockImplementation(async () => [file(12, 1, 'b', 'b', 1), link(100, 1, 'Spec', 'https://example.com/spec')]);
    await restoreRemovedAttachment(removed, [12, 11]);
    expect(calls()).toEqual([
      ['addLinkAttachment', 1, 'https://example.com/spec', 'Spec'],
      ['listAttachments', 1],
    ]);
  });
});

describe('undo never auto-starts the parent', () => {
  const withStatus = (statuses: Record<number, Task['status']>) =>
    vi.mocked(api.getTask).mockImplementation(async (id: number) => ({ ...mk(id, id === 1 ? null : 1), status: statuses[id] ?? 'todo' }));

  it('a To do parent with an In progress subtask is put back to To do last', async () => {
    withStatus({ 2: 'inProgress' });
    const snap = await snapshotWithSubtasks(1);
    vi.clearAllMocks();
    await restoreWithSubtasks(snap);
    const all = calls();
    expect(all.at(-1)).toEqual(['setTaskStatus', 100, 'todo']);
    expect(all.filter((c) => c[0] === 'setTaskStatus')).toHaveLength(1);
  });

  it('leaves a parent alone when nothing could have started it', async () => {
    withStatus({ 1: 'inProgress', 2: 'inProgress' });
    const snap = await snapshotWithSubtasks(1);
    vi.clearAllMocks();
    await restoreWithSubtasks(snap);
    expect(api.setTaskStatus).not.toHaveBeenCalled();
  });

  it('a restored In progress subtask puts its To do parent back to To do', async () => {
    withStatus({ 2: 'inProgress' });
    const snap = await snapshotSubtask(2, [2, 3]);
    vi.clearAllMocks();
    vi.mocked(api.listSubtasks).mockImplementation(async () => [mk(3, 1), mk(100, 1)]);
    await restoreSubtask(snap);
    const all = calls();
    expect(all[0]).toEqual(['getTask', 1]);
    expect(all.at(-1)).toEqual(['setTaskStatus', 1, 'todo']);
  });

  it('a restored In progress subtask under an In progress parent changes nothing', async () => {
    withStatus({ 1: 'inProgress', 2: 'inProgress' });
    const snap = await snapshotSubtask(2, [2, 3]);
    vi.clearAllMocks();
    vi.mocked(api.listSubtasks).mockImplementation(async () => [mk(3, 1), mk(100, 1)]);
    await restoreSubtask(snap);
    expect(api.setTaskStatus).not.toHaveBeenCalled();
  });
});

describe('undo with attachments that are gone', () => {
  it('skips a purged file, keeps going, and says which ones are missing once', async () => {
    const snap = await snapshotWithSubtasks(1);
    vi.clearAllMocks();
    vi.mocked(api.restoreFileAttachment).mockRejectedValue(new Error('not_found'));
    try {
      await restoreWithSubtasks(snap);
      const names = calls().map((c) => c[0]);
      // Every subtask still came back.
      expect(names.filter((n) => n === 'createTask')).toHaveLength(3);
      expect(api.addLinkAttachment).toHaveBeenCalledTimes(1);
      expect(api.restoreFileAttachment).toHaveBeenCalledTimes(2);
      expect(toast.current?.message).toBe('Restored without brief.pdf, photo.jpg.');
    } finally {
      vi.mocked(api.restoreFileAttachment).mockReset().mockImplementation(async () => ({ id: nextId++ }) as Attachment);
      dismiss();
    }
  });

  it('a restored subtask also skips a purged file and says so', async () => {
    const snap = await snapshotSubtask(2, [2, 3]);
    vi.clearAllMocks();
    vi.mocked(api.listSubtasks).mockImplementation(async () => [mk(3, 1), mk(100, 1)]);
    vi.mocked(api.restoreFileAttachment).mockRejectedValue(new Error('not_found'));
    try {
      await restoreSubtask(snap);
      expect(api.reorderSubtasks).toHaveBeenCalledWith(1, [100, 3]);
      expect(toast.current?.message).toBe('Restored without photo.jpg.');
    } finally {
      vi.mocked(api.restoreFileAttachment).mockReset().mockImplementation(async () => ({ id: nextId++ }) as Attachment);
      dismiss();
    }
  });

  it('no toast when everything came back', async () => {
    const snap = await snapshotWithSubtasks(1);
    vi.clearAllMocks();
    await restoreWithSubtasks(snap);
    expect(toast.current).toBeNull();
  });
});
