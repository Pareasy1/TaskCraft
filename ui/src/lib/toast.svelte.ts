// One transient message at the bottom of the main window, optionally with Undo and one more action.
import { errorMessage } from './api';
import { t, type Key } from './i18n';

const SHOW_MS = 6000;

export interface ToastAction { label: string; run: () => Promise<void> }

let undoFn: (() => Promise<void>) | null = null;
let actionFn: (() => Promise<void>) | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export const toast = $state<{ current: { message: string; canUndo: boolean; action?: string } | null }>({ current: null });

export function notify(message: string, onUndo?: () => Promise<void>, action?: ToastAction) {
  clearTimeout(timer);
  undoFn = onUndo ?? null;
  actionFn = action?.run ?? null;
  toast.current = { message, canUndo: !!onUndo, action: action?.label };
  timer = setTimeout(dismiss, SHOW_MS);
}

export function dismiss() {
  clearTimeout(timer);
  undoFn = actionFn = null;
  toast.current = null;
}

async function runOnce(fn: (() => Promise<void>) | null, failed: Key) {
  if (!fn) return;
  dismiss();
  try {
    await fn();
  } catch (e) {
    notify(t(failed, { reason: errorMessage(e) }));
  }
}

export const undo = () => runOnce(undoFn, 'msg.undoFailed');
/** The extra button (e.g. "Complete it"). */
export const runAction = () => runOnce(actionFn, 'msg.actionFailed');
