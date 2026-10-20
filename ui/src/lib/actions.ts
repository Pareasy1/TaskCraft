// User actions shared by several views.
import { api } from './api';
import { taskInput } from './edit';
import { STATUS_LABEL } from './tasks';
import { notify } from './toast.svelte';
import type { Task, TaskStatus } from './types';

/** Changes a task's status and offers Undo (which restores the task as it was, due date included). */
export async function changeStatus(t: Task, s: TaskStatus): Promise<void> {
  const before = taskInput(t);
  await api.setTaskStatus(t.id, s);
  notify(s === 'done' ? `Completed “${t.title}”` : `Moved “${t.title}” to ${STATUS_LABEL[s]}`, () => api.updateTask(t.id, before));
}
