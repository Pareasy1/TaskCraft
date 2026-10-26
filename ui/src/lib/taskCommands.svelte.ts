// Duplicate and "Save as template…", shared by the card menus and the details dialog.
// The template name dialog is rendered once in Main and reads `templateSave`.
import { api } from './api';
import { t } from './i18n';
import { duplicatedMessage } from './templates';
import { notify } from './toast.svelte';
import type { Task } from './types';

/** The task being saved as a template, while its name dialog is open. */
export const templateSave = $state<{ current: Task | null }>({ current: null });

export function askSaveTemplate(task: Task) {
  templateSave.current = task;
}
export function closeSaveTemplate() {
  templateSave.current = null;
}

/**
 * Duplicates a task (with its subtasks, checklist and attachments) and says so, naming any files
 * that could not be copied. Undo deletes the copy; `open` adds a button that opens it.
 */
export async function duplicate(task: Task, open?: (id: number) => void): Promise<number> {
  const r = await api.duplicateTask(task.id);
  notify(
    duplicatedMessage(task.title, r.skipped),
    () => api.deleteTask(r.id),
    open ? { label: t('task.openCopy'), run: async () => open(r.id) } : undefined,
  );
  return r.id;
}
