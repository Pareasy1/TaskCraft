// Pure helpers for subtasks, checklists and priority columns.
import { t } from './i18n';
import { PRIORITIES, type Priority } from './types';

type Child = { id: number; parentId: number | null; sortOrder: number };
const bySort = (a: Child, b: Child) => a.sortOrder - b.sortOrder || a.id - b.id;

/** Subtasks keyed by parent id, siblings in order. Parents without subtasks have no entry. */
export function childrenByParent<T extends Child>(tasks: T[]): Map<number, T[]> {
  const m = new Map<number, T[]>();
  for (const x of tasks) {
    if (x.parentId == null) continue;
    const list = m.get(x.parentId);
    if (list) list.push(x);
    else m.set(x.parentId, [x]);
  }
  for (const list of m.values()) list.sort(bySort);
  return m;
}

export function topLevel<T extends Child>(tasks: T[]): T[] {
  return tasks.filter((x) => x.parentId == null);
}

/**
 * Each shown top-level task with its shown children. A shown subtask whose parent
 * is not shown appears under that parent (taken from `all`), which is `dimmed`.
 */
export function groupWithParents<T extends Child>(shown: T[], all: T[]): { parent: T; dimmed: boolean; children: T[] }[] {
  const byId = new Map(all.map((x) => [x.id, x]));
  const shownIds = new Set(shown.map((x) => x.id));
  const out = new Map<number, { parent: T; dimmed: boolean; children: T[] }>();
  for (const x of shown) {
    if (x.parentId == null) {
      if (!out.has(x.id)) out.set(x.id, { parent: x, dimmed: false, children: [] });
      continue;
    }
    const parent = byId.get(x.parentId);
    if (!parent) continue;
    if (!out.has(parent.id)) out.set(parent.id, { parent, dimmed: !shownIds.has(parent.id), children: [] });
    out.get(parent.id)!.children.push(x);
  }
  for (const g of out.values()) g.children.sort(bySort);
  return [...out.values()];
}

export function progressLabel(x: { subtaskOpen: number; subtaskTotal: number }): string | null {
  if (!x.subtaskTotal) return null;
  return t('subtasks.progress', { done: x.subtaskTotal - x.subtaskOpen, total: x.subtaskTotal });
}

export function checklistLabel(x: { checklistDone: number; checklistTotal: number }): string | null {
  return x.checklistTotal ? `${x.checklistDone}/${x.checklistTotal}` : null;
}

type Done = { completedAt: string | null };

/** Completing a task that still has open subtasks asks what to do with them. */
export function completionAsk(_task: Done, children: Done[]): 'askChildren' | 'none' {
  return children.some((c) => !c.completedAt) ? 'askChildren' : 'none';
}

/** After `task` was completed: true when no sibling is open any more and the parent is still open. */
export function offerParentAfter(task: Done & { id: number }, siblings: (Done & { id: number })[], parent: Done): boolean {
  return !parent.completedAt && siblings.every((s) => s.id === task.id || !!s.completedAt);
}

export function priorityColumns<T extends { priority: Priority; sortOrder: number; id: number }>(tasks: T[]): { key: Priority; tasks: T[] }[] {
  return PRIORITIES.map((key) => ({
    key,
    tasks: tasks.filter((x) => x.priority === key).sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id),
  }));
}
