// Pure helpers for subtasks, checklists and priority columns.
import { t } from './i18n';
import { PRIORITIES, type Priority, type TaskInput, type TaskStatus } from './types';

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
 * is not shown appears under that parent (taken from `all`), which is `dimmed`. A subtask whose
 * parent is not in `all` either (e.g. on a hidden calendar) is a plain row of its own.
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
    if (!parent) {
      if (!out.has(x.id)) out.set(x.id, { parent: x, dimmed: false, children: [] });
      continue;
    }
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

/** The subtask bar's segments, as counts: done, in progress, still to do. */
export function progressSegments(x: { subtaskTotal: number; subtaskOpen: number; subtaskInProgress: number }): { done: number; inProgress: number; todo: number } {
  const open = Math.max(0, Math.min(x.subtaskOpen, x.subtaskTotal));
  const inProgress = Math.max(0, Math.min(x.subtaskInProgress, open));
  return { done: x.subtaskTotal - open, inProgress, todo: open - inProgress };
}

/**
 * The bar's track: 'started' (tinted yellow) when the parent is in progress but no subtask is,
 * so a started task still looks started; otherwise neutral (a subtask in progress already shows yellow).
 */
export function barTrack(x: { status: TaskStatus; subtaskInProgress: number }): 'neutral' | 'started' {
  return x.status === 'inProgress' && x.subtaskInProgress === 0 ? 'started' : 'neutral';
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

/** A subtask's form save that took it from open to done (the parent question may follow). */
export function completedBySave(before: { status: TaskStatus; parentId: number | null }, after: { status: TaskStatus; parentId: number | null }): boolean {
  return after.parentId != null && before.status !== 'done' && after.status === 'done';
}

export function priorityColumns<T extends { priority: Priority; sortOrder: number; id: number }>(tasks: T[]): { key: Priority; tasks: T[] }[] {
  return PRIORITIES.map((key) => ({
    key,
    tasks: tasks.filter((x) => x.priority === key).sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id),
  }));
}

type Candidate = Child & { planId: number; title: string; completedAt: string | null };

/**
 * Tasks that can be picked as `selfId`'s parent: open top-level tasks of the same plan,
 * by title. The current parent stays listed even when completed, so the picker shows it.
 */
export function parentCandidates<T extends Candidate>(tasks: T[], selfId: number, planId: number, currentParentId: number | null): T[] {
  return tasks
    .filter((x) => x.id !== selfId && x.parentId == null && x.planId === planId && (!x.completedAt || x.id === currentParentId))
    .sort((a, b) => a.title.localeCompare(b.title) || a.id - b.id);
}

/** `ids` with `id` moved one place up (-1) or down (1); null when it can't move. */
export function shiftId(ids: number[], id: number, by: -1 | 1): number[] | null {
  const i = ids.indexOf(id), j = i + by;
  if (i < 0 || j < 0 || j >= ids.length) return null;
  const out = [...ids];
  [out[i], out[j]] = [out[j], out[i]];
  return out;
}

/** Row to focus after `id` leaves the list: the next one, else the previous one, else none. */
export function focusAfterRemoval(ids: number[], id: number): number | null {
  const i = ids.indexOf(id);
  if (i < 0) return null;
  return ids[i + 1] ?? ids[i - 1] ?? null;
}

/** What `changeStatus` needs to decide on prompts: the task's subtasks, its siblings (itself included) and its parent. */
export function statusCtx<T extends Child>(task: T, all: T[]): { children: T[]; siblings: T[]; parent: T | null } {
  const parent = task.parentId == null ? null : all.find((x) => x.id === task.parentId) ?? null;
  return {
    children: all.filter((x) => x.parentId === task.id).sort(bySort),
    siblings: parent ? all.filter((x) => x.parentId === parent.id).sort(bySort) : [],
    parent,
  };
}

/**
 * The updates that undo "Complete all": each subtask it closed back as it was, then the parent.
 * `closed` holds only the subtasks that were open before.
 */
export function completeUndo(parentId: number, before: TaskInput, closed: { id: number; input: TaskInput }[]): { id: number; input: TaskInput }[] {
  return [...closed, { id: parentId, input: before }];
}

/**
 * The Tasks list: each date group's rows of parent + indented subtasks. A shown parent takes
 * every shown subtask of its own, whichever group the subtask fell in; a subtask whose parent
 * is not shown stays in its own group under a dimmed parent. Groups left empty are dropped.
 */
export function nestGroups<T extends Child, G extends { tasks: T[] }>(groups: G[], all: T[]): (G & { rows: { parent: T; dimmed: boolean; children: T[] }[] })[] {
  const shown = groups.flatMap((g) => g.tasks);
  const shownTop = new Set(shown.filter((x) => x.parentId == null).map((x) => x.id));
  const kids = childrenByParent(shown);
  return groups
    .map((g) => ({
      ...g,
      rows: groupWithParents(g.tasks.filter((x) => x.parentId == null || !shownTop.has(x.parentId)), all)
        .map((r) => (r.dimmed ? r : { ...r, children: kids.get(r.parent.id) ?? [] })),
    }))
    .filter((g) => g.rows.length);
}

/** Rows to draw: each parent, then its subtasks unless it is collapsed. `kids` = its subtasks in this group. */
export function flattenRows<T extends { id: number }>(
  groups: { parent: T; dimmed: boolean; children: T[] }[],
  collapsed: Set<number>,
): { task: T; sub: boolean; dimmed: boolean; kids: number }[] {
  return groups.flatMap((g) => [
    { task: g.parent, sub: false, dimmed: g.dimmed, kids: g.children.length },
    ...(collapsed.has(g.parent.id) ? [] : g.children.map((c) => ({ task: c, sub: true, dimmed: false, kids: 0 }))),
  ]);
}
