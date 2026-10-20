import { describe, expect, it } from 'vitest';
import { formToTask, taskToForm } from './edit';
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
});
