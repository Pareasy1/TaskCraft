// The pending completion question, answered by CompleteDialog (rendered once in Main):
// "Complete the open subtasks too?" when ticking a parent, or "Mark the task as done too?"
// once its last subtask is done.
import type { Task } from './types';

/** 'all' / 'only' / 'cancel' answer the subtasks question; 'yes' / 'cancel' the parent one. */
export type CompleteChoice = 'all' | 'only' | 'yes' | 'cancel';

export type CompleteAsk =
  | { kind: 'children'; task: Task; openChildren: number }
  | { kind: 'parent'; task: Task };

let resolveFn: ((c: CompleteChoice) => void) | null = null;

export const pending = $state<{ current: CompleteAsk | null }>({ current: null });

/** A newer request cancels an unanswered one. */
function ask(q: CompleteAsk): Promise<CompleteChoice> {
  resolveFn?.('cancel');
  pending.current = q;
  return new Promise((resolve) => (resolveFn = resolve));
}

/** Asks what to do with `task`'s open subtasks. */
export function requestComplete(task: Task, openChildren: number): Promise<CompleteChoice> {
  return ask({ kind: 'children', task, openChildren });
}

/** All of `parent`'s subtasks are done: asks whether to mark it done too (true = Yes). */
export async function requestCompleteParent(parent: Task): Promise<boolean> {
  return (await ask({ kind: 'parent', task: parent })) === 'yes';
}

export function answerComplete(choice: CompleteChoice) {
  const fn = resolveFn;
  resolveFn = null;
  pending.current = null;
  fn?.(choice);
}
