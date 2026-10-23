// The pending "Complete the open subtasks too?" question, answered by CompleteDialog (rendered once in Main).
import type { Task } from './types';

export type CompleteChoice = 'all' | 'only' | 'cancel';

let resolveFn: ((c: CompleteChoice) => void) | null = null;

export const pending = $state<{ current: { task: Task; openChildren: number } | null }>({ current: null });

/** Asks what to do with `task`'s open subtasks. A newer request cancels an unanswered one. */
export function requestComplete(task: Task, openChildren: number): Promise<CompleteChoice> {
  resolveFn?.('cancel');
  pending.current = { task, openChildren };
  return new Promise((resolve) => (resolveFn = resolve));
}

export function answerComplete(choice: CompleteChoice) {
  const fn = resolveFn;
  resolveFn = null;
  pending.current = null;
  fn?.(choice);
}
