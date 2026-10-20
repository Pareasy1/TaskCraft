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
