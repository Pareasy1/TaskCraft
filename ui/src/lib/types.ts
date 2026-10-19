// Mirrors of the Rust types exchanged over Tauri commands (camelCase).

export type Label =
  | 'title' | 'taskPrefix' | 'date' | 'time' | 'duration' | 'recurrence'
  | 'alert' | 'location' | 'attendees' | 'calendar' | 'url' | 'separator';

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

export interface TaskInput {
  calendarId: number;
  title: string;
  due: string | null;
  dueHasTime: boolean;
  tz: string;
  rrule: string | null;
  notes: string | null;
  alerts: AlertSpecUtc[];
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
