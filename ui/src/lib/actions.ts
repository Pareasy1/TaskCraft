// User actions shared by several views.
import { api } from './api';
import { taskInput } from './edit';
import { STATUS_LABEL } from './tasks';
import { notify } from './toast.svelte';
import { requestBlocked, requestComplete, requestCompleteParent } from './complete.svelte';
import { completedBySave, completeUndo, completionAsk, offerParentAfter } from './subtasks';
import type { Bucket, Plan, Priority, Task, TaskInput, TaskStatus } from './types';
import { lastTopLevelIn, quickAddNeedsMove } from './plans';
import { t, type Key } from './i18n';
import { blockedPrompt } from './dependencies';

/** A task's subtasks, its siblings (itself included) and its parent; see `statusCtx`. */
export interface StatusCtx { children: Task[]; siblings: Task[]; parent: Task | null }

/** How the parent gets completed after "Mark the task as done too?" (default: `changeStatus(parent, 'done')`). */
export interface StatusHooks { completeParent?: (parent: Task) => Promise<void> }

/**
 * Starting or finishing a task that still waits on open tasks asks first (false = No).
 * Anything else (other statuses, no change, nothing open) is allowed without a question.
 */
export async function confirmBlocked(task: Task, next: TaskStatus): Promise<boolean> {
  if ((next !== 'inProgress' && next !== 'done') || next === task.status || task.blockedBy <= 0) return true;
  const deps = await api.listDependencies([task.id]);
  const ids = deps.filter((d) => d.taskId === task.id).map((d) => d.dependsOn);
  const open = (await Promise.all(ids.map((id) => api.getTask(id).catch(() => null)))).filter((x): x is Task => !!x && !x.completedAt && x.status !== 'done');
  if (!open.length) return true;
  return requestBlocked(blockedPrompt(open));
}

/**
 * Changes a task's status and offers Undo (which restores the task as it was, due date included).
 * With `ctx`, completing a parent with open subtasks asks first, and completing the last open
 * subtask asks whether to complete its parent too. A subtask put in progress starts a To do
 * parent (the store does it); the toast then says so and its Undo puts both back.
 */
export async function changeStatus(task: Task, s: TaskStatus, ctx?: StatusCtx, hooks: StatusHooks = {}): Promise<void> {
  if (!(await confirmBlocked(task, s))) return;
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
  const starting = s === 'inProgress' && task.status !== 'inProgress' ? await parentToStart(task, ctx) : null;
  await api.setTaskStatus(task.id, s);
  if (starting) {
    // Undo puts the subtask back as it was, then the parent back to To do.
    notify(t('msg.parentStarted', { title: starting.title }), async () => {
      await api.updateTask(task.id, before);
      await api.setTaskStatus(starting.id, 'todo');
    });
    return;
  }
  notify(s === 'done' ? t('msg.completed', { title: task.title }) : t('msg.movedTo', { title: task.title, where: STATUS_LABEL[s] }), () =>
    api.updateTask(task.id, before),
  );
  const parent = ctx?.parent;
  if (s === 'done' && ctx && parent && offerParentAfter(task, ctx.siblings, parent) && (await requestCompleteParent(parent))) {
    // Its subtasks are all done now, so completing it asks nothing.
    await (hooks.completeParent ?? ((p: Task) => changeStatus(p, 'done')))(parent);
  }
}

/**
 * After a task's details form was saved: when that took a subtask from open to done and it was
 * the last open one, asks whether to complete the parent too (Yes completes it the usual way).
 */
export async function askParentAfterSave(id: number, statusBefore: TaskStatus): Promise<void> {
  const me = await api.getTask(id);
  if (me.parentId == null || !completedBySave({ status: statusBefore, parentId: me.parentId }, me)) return;
  const [parent, siblings] = await Promise.all([api.getTask(me.parentId), api.listSubtasks(me.parentId)]);
  if (offerParentAfter(me, siblings, parent) && (await requestCompleteParent(parent))) await changeStatus(parent, 'done');
}

/** The parent the store will move from To do to In progress when `task` starts, if any. */
async function parentToStart(task: Task, ctx?: StatusCtx): Promise<Task | null> {
  if (task.parentId == null) return null;
  const parent = ctx?.parent ?? (await api.getTask(task.parentId).catch(() => null));
  return parent && parent.status === 'todo' && !parent.completedAt ? parent : null;
}

/** Sets a task's priority, with Undo. */
export async function changePriority(task: Task, p: Priority): Promise<void> {
  if (task.priority === p) return;
  const before = taskInput(task);
  await api.updateTask(task.id, { ...before, priority: p });
  notify(t('msg.movedTo', { title: task.title, where: t(`priority.${p}`) }), () => api.updateTask(task.id, before));
}

/** New start / due for a task (a Planner drag or keyboard move). */
export interface TaskDates { start: string | null; startHasTime: boolean; due: string | null; dueHasTime: boolean }

/**
 * Saves a task's new dates, keeping every other field, and offers Undo (which puts the task back
 * as it was). `message` is the toast's text key (with {title}). A failed save throws; the caller
 * reverts its preview and reports the error.
 */
export async function saveDates(task: Task, next: TaskDates, message: Key = 'planner.moved'): Promise<void> {
  const before = taskInput(task);
  await api.updateTask(task.id, { ...before, ...next });
  notify(t(message, { title: task.title }), () => api.updateTask(task.id, before));
}

/**
 * A new task's input with the app's defaults: the default calendar, the local time zone, no date.
 * Without a plan the store uses the default plan; without a bucket, the plan's first bucket.
 * `oncreated` runs as soon as the task exists, before the (optional) move into the plan.
 */
export async function newTaskInput(title: string, planId: number | null = null, bucketId: number | null = null): Promise<TaskInput> {
  const [s, tz] = await Promise.all([api.getSettings(), api.localTimeZone()]);
  return {
    calendarId: s.defaultCalendarId, title, due: null, dueHasTime: false, tz, rrule: null, notes: null,
    alerts: [], status: 'todo', labels: [], planId, bucketId,
  };
}

/**
 * Creates a task from quick-entry text (Tasks page, Grid). Inside a plan, a task without its own
 * plan hint goes to the end of that plan's first bucket.
 */
export async function quickAddTask(typed: string, scopePlan: Plan | null, first: Bucket | undefined, tasks: Task[], oncreated?: () => void): Promise<void> {
  // Parse now rather than trusting a debounced preview, which may lag behind a fast Enter.
  const hint = scopePlan ? (await api.parsePreview(typed, true)).plan : null;
  const ref = await api.createFromText(typed, true);
  // The task exists now: the caller clears its input here, so a failed move can't cause a duplicate.
  oncreated?.();
  if (!scopePlan || !first || ref.kind !== 'task') return;
  const [created, defaultPlanId] = await Promise.all([api.getTask(ref.id), api.getDefaultPlan()]);
  if (quickAddNeedsMove({ hint, taskPlanId: created.planId, scopePlanId: scopePlan.id, defaultPlanId })) {
    const last = lastTopLevelIn(tasks, first.id);
    await api.moveTask(ref.id, first.id, last?.id ?? null, null);
  }
}
