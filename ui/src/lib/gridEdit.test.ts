import { describe, expect, it } from 'vitest';
import { bucketMove, bucketUndo, dateCommitAction, editInput } from './gridEdit';
import type { Task } from './types';

const mk = (id: number, extra: Partial<Task> = {}): Task => ({
  id, calendarId: 1, title: `T${id}`, due: null, dueHasTime: false, tz: 'Europe/Rome', completedAt: null,
  recurring: false, rrule: null, notes: 'n', color: '#000', alerts: [{ type: 'before', value: 15 }], status: 'todo',
  labels: [{ id: 1, name: 'home', color: '#0f0' }], planId: 3, bucketId: 7, sortOrder: id, parentId: null, priority: 'medium',
  start: null, startHasTime: false, subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0,
  attachmentCount: 2, ...extra,
});

describe('editInput', () => {
  it('gridEditUsesTaskInput: a priority edit keeps start, parentId, labels, alerts, plan and bucket', () => {
    const start = new Date(2026, 9, 20, 9, 30).toISOString();
    const x = mk(1, { start, startHasTime: true, parentId: 9, priority: 'low', due: new Date(2026, 9, 25).toISOString() });
    const i = editInput(x, 'priority', 'urgent');
    expect(i.priority).toBe('urgent');
    expect(i).toMatchObject({
      start, startHasTime: true, parentId: 9, labels: ['home'], alerts: [{ type: 'before', value: 15 }], planId: 3, bucketId: 7,
      notes: 'n', due: x.due, title: 'T1', status: 'todo',
    });
  });
  it('a title edit trims and keeps the rest', () => {
    const i = editInput(mk(1, { priority: 'important' }), 'title', '  New name ');
    expect(i.title).toBe('New name');
    expect(i.priority).toBe('important');
  });
  it('a due edit keeps the time of day when the task had one', () => {
    const x = mk(1, { due: new Date(2026, 9, 20, 18, 45).toISOString(), dueHasTime: true });
    const i = editInput(x, 'due', '2026-11-02');
    const d = new Date(i.due!);
    expect([d.getFullYear(), d.getMonth(), d.getDate(), d.getHours(), d.getMinutes()]).toEqual([2026, 10, 2, 18, 45]);
    expect(i.dueHasTime).toBe(true);
  });
  it('a due edit without a time gives local midnight; empty clears', () => {
    const i = editInput(mk(1), 'due', '2026-03-29');
    const d = new Date(i.due!);
    expect([d.getDate(), d.getHours(), d.getMinutes()]).toEqual([29, 0, 0]);
    expect(i.dueHasTime).toBe(false);
    const c = editInput(mk(1, { due: i.due, dueHasTime: true }), 'due', '');
    expect(c.due).toBeNull();
    expect(c.dueHasTime).toBe(false);
  });
  it('a start edit keeps its time and leaves due alone', () => {
    const due = new Date(2026, 9, 30).toISOString();
    const x = mk(1, { start: new Date(2026, 9, 20, 8, 15).toISOString(), startHasTime: true, due });
    const i = editInput(x, 'start', '2026-10-22');
    const s = new Date(i.start!);
    expect([s.getDate(), s.getHours(), s.getMinutes()]).toEqual([22, 8, 15]);
    expect(i.startHasTime).toBe(true);
    expect(i.due).toBe(due);
    expect(editInput(x, 'start', '').start).toBeNull();
  });
});

describe('bucketMove', () => {
  const col = [mk(1, { bucketId: 8, sortOrder: 5 }), mk(2, { bucketId: 8, sortOrder: 9 }), mk(3, { bucketId: 8, sortOrder: 1 }), mk(4, { bucketId: 8, sortOrder: 20, parentId: 2 })];
  it('appends after the last top-level task of the bucket', () => {
    expect(bucketMove(mk(9), 8, col)).toEqual({ bucketId: 8, before: 2, after: null });
  });
  it('an empty bucket has no neighbour', () => {
    expect(bucketMove(mk(9), 8, [])).toEqual({ bucketId: 8, before: null, after: null });
  });
  it('refuses subtasks and a move to the same bucket', () => {
    expect(bucketMove(mk(9, { parentId: 1 }), 8, col)).toBeNull();
    expect(bucketMove(mk(9, { bucketId: 8 }), 8, col)).toBeNull();
  });
  it('bucketUndo puts the task back between its old neighbours', () => {
    const old = [mk(1, { sortOrder: 1 }), mk(2, { sortOrder: 2 }), mk(3, { sortOrder: 3 }), mk(5, { sortOrder: 2.5, parentId: 1 })];
    expect(bucketUndo(old[1], old)).toEqual({ bucketId: 7, before: 1, after: 3 });
    expect(bucketUndo(old[0], old)).toEqual({ bucketId: 7, before: null, after: 2 });
  });
});

describe('dateCommitAction', () => {
  it('a half-typed date is an error on Enter and a cancel on blur, never a clear', () => {
    expect(dateCommitAction('', true, '2026-10-20', 'enter')).toBe('error');
    expect(dateCommitAction('', true, '2026-10-20', 'blur')).toBe('cancel');
    expect(dateCommitAction('', true, '', 'enter')).toBe('error');
  });
  it('an unchanged value cancels', () => {
    expect(dateCommitAction('2026-10-20', false, '2026-10-20', 'enter')).toBe('cancel');
    expect(dateCommitAction('', false, '', 'blur')).toBe('cancel');
  });
  it('a new complete date saves', () => {
    expect(dateCommitAction('2026-11-02', false, '2026-10-20', 'enter')).toBe('save');
    expect(dateCommitAction('2026-11-02', false, '', 'blur')).toBe('save');
  });
  it('an emptied field clears only on Enter; blur keeps the stored date', () => {
    expect(dateCommitAction('', false, '2026-10-20', 'enter')).toBe('clear');
    expect(dateCommitAction('', false, '2026-10-20', 'blur')).toBe('cancel');
  });
});
