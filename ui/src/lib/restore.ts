// Snapshots taken before a task is deleted, so Undo can bring back what went with it.
import { api } from './api';
import { taskInput } from './edit';
import { t } from './i18n';
import { notify } from './toast.svelte';
import type { Attachment, TaskInput } from './types';

export type AttachmentSnapshot = Pick<Attachment, 'kind' | 'name' | 'stored' | 'url' | 'size'>;

export interface TaskSnapshot {
  input: TaskInput;
  checklist: { text: string; done: boolean }[];
  /** In their order. A deleted task's stored files wait 15 s before going, so Undo can re-link them. */
  attachments: AttachmentSnapshot[];
  /** The task's own id before the delete (to remap dependencies inside a restored tree). */
  id: number;
  /** Ids of tasks this one depends on. */
  dependsOn: number[];
  /** Ids of tasks that depend on this one. */
  blocks: number[];
}

async function snapshot(id: number): Promise<TaskSnapshot> {
  const [task, list, atts, deps] = await Promise.all([
    api.getTask(id), api.listChecklist(id), api.listAttachments(id), api.listDependencies([id]),
  ]);
  return {
    id,
    dependsOn: deps.filter((d) => d.taskId === id).map((d) => d.dependsOn),
    blocks: deps.filter((d) => d.dependsOn === id).map((d) => d.taskId),
    input: taskInput(task),
    checklist: list.map(({ text, done }) => ({ text, done })),
    attachments: atts.map(({ kind, name, stored, url, size }) => ({ kind, name, stored, url, size })),
  };
}

/** Brings one attachment back on `taskId`: a file re-links its stored copy, a link is added again. */
export async function restoreAttachment(taskId: number, a: AttachmentSnapshot): Promise<Attachment> {
  return a.kind === 'file'
    ? api.restoreFileAttachment(taskId, a.name, a.stored ?? '', a.size ?? 0)
    : api.addLinkAttachment(taskId, a.url ?? '', a.name);
}

/**
 * Creates the task, then its checklist items (with their done state) and attachments, in order.
 * Best effort for attachments: one that cannot come back (e.g. its file was already purged) is
 * skipped and its name pushed on `missed`. Returns the new id.
 */
async function recreate(s: TaskSnapshot, missed: string[], parentId?: number, withDeps = true): Promise<number> {
  const id = await api.createTask(parentId === undefined ? s.input : { ...s.input, parentId });
  for (const item of s.checklist) {
    const itemId = await api.addChecklistItem(id, item.text);
    if (item.done) await api.updateChecklistItem(itemId, { done: true });
  }
  for (const a of s.attachments) {
    try {
      await restoreAttachment(id, a);
    } catch {
      missed.push(a.name);
    }
  }
  if (withDeps) await restoreDependencies(id, s);
  return id;
}

/**
 * Re-adds a restored task's dependencies in both directions. `remap`: old id -> new id for tasks
 * restored together (a parent and its subtasks). A pair whose other task is gone (or that would
 * now make a cycle) is skipped silently.
 */
async function restoreDependencies(newId: number, s: TaskSnapshot, remap: Map<number, number> = new Map()): Promise<void> {
  const to = (x: number) => remap.get(x) ?? x;
  for (const x of s.dependsOn) {
    try { await api.addDependency(newId, to(x)); } catch { /* other task is gone */ }
  }
  for (const y of s.blocks) {
    // Inside a tree the other side adds this pair from its own dependsOn.
    if (remap.has(y)) continue;
    try { await api.addDependency(y, newId); } catch { /* other task is gone */ }
  }
}

/** One toast naming every attachment that could not come back. */
function reportMissed(missed: string[]): void {
  if (missed.length) notify(t('msg.restoredWithout', { names: missed.join(', ') }));
}

// ── A task with its subtasks (they are deleted with it) ──
export interface TreeSnapshot { task: TaskSnapshot; subtasks: TaskSnapshot[] }

export async function snapshotWithSubtasks(id: number): Promise<TreeSnapshot> {
  const [task, subs] = await Promise.all([snapshot(id), api.listSubtasks(id)]);
  return { task, subtasks: await Promise.all(subs.map((s) => snapshot(s.id))) };
}

/**
 * Recreates the task (checklist, attachments), then each subtask in their order with its own.
 * An In progress subtask makes the store start a To do parent; the parent's saved To do is put
 * back last, so Undo restores exactly what was deleted.
 */
export async function restoreWithSubtasks(s: TreeSnapshot): Promise<number> {
  const missed: string[] = [];
  const id = await recreate(s.task, missed, undefined, false);
  const remap = new Map([[s.task.id, id]]);
  const created = [{ snap: s.task, id }];
  for (const sub of s.subtasks) {
    const subId = await recreate(sub, missed, id, false);
    remap.set(sub.id, subId);
    created.push({ snap: sub, id: subId });
  }
  // Dependencies last, once every task of the tree has its new id.
  for (const c of created) await restoreDependencies(c.id, c.snap, remap);
  if (s.task.input.status === 'todo' && s.subtasks.some((x) => x.input.status === 'inProgress')) {
    await api.setTaskStatus(id, 'todo');
  }
  reportMissed(missed);
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

/**
 * Recreates the subtask and its checklist, then puts it back in its place among its siblings.
 * Coming back In progress must not start a To do parent: it is put back to To do last.
 */
export async function restoreSubtask(s: SubtaskSnapshot): Promise<number> {
  const parentId = s.task.input.parentId;
  const parentTodo = parentId != null && s.task.input.status === 'inProgress'
    && (await api.getTask(parentId).then((p) => p.status === 'todo', () => false));
  const missed: string[] = [];
  const id = await recreate(s.task, missed);
  if (parentId != null) {
    const current = (await api.listSubtasks(parentId)).map((x) => x.id);
    const order = restoredOrder(s.order, s.id, id, current);
    if (order.some((x, i) => x !== current[i])) await api.reorderSubtasks(parentId, order);
    if (parentTodo) await api.setTaskStatus(parentId, 'todo');
  }
  reportMissed(missed);
  return id;
}

// ── One removed attachment ──
/** Brings a removed attachment back and puts it in its old place. `order`: the ids before the remove. */
export async function restoreRemovedAttachment(a: Attachment, order: number[]): Promise<void> {
  const back = await restoreAttachment(a.taskId, a);
  const current = (await api.listAttachments(a.taskId)).map((x) => x.id);
  const next = restoredOrder(order, a.id, back.id, current);
  if (next.some((x, i) => x !== current[i])) await api.reorderAttachments(a.taskId, next);
}
