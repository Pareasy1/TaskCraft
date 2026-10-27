import { describe, expect, it } from 'vitest';
import { formToTask, taskInput, taskToForm } from './edit';
import type { TaskInput } from './types';

describe('task form', () => {
  it('round-trips status and labels so saving never drops them', () => {
    const input: TaskInput = {
      calendarId: 1, title: 'Renew passport', due: null, dueHasTime: false, tz: 'Europe/Rome',
      rrule: null, notes: null, alerts: [], status: 'inProgress', labels: ['errands', 'Home'],
    };
    const out = formToTask(taskToForm(input));
    expect(out.status).toBe('inProgress');
    expect(out.labels).toEqual(['errands', 'Home']);
  });

  it('round-trips plan and bucket', () => {
    const input: TaskInput = {
      calendarId: 1, title: 'Print stand', due: null, dueHasTime: false, tz: 'Europe/Rome',
      rrule: null, notes: null, alerts: [], status: 'todo', labels: [], planId: 2, bucketId: 5,
    };
    const out = formToTask(taskToForm(input));
    expect(out.planId).toBe(2);
    expect(out.bucketId).toBe(5);
  });
});

import { mergeTaskSave } from './edit';

describe('mergeTaskSave', () => {
  const original: TaskInput = {
    calendarId: 1, title: 'Standup notes', due: '2026-10-20T07:00:00.000Z', dueHasTime: true, tz: 'Europe/Rome',
    rrule: 'FREQ=WEEKLY;INTERVAL=1', notes: null, alerts: [], status: 'todo', labels: ['work'],
  };
  // Meanwhile the card was moved on the board: status changed and the series advanced.
  const latest: TaskInput = { ...original, status: 'inProgress', due: '2026-10-27T08:00:00.000Z' };

  it('keeps changes made elsewhere for fields the user did not edit', () => {
    const edited = formToTask({ ...taskToForm(original), notes: 'bring numbers' });
    const out = mergeTaskSave(original, edited, latest);
    expect(out.notes).toBe('bring numbers');
    expect(out.status).toBe('inProgress');
    expect(out.due).toBe('2026-10-27T08:00:00.000Z');
    expect(out.labels).toEqual(['work']);
  });

  it('the user’s own edits win', () => {
    const edited = formToTask({ ...taskToForm(original), status: 'done', labels: ['work', 'urgent'] });
    const out = mergeTaskSave(original, edited, latest);
    expect(out.status).toBe('done');
    expect(out.labels).toEqual(['work', 'urgent']);
  });

  it('plan and bucket move together', () => {
    const placed = { ...original, planId: 1, bucketId: 1 };
    // The card was dragged to another plan's bucket while the dialog was open.
    const moved = { ...placed, planId: 2, bucketId: 4 };
    expect(mergeTaskSave(placed, formToTask(taskToForm(placed)), moved)).toMatchObject({ planId: 2, bucketId: 4 });
    // The user picked another bucket in the same plan: their pair wins as a whole.
    const edited = formToTask({ ...taskToForm(placed), bucketId: 2 });
    expect(mergeTaskSave(placed, edited, moved)).toMatchObject({ planId: 1, bucketId: 2 });
  });
});

import type { Task } from './types';

describe('taskInput', () => {
  it('turns a stored task back into a saveable input', () => {
    const t: Task = {
      id: 7, calendarId: 2, title: 'Pay rent', due: '2026-10-23T07:00:00.000Z', dueHasTime: true, tz: 'Europe/Rome',
      completedAt: null, recurring: false, rrule: null, notes: 'n', color: '#fff', alerts: [], status: 'inProgress',
      labels: [{ id: 1, name: 'home', color: '#000' }], planId: 3, bucketId: 8, sortOrder: 20,
      parentId: null, priority: 'medium', start: null, startHasTime: false,
      subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0, attachmentCount: 0, blockedBy: 0,
    };
    expect(taskInput(t)).toEqual({
      calendarId: 2, title: 'Pay rent', due: '2026-10-23T07:00:00.000Z', dueHasTime: true, tz: 'Europe/Rome',
      rrule: null, notes: 'n', alerts: [], status: 'inProgress', labels: ['home'], planId: 3, bucketId: 8,
      parentId: null, priority: 'medium', start: null, startHasTime: false,
    });
  });
});

