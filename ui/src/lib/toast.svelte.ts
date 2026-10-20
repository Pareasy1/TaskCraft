// One transient message at the bottom of the main window, optionally with Undo.
import { errorMessage } from './api';
import { t } from './i18n';

const SHOW_MS = 6000;

let undoFn: (() => Promise<void>) | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export const toast = $state<{ current: { message: string; canUndo: boolean } | null }>({ current: null });

export function notify(message: string, onUndo?: () => Promise<void>) {
  clearTimeout(timer);
  undoFn = onUndo ?? null;
  toast.current = { message, canUndo: !!onUndo };
  timer = setTimeout(dismiss, SHOW_MS);
}

export function dismiss() {
  clearTimeout(timer);
  undoFn = null;
  toast.current = null;
}

export async function undo() {
  const fn = undoFn;
  if (!fn) return;
  dismiss();
  try {
    await fn();
  } catch (e) {
    notify(t('msg.undoFailed', { reason: errorMessage(e) }));
  }
}
