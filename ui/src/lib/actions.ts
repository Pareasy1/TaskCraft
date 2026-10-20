// User actions shared by several views.
import { api } from './api';
import { taskInput } from './edit';
import { STATUS_LABEL } from './tasks';
import { notify } from './toast.svelte';
import type { Task, TaskStatus } from './types';
import { t } from './i18n';

/** Changes a task's status and offers Undo (which restores the task as it was, due date included). */
export async function changeStatus(task: Task, s: TaskStatus): Promise<void> {
  const before = taskInput(task);
  await api.setTaskStatus(task.id, s);
  notify(
    s === 'done' ? t('msg.completed', { title: task.title }) : t('msg.movedTo', { title: task.title, where: STATUS_LABEL[s] }),
    () => api.updateTask(task.id, before),
  );
}