describe('subtask fields', () => {
  const t: Task = {
    id: 8, calendarId: 1, title: 'Sub', due: null, dueHasTime: false, tz: 'Europe/Rome',
    completedAt: null, recurring: false, rrule: null, notes: null, color: '#fff', alerts: [], status: 'todo',
    labels: [], planId: 1, bucketId: 1, sortOrder: 10,
    parentId: 3, priority: 'urgent', start: '2026-10-20T07:00:00.000Z', startHasTime: true,
    subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0, attachmentCount: 0, blockedBy: 0,
  };
  it('taskInput carries priority, start and parent', () => {
    expect(taskInput(t)).toMatchObject({ parentId: 3, priority: 'urgent', start: '2026-10-20T07:00:00.000Z', startHasTime: true });
  });
  it('mergeTaskSave keeps the latest parent when the dialog didn’t change it', () => {
    const original = taskInput(t);
    const edited = formToTask({ ...taskToForm(original), notes: 'x' });
    const latest = { ...original, parentId: 9, priority: 'low' as const, start: null };
    expect(mergeTaskSave(original, edited, latest)).toMatchObject({ notes: 'x', parentId: 9, priority: 'low', start: null });
  });
});

import { startAfterDue } from './edit';

describe('start and priority in the form', () => {
  const base: TaskInput = {
    calendarId: 1, title: 'Ship', due: '2026-10-28T15:30:00.000Z', dueHasTime: true, tz: 'Europe/Rome',
    rrule: null, notes: null, alerts: [], status: 'todo', labels: [], priority: 'important',
    start: '2026-10-24T07:00:00.000Z', startHasTime: true,
  };
  it('splits the start into date and time fields and back', () => {
    const f = taskToForm(base);
    expect(f.startDate).toBe(new Date('2026-10-24T07:00:00.000Z').toLocaleDateString('sv'));
    expect(f.startTime).toMatch(/^\d\d:\d\d$/);
    expect(f.priority).toBe('important');
    const out = formToTask(f);
    expect(new Date(out.start!).getTime()).toBe(new Date('2026-10-24T07:00:00.000Z').getTime());
    expect(out.startHasTime).toBe(true);
    expect(out.priority).toBe('important');
  });
  it('a date-only start is local midnight; no start date means no start', () => {
    const f = { ...taskToForm(base), startDate: '2026-10-24', startTime: '' };
    expect(formToTask(f)).toMatchObject({ start: new Date(2026, 9, 24).toISOString(), startHasTime: false });
    expect(formToTask({ ...f, startDate: '' })).toMatchObject({ start: null, startHasTime: false });
  });
  it('a task without priority edits as Medium', () => {
    const { priority: _p, ...rest } = base;
    void _p;
    expect(taskToForm(rest).priority).toBe('medium');
  });
  it('startAfterDue compares instants only when both are set', () => {
    expect(startAfterDue({ start: '2026-10-29T00:00:00.000Z', due: '2026-10-28T15:30:00.000Z' })).toBe(true);
    expect(startAfterDue({ start: '2026-10-28T15:30:00.000Z', due: '2026-10-28T15:30:00.000Z' })).toBe(false);
    expect(startAfterDue({ start: '2026-10-29T00:00:00.000Z', due: null })).toBe(false);
    expect(startAfterDue({ start: null, due: '2026-10-28T15:30:00.000Z' })).toBe(false);
  });
});

import { taskFormDirty } from './edit';

describe('taskFormDirty', () => {
  const original: TaskInput = {
    calendarId: 1, title: 'Ship', due: '2026-10-28T15:30:00Z', dueHasTime: true, tz: 'Europe/Rome',
    rrule: null, notes: null, alerts: [], status: 'todo', labels: ['a'], priority: 'low', parentId: 4,
    start: '2026-10-24T22:00:00Z', startHasTime: false, planId: 1, bucketId: 2,
  };
  it('an untouched form is clean, whatever the stored formats', () => {
    expect(taskFormDirty(original, taskToForm(original), 4)).toBe(false);
  });
  it('any edited field, or another parent, makes it dirty', () => {
    expect(taskFormDirty(original, { ...taskToForm(original), notes: 'x' }, 4)).toBe(true);
    expect(taskFormDirty(original, { ...taskToForm(original), priority: 'urgent' }, 4)).toBe(true);
    expect(taskFormDirty(original, { ...taskToForm(original), startDate: '' }, 4)).toBe(true);
    expect(taskFormDirty(original, taskToForm(original), null)).toBe(true);
  });
  it('trailing spaces in the title are not an edit', () => {
    expect(taskFormDirty(original, { ...taskToForm(original), title: 'Ship  ' }, 4)).toBe(false);
  });
});
