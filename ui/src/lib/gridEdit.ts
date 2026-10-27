// Updates made by the Grid's inline editors. Each builds on `taskInput`, so a cell edit
// never resets the task's other fields.
import { midnight, taskInput } from './edit';
import type { Priority, Task, TaskInput } from './types';

export type EditField = 'title' | 'priority' | 'start' | 'due';
type EditValue<F extends EditField> = F extends 'priority' ? Priority : string;

/** `date` (`YYYY-MM-DD`, '' clears) keeping the local time of `iso` when the task had one. */
function withDate(iso: string | null, hasTime: boolean, date: string): { at: string | null; hasTime: boolean } {
  if (!date) return { at: null, hasTime: false };
  if (!iso || !hasTime) return { at: midnight(date), hasTime: false };
  const old = new Date(iso);
  const [y, m, d] = date.split('-').map(Number);
  return { at: new Date(y, m - 1, d, old.getHours(), old.getMinutes()).toISOString(), hasTime: true };
}

/** The task's input with one field changed. Dates take a `YYYY-MM-DD` value ('' clears). */
export function editInput<F extends EditField>(task: Task, field: F, value: EditValue<F>): TaskInput {
  const i = taskInput(task);
  switch (field) {
    case 'title': return { ...i, title: (value as string).trim() };
    case 'priority': return { ...i, priority: value as Priority };
    case 'start': {
      const s = withDate(task.start, task.startHasTime, value as string);
      return { ...i, start: s.at, startHasTime: s.hasTime };
    }
    default: {
      const d = withDate(task.due, task.dueHasTime, value as string);
      return { ...i, due: d.at, dueHasTime: d.hasTime };
    }
  }
}

export interface BucketPlace { bucketId: number; before: number | null; after: number | null }

const topIn = (tasks: Task[], bucketId: number, skip: number) =>
  tasks.filter((x) => x.bucketId === bucketId && x.parentId == null && x.id !== skip).sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id);

/**
 * Where `api.moveTask` puts a task moved to another bucket: at the end, after the bucket's last
 * top-level task (`before` is the task it follows, as in the store). Null for a subtask (they
 * follow their parent) or when the bucket doesn't change.
 */
export function bucketMove(task: Task, bucketId: number, columnTasks: Task[]): BucketPlace | null {
  if (task.parentId != null || task.bucketId === bucketId) return null;
  return { bucketId, before: topIn(columnTasks, bucketId, task.id).at(-1)?.id ?? null, after: null };
}

/** The place a task held in its bucket (from the tasks before the move), for Undo. */
export function bucketUndo(task: Task, tasks: Task[]): BucketPlace {
  const col = topIn([...tasks.filter((x) => x.id !== task.id), task], task.bucketId, -1);
  const i = col.findIndex((x) => x.id === task.id);
  return { bucketId: task.bucketId, before: col[i - 1]?.id ?? null, after: col[i + 1]?.id ?? null };
}

/**
 * What a date editor does with its field. A half-typed date (`badInput`: the browser reports
 * value '') is never taken as "clear": Enter says it's invalid, leaving the field cancels.
 * An emptied field clears only on Enter (the Clear button is the other way); leaving keeps the date.
 */
export function dateCommitAction(raw: string, badInput: boolean, initial: string, via: 'enter' | 'blur'): 'save' | 'clear' | 'error' | 'cancel' {
  if (badInput) return via === 'enter' ? 'error' : 'cancel';
  if (raw === initial) return 'cancel';
  if (!raw) return via === 'enter' ? 'clear' : 'cancel';
  return 'save';
}
