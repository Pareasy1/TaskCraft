// Typed wrappers over the Rust commands.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { t } from './i18n';
import type {
  Calendar, CalendarInput, EventFull, EventInput, Hit, ItemRef, Occurrence, ParsedItem,
  Scope, Settings, Task, TaskFilter, TaskInput, TaskLabel, TaskStatus,
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
  exportIcs: (path: string) => invoke<void>('export_ics', { path }),
  localTimeZone: () => invoke<string>('local_time_zone'),
  openMain: () => invoke<void>('open_main'),
  openSettings: () => invoke<void>('open_settings'),
};

/** Subscribes to data changes from any window. */
export function onItemsChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('items-changed', cb);
}

const TRANSLATED_ERRORS = ['empty', 'no_calendar', 'not_found', 'hotkey_unavailable'] as const;

/** A user-facing message for an error from a command, in the app language when known. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'code' in e) {
    const code = String((e as { code: unknown }).code);
    if ((TRANSLATED_ERRORS as readonly string[]).includes(code)) return t(`error.${code as (typeof TRANSLATED_ERRORS)[number]}`);
  }
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return String(e);
}
