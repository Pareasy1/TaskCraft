// Dependency helpers: who blocks whom, cycle check, and the "still open" question text.
import { t } from './i18n';
import type { Dependency, Task } from './types';
import { ROW_H } from './timeline';

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

type Point = { x: number; y: number };
/** How far an arrow runs straight out of A's bar (and into B's when routed around). */
const STUB = 8;

/**
 * SVG path of the arrow "B depends on A": from the end of A's bar (`from`, right edge, row middle) to
 * the start of B's (`to`, left edge, row middle). Out, along to B's row, in; when B starts too close
 * or to the left, it turns half a row towards B, runs back past B's start, and comes in from the left.
 */
export function arrowPath(from: Point, to: Point): string {
  const out = from.x + STUB;
  if (to.x >= from.x + 2 * STUB) return `M${from.x} ${from.y}H${out}V${to.y}H${to.x}`;
  const mid = from.y + (to.y >= from.y ? 1 : -1) * (ROW_H / 2);
  return `M${from.x} ${from.y}H${out}V${mid}H${to.x - STUB}V${to.y}H${to.x}`;
}

/** True when B (depending on A) starts on a day before A is due: shown in the warning colour. */
type DaySpan = { from: number; to: number };
export const arrowConflict = (a: DaySpan, b: DaySpan): boolean => b.from < a.to;
