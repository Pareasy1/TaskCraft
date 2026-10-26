// Mirrors of the Rust types exchanged over Tauri commands (camelCase).

export type Label =
  | 'title' | 'taskPrefix' | 'date' | 'time' | 'duration' | 'recurrence'
  | 'alert' | 'location' | 'attendees' | 'calendar' | 'url' | 'separator' | 'tag' | 'plan' | 'priority' | 'template';

export type Priority = 'urgent' | 'important' | 'medium' | 'low';
export const PRIORITIES: Priority[] = ['urgent', 'important', 'medium', 'low'];

export interface ChecklistItem { id: number; taskId: number; text: string; done: boolean; sort: number }

export interface When { date: string; time: string | null }
export interface Span { start: number; end: number; label: Label }
export type AlertSpec = { type: 'before'; value: number } | { type: 'at'; value: When };
export type AlertSpecUtc = { type: 'before'; value: number } | { type: 'at'; value: string };

export interface ParsedItem {
  kind: 'event' | 'task';
  title: string;
  start: When | null;
  end: When | null;
  allDay: boolean;
  location: string | null;
  attendees: string[];
  calendar: string | null;
  recurrence: { rule: { freq: string; interval: number; days?: string[]; monthDay?: number | null }; until: string | null } | null;
  alerts: AlertSpec[];
  url: string | null;
  tags: string[];
  plan: string | null;
  priority: Priority | null;
  template: string | null;
  spans: Span[];
}

export type ItemKind = 'event' | 'task';
export interface ItemRef { kind: ItemKind; id: number }

export interface Calendar { id: number; name: string; color: string; visible: boolean; sort: number }
export interface CalendarInput { id: number | null; name: string; color: string; visible: boolean }

export interface EventInput {
  calendarId: number;
  title: string;
  start: string;
  end: string;
  tz: string;
  allDay: boolean;
  location: string | null;
  attendees: string[];
  notes: string | null;
  url: string | null;
  rrule: string | null;
  alerts: AlertSpecUtc[];
}
export interface EventFull extends EventInput { id: number }

export type TaskStatus = 'todo' | 'inProgress' | 'done';
export interface TaskLabel { id: number; name: string; color: string }

export interface TaskInput {
  calendarId: number;
  title: string;
  due: string | null;
  dueHasTime: boolean;
  tz: string;
  rrule: string | null;
  notes: string | null;
  alerts: AlertSpecUtc[];
  status: TaskStatus;
  /** Label names; missing labels are created on save. */
  labels: string[];
  planId?: number | null;
  bucketId?: number | null;
  parentId?: number | null;
  priority?: Priority;
  start?: string | null;
  startHasTime?: boolean;
}

export interface Task {
  id: number;
  calendarId: number;
  title: string;
  due: string | null;
  dueHasTime: boolean;
  tz: string;
  completedAt: string | null;
  recurring: boolean;
  rrule: string | null;
  notes: string | null;
  color: string;
  alerts: AlertSpecUtc[];
  status: TaskStatus;
  labels: TaskLabel[];
  planId: number;
  bucketId: number;
  sortOrder: number;
  parentId: number | null;
  priority: Priority;
  start: string | null;
  startHasTime: boolean;
  subtaskOpen: number;
  subtaskTotal: number;
  /** Subtasks in progress (counted in `subtaskOpen` too). */
  subtaskInProgress: number;
  checklistDone: number;
  checklistTotal: number;
  attachmentCount: number;
}

export interface Occurrence {
  eventId: number;
  calendarId: number;
  title: string;
  start: string;
  end: string;
  allDay: boolean;
  location: string | null;
  recurring: boolean;
  color: string;
}

export type Scope = 'this' | 'all';
export type TaskFilter = 'overdue' | 'today' | 'upcoming' | 'completed' | 'all';
export interface Hit { kind: ItemKind; id: number; title: string; when: string | null }

export type Weekday = 'Mon' | 'Tue' | 'Wed' | 'Thu' | 'Fri' | 'Sat' | 'Sun';
export interface Settings {
  hotkey: string;
  launchAtLogin: boolean;
  defaultCalendarId: number;
  defaultEventMinutes: number;
  weekStart: Weekday;
  dateOrder: 'dayMonth' | 'monthDay';
}

export interface CmdError { code: string; message: string }

export interface Plan { id: number; name: string; color: string; sort: number; openCount: number }
export interface Bucket { id: number; planId: number; name: string; sort: number }
export type DeleteMode = { moveTo: number } | 'deleteTasks';

export interface Attachment {
  id: number;
  taskId: number;
  kind: 'file' | 'link';
  name: string;
  stored: string | null;
  url: string | null;
  size: number | null;
  createdAt: string;
  sort: number;
}
export interface AddFilesResult { added: Attachment[]; errors: { name: string; code: string }[] }
export interface DuplicateResult { id: number; skipped: string[] }
export interface TemplateInfo { id: number; name: string; subtaskCount: number }
