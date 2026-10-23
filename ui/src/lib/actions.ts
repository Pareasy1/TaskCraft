// User actions shared by several views.
import { api } from './api';
import { taskInput } from './edit';
import { STATUS_LABEL } from './tasks';
import { notify } from './toast.svelte';
import { requestComplete } from './complete.svelte';
import { completeUndo, completionAsk, offerParentAfter } from './subtasks';
import type { Priority, Task, TaskStatus } from './types';
import { t } from './i18n';

/** A task's subtasks, its siblings (itself included) and its parent; see `statusCtx`. */
export interface StatusCtx { children: Task[]; siblings: Task[]; parent: Task | null }

/**
 * Changes a task's status and offers Undo (which restores the task as it was, due date included).
 * With `ctx`, completing a parent with open subtasks asks first, and completing the last open
 * subtask offers to complete its parent.
 */
export async function changeStatus(task: Task, s: TaskStatus, ctx?: StatusCtx): Promise<void> {
  const before = taskInput(task);
  if (s === 'done' && ctx && completionAsk(task, ctx.children) === 'askChildren') {
    const open = ctx.children.filter((c) => !c.completedAt);
    const choice = await requestComplete(task, open.length);
    if (choice === 'cancel') return;
    if (choice === 'all') {
      await api.completeWithSubtasks(task.id);
      const calls = completeUndo(task.id, before, open.map((c) => ({ id: c.id, input: taskInput(c) })));
      notify(t('msg.completed', { title: task.title }), async () => {
        for (const c of calls) await api.updateTask(c.id, c.input);
      });
      return;
    }
  }
  await api.setTaskStatus(task.id, s);
  const undo = () => api.updateTask(task.id, before);
  const parent = ctx?.parent;
  if (s === 'done' && ctx && parent && offerParentAfter(task, ctx.siblings, parent)) {
    // Its subtasks are all done now, so completing it asks nothing.
    notify(t('complete.allDone', { title: parent.title }), undo, {
      label: t('complete.parent'),
      run: () => changeStatus(parent, 'done'),
    });
    return;
  }
  notify(s === 'done' ? t('msg.completed', { title: task.title }) : t('msg.movedTo', { title: task.title, where: STATUS_LABEL[s] }), undo);
}

/** Sets a task's priority, with Undo. */
export async function changePriority(task: Task, p: Priority): Promise<void> {
  if (task.priority === p) return;
  const before = taskInput(task);
  await api.updateTask(task.id, { ...before, priority: p });
  notify(t('msg.movedTo', { title: task.title, where: t(`priority.${p}`) }), () => api.updateTask(task.id, before));
}
