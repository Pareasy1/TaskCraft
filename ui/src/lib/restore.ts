// Snapshots taken before a task is deleted, so Undo can bring back what went with it.
import { api } from './api';
import { taskInput } from './edit';
import type { TaskInput } from './types';

export interface TaskSnapshot {
  input: TaskInput;
  checklist: { text: string; done: boolean }[];
}

async function snapshot(id: number): Promise<TaskSnapshot> {
  const [task, list] = await Promise.all([api.getTask(id), api.listChecklist(id)]);
  return { input: taskInput(task), checklist: list.map(({ text, done }) => ({ text, done })) };
}

/** Creates the task, then its checklist items in order with their done state. Returns the new id. */
async function recreate(s: TaskSnapshot, parentId?: number): Promise<number> {
  const id = await api.createTask(parentId === undefined ? s.input : { ...s.input, parentId });
  for (const item of s.checklist) {
    const itemId = await api.addChecklistItem(id, item.text);
    if (item.done) await api.updateChecklistItem(itemId, { done: true });
  }
  return id;
}

// ── A task with its subtasks (they are deleted with it) ──
export interface TreeSnapshot { task: TaskSnapshot; subtasks: TaskSnapshot[] }

export async function snapshotWithSubtasks(id: number): Promise<TreeSnapshot> {
  const [task, subs] = await Promise.all([snapshot(id), api.listSubtasks(id)]);
  return { task, subtasks: await Promise.all(subs.map((s) => snapshot(s.id))) };
}

/** Recreates the task and its checklist, then each subtask (in their order) with its checklist. */
export async function restoreWithSubtasks(s: TreeSnapshot): Promise<number> {
  const id = await recreate(s.task);
  for (const sub of s.subtasks) await recreate(sub, id);
  return id;
}

// ── One subtask ──
export interface SubtaskSnapshot { id: number; task: TaskSnapshot; order: number[] }

/** `order`: the sibling ids in their order before the delete, this one included. */
export async function snapshotSubtask(id: number, order: number[]): Promise<SubtaskSnapshot> {
  return { id, task: await snapshot(id), order };
}

/**
 * Sibling order after `oldId` came back as `newId`: the original order with siblings that are
 * gone dropped, then any added since, in their current order.
 */
export function restoredOrder(original: number[], oldId: number, newId: number, current: number[]): number[] {
  const now = new Set(current);
  const kept = original.map((x) => (x === oldId ? newId : x)).filter((x) => now.has(x));
  const seen = new Set(kept);
  return [...kept, ...current.filter((x) => !seen.has(x))];
}

/** Recreates the subtask and its checklist, then puts it back in its place among its siblings. */
export async function restoreSubtask(s: SubtaskSnapshot): Promise<number> {
  const parentId = s.task.input.parentId;
  const id = await recreate(s.task);
  if (parentId == null) return id;
  const current = (await api.listSubtasks(parentId)).map((x) => x.id);
  const order = restoredOrder(s.order, s.id, id, current);
  if (order.some((x, i) => x !== current[i])) await api.reorderSubtasks(parentId, order);
  return id;
}
