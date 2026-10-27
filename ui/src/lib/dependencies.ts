// Dependency helpers: who blocks whom, cycle check, and the "still open" question text.
import { t } from './i18n';
import type { Dependency, Task } from './types';

const isOpen = (x: Task) => !x.completedAt && x.status !== 'done';

/** The open tasks `id` depends on. */
export function blockers(id: number, deps: Dependency[], tasks: Task[]): Task[] {
  const ids = new Set(deps.filter((d) => d.taskId === id).map((d) => d.dependsOn));
  return tasks.filter((x) => ids.has(x.id) && isOpen(x));
}

/** The tasks that depend on `id`. */
export function blocking(id: number, deps: Dependency[], tasks: Task[]): Task[] {
  const ids = new Set(deps.filter((d) => d.dependsOn === id).map((d) => d.taskId));
  return tasks.filter((x) => ids.has(x.id));
}

/** True when "`taskId` depends on `dependsOn`" is the task itself or closes a loop. */
export function wouldCycle(taskId: number, dependsOn: number, deps: Dependency[]): boolean {
  if (taskId === dependsOn) return true;
  const seen = new Set<number>();
  const stack = [dependsOn];
  while (stack.length) {
    const cur = stack.pop()!;
    if (cur === taskId) return true;
    if (seen.has(cur)) continue;
    seen.add(cur);
    for (const d of deps) if (d.taskId === cur) stack.push(d.dependsOn);
  }
  return false;
}

/** The question asked before starting or finishing a task that waits on open ones. */
export function blockedPrompt(list: { title: string }[]): string {
  return list.length === 1 ? t('dep.blockedOne', { title: list[0].title }) : t('dep.blockedMany', { n: list.length });
}

const fold = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

/** Picker filter: case and accents ignored; an empty query matches everything. */
export function pickerMatches(title: string, query: string): boolean {
  const q = fold(query).trim();
  return !q || fold(title).includes(q);
}

/** Ids the picker must not offer for `taskId`: itself, its current blockers, and any task that would close a loop. */
export function pickerExclude(taskId: number, deps: Dependency[]): Set<number> {
  const out = new Set<number>([taskId]);
  for (const d of deps) {
    if (d.taskId === taskId) out.add(d.dependsOn);
    for (const id of [d.taskId, d.dependsOn]) if (wouldCycle(taskId, id, deps)) out.add(id);
  }
  return out;
}
