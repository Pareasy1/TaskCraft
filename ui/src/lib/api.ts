// Typed wrappers over the Rust commands.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { t } from './i18n';
import type {
  AddFilesResult, Attachment, Bucket, Calendar, ChecklistItem, CalendarInput, DeleteMode, DuplicateResult, EventFull, EventInput, Hit, ItemRef, Occurrence, ParsedItem, Plan,
  Scope, Settings, Task, TaskFilter, TaskInput, TaskLabel, TaskStatus, TemplateInfo,
} from './types';

export const api = {
  parsePreview: (text: string, asTask = false) => invoke<ParsedItem>('parse_preview', { text, asTask }),
  createFromText: (text: string, asTask = false) => invoke<ItemRef>('create_from_text', { text, asTask }),
  getEvent: (id: number) => invoke<EventFull>('get_event', { id }),
  createEvent: (input: EventInput) => invoke<number>('create_event', { input }),
  updateEvent: (id: number, input: EventInput) => invoke<void>('update_event', { id, input }),
  moveEvent: (id: number, occurrence: string | null, newStart: string, newEnd: string, scope: Scope) =>
    invoke<number>('move_event', { id, occurrence, newStart, newEnd, scope }),
  deleteEvent: (id: number, occurrence: string | null, scope: Scope) =>
    invoke<void>('delete_event', { id, occurrence, scope }),
  getTask: (id: number) => invoke<Task>('get_task', { id }),
  createTask: (input: TaskInput) => invoke<number>('create_task', { input }),
  updateTask: (id: number, input: TaskInput) => invoke<void>('update_task', { id, input }),
  setTaskCompleted: (id: number, done: boolean) => invoke<void>('set_task_completed', { id, done }),
  deleteTask: (id: number) => invoke<void>('delete_task', { id }),
  setTaskStatus: (id: number, status: TaskStatus) => invoke<void>('set_task_status', { id, status }),
  listLabels: () => invoke<TaskLabel[]>('list_labels'),
  updateLabel: (id: number, name: string, color: string) => invoke<void>('update_label', { id, name, color }),
  deleteLabel: (id: number) => invoke<void>('delete_label', { id }),
  listRange: (start: Date, end: Date) =>
    invoke<Occurrence[]>('list_range', { start: start.toISOString(), end: end.toISOString() }),
  listTasks: (filter: TaskFilter) => invoke<Task[]>('list_tasks', { filter }),
  search: (q: string) => invoke<Hit[]>('search', { q }),
  listCalendars: () => invoke<Calendar[]>('list_calendars'),
  upsertCalendar: (input: CalendarInput) => invoke<number>('upsert_calendar', { input }),
  deleteCalendar: (id: number, moveTo: number) => invoke<void>('delete_calendar', { id, moveTo }),
  getSettings: () => invoke<Settings>('get_settings'),
  setSettings: (settings: Settings) => invoke<Settings>('set_settings', { settings }),
  listPlans: () => invoke<Plan[]>('list_plans'),
  createPlan: (name: string, color: string) => invoke<number>('create_plan', { name, color }),
  updatePlan: (id: number, p: { name?: string | null; color?: string | null }) =>
    invoke<void>('update_plan', { id, name: p.name ?? null, color: p.color ?? null }),
  reorderPlans: (ids: number[]) => invoke<void>('reorder_plans', { ids }),
  deletePlan: (id: number, mode: DeleteMode) => invoke<void>('delete_plan', { id, mode }),
  listBuckets: (planId: number) => invoke<Bucket[]>('list_buckets', { planId }),
  createBucket: (planId: number, name: string) => invoke<number>('create_bucket', { planId, name }),
  renameBucket: (id: number, name: string) => invoke<void>('rename_bucket', { id, name }),
  reorderBuckets: (planId: number, ids: number[]) => invoke<void>('reorder_buckets', { planId, ids }),
  deleteBucket: (id: number, mode: DeleteMode) => invoke<void>('delete_bucket', { id, mode }),
  moveTask: (taskId: number, bucketId: number, before: number | null, after: number | null) =>
    invoke<void>('move_task', { taskId, bucketId, before, after }),
  listSubtasks: (parentId: number) => invoke<Task[]>('list_subtasks', { parentId }),
  reorderSubtasks: (parentId: number, ids: number[]) => invoke<void>('reorder_subtasks', { parentId, ids }),
  setTaskParent: (taskId: number, parentId: number | null) => invoke<void>('set_task_parent', { taskId, parentId }),
  completeWithSubtasks: (id: number) => invoke<void>('complete_with_subtasks', { id }),
  listChecklist: (taskId: number) => invoke<ChecklistItem[]>('list_checklist', { taskId }),
  addChecklistItem: (taskId: number, text: string) => invoke<number>('add_checklist_item', { taskId, text }),
  updateChecklistItem: (id: number, p: { text?: string | null; done?: boolean | null }) =>
    invoke<void>('update_checklist_item', { id, text: p.text ?? null, done: p.done ?? null }),
  reorderChecklist: (taskId: number, ids: number[]) => invoke<void>('reorder_checklist', { taskId, ids }),
  deleteChecklistItem: (id: number) => invoke<void>('delete_checklist_item', { id }),
  checklistItemToSubtask: (id: number) => invoke<number>('checklist_item_to_subtask', { id }),
  listAttachments: (taskId: number) => invoke<Attachment[]>('list_attachments', { taskId }),
  pickAndAddFiles: (taskId: number) => invoke<AddFilesResult>('pick_and_add_files', { taskId }),
  addLinkAttachment: (taskId: number, url: string, name: string | null = null) =>
    invoke<Attachment>('add_link_attachment', { taskId, url, name }),
  renameAttachment: (id: number, name: string) => invoke<void>('rename_attachment', { id, name }),
  reorderAttachments: (taskId: number, ids: number[]) => invoke<void>('reorder_attachments', { taskId, ids }),
  removeAttachment: (id: number) => invoke<Attachment>('remove_attachment', { id }),
  restoreFileAttachment: (taskId: number, name: string, stored: string, size: number) =>
    invoke<Attachment>('restore_file_attachment', { taskId, name, stored, size }),
  openAttachment: (id: number) => invoke<void>('open_attachment', { id }),
  revealAttachment: (id: number) => invoke<void>('reveal_attachment', { id }),
  duplicateTask: (id: number) => invoke<DuplicateResult>('duplicate_task', { id }),
  saveTemplate: (taskId: number, name: string, replace = false) => invoke<number>('save_template', { taskId, name, replace }),
  listTemplates: () => invoke<TemplateInfo[]>('list_templates'),
  renameTemplate: (id: number, name: string) => invoke<void>('rename_template', { id, name }),
  deleteTemplate: (id: number) => invoke<void>('delete_template', { id }),
  createFromTemplate: (templateId: number, input: TaskInput) => invoke<number>('create_from_template', { templateId, input }),
  getDefaultPlan: () => invoke<number>('get_default_plan'),
  setDefaultPlan: (id: number) => invoke<void>('set_default_plan', { id }),
  exportIcs: (path: string) => invoke<void>('export_ics', { path }),
  localTimeZone: () => invoke<string>('local_time_zone'),
  openMain: () => invoke<void>('open_main'),
  openSettings: () => invoke<void>('open_settings'),
};

/** Subscribes to data changes from any window. */
export function onItemsChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('items-changed', cb);
}

const TRANSLATED_ERRORS = ['empty', 'no_calendar', 'not_found', 'hotkey_unavailable', 'duplicate', 'last_plan', 'last_bucket',
  'parent_is_subtask', 'has_subtasks', 'repeating_subtask', 'start_after_due', 'checklist_text',
  'attachment_too_large', 'attachment_url', 'template_name'] as const;

/** The app-language message for an error code, or null when the code has none. */
export function knownErrorMessage(code: string): string | null {
  return (TRANSLATED_ERRORS as readonly string[]).includes(code) ? t(`error.${code as (typeof TRANSLATED_ERRORS)[number]}`) : null;
}

/** A user-facing message for an error from a command, in the app language when known. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'code' in e) {
    const known = knownErrorMessage(String((e as { code: unknown }).code));
    if (known) return known;
  }
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return String(e);
}
