// Conversions between store records and inspector form fields.
import type { AlertSpecUtc, EventInput, TaskInput, TaskStatus } from './types';
import { addDays } from './layout';
import { ymd } from './format';

const pad = (n: number) => String(n).padStart(2, '0');
/** `YYYY-MM-DDTHH:MM` in local time, for <input type="datetime-local">. */
export const toLocalInput = (iso: string) => {
  const d = new Date(iso);
  return `${ymd(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
export const fromLocalInput = (s: string) => new Date(s).toISOString();
export const dateOf = (iso: string) => ymd(new Date(iso));
/** Local midnight of a `YYYY-MM-DD` date as ISO UTC. */
export const midnight = (date: string) => {
  const [y, m, d] = date.split('-').map(Number);
  return new Date(y, m - 1, d).toISOString();
};

export interface EventForm {
  title: string;
  calendarId: number;
  allDay: boolean;
  start: string; // datetime-local or date
  end: string;
  location: string;
  attendees: string;
  url: string;
  notes: string;
  repeat: string; // '', 'DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY' or 'custom'
  rrule: string | null;
  alerts: number[]; // minutes before
  absoluteAlerts: AlertSpecUtc[];
  tz: string;
}

export function eventToForm(e: EventInput): EventForm {
  const freq = e.rrule?.match(/^FREQ=(\w+);INTERVAL=1$/)?.[1];
  return {
    title: e.title,
    calendarId: e.calendarId,
    allDay: e.allDay,
    start: e.allDay ? dateOf(e.start) : toLocalInput(e.start),
    end: e.allDay ? ymd(addDays(new Date(e.end), -1)) : toLocalInput(e.end),
    location: e.location ?? '',
    attendees: e.attendees.join(', '),
    url: e.url ?? '',
    notes: e.notes ?? '',
    repeat: e.rrule ? freq ?? 'custom' : '',
    rrule: e.rrule,
    alerts: e.alerts.filter((a) => a.type === 'before').map((a) => a.value as number),
    absoluteAlerts: e.alerts.filter((a) => a.type === 'at'),
    tz: e.tz,
  };
}

export function formToEvent(f: EventForm): EventInput {
  let start: string, end: string;
  if (f.allDay) {
    start = midnight(f.start.slice(0, 10));
    const last = f.end ? f.end.slice(0, 10) : f.start.slice(0, 10);
    end = midnight(ymd(addDays(new Date(midnight(last)), 1)));
  } else {
    start = fromLocalInput(f.start);
    end = f.end ? fromLocalInput(f.end) : new Date(new Date(start).getTime() + 3_600_000).toISOString();
  }
  if (new Date(end) < new Date(start)) end = start;
  const rrule = f.repeat === 'custom' ? f.rrule : f.repeat ? `FREQ=${f.repeat};INTERVAL=1` : null;
  const strOrNull = (s: string) => (s.trim() ? s.trim() : null);
  return {
    calendarId: f.calendarId,
    title: f.title.trim(),
    start,
    end,
    tz: f.tz,
    allDay: f.allDay,
    location: strOrNull(f.location),
    attendees: f.attendees.split(',').map((s) => s.trim()).filter(Boolean),
    notes: strOrNull(f.notes),
    url: strOrNull(f.url),
    rrule,
    alerts: [...f.alerts.map((m) => ({ type: 'before' as const, value: m })), ...f.absoluteAlerts],
  };
}

export interface TaskForm {
  title: string;
  calendarId: number;
  dueDate: string;
  dueTime: string;
  notes: string;
  rrule: string | null;
  alerts: AlertSpecUtc[];
  tz: string;
  status: TaskStatus;
  labels: string[];
}

export function taskToForm(t: TaskInput): TaskForm {
  const pad2 = (n: number) => String(n).padStart(2, '0');
  const d = t.due ? new Date(t.due) : null;
  return {
    title: t.title,
    calendarId: t.calendarId,
    dueDate: d ? ymd(d) : '',
    dueTime: d && t.dueHasTime ? `${pad2(d.getHours())}:${pad2(d.getMinutes())}` : '',
    notes: t.notes ?? '',
    rrule: t.rrule,
    alerts: t.alerts,
    tz: t.tz,
    status: t.status,
    labels: [...t.labels],
  };
}

export function formToTask(f: TaskForm): TaskInput {
  let due: string | null = null;
  if (f.dueDate) due = f.dueTime ? new Date(`${f.dueDate}T${f.dueTime}`).toISOString() : midnight(f.dueDate);
  return {
    calendarId: f.calendarId,
    title: f.title.trim(),
    due,
    dueHasTime: !!(f.dueDate && f.dueTime),
    tz: f.tz,
    rrule: f.rrule,
    notes: f.notes.trim() || null,
    alerts: f.alerts,
    status: f.status,
    labels: f.labels,
  };
}

/** RRULE body from a parser recurrence (mirrors Rust `RecurrenceSpec::to_rrule`). */
export function recurrenceToRrule(r: NonNullable<import('./types').ParsedItem['recurrence']>): string {
  let s = `FREQ=${r.rule.freq.toUpperCase()};INTERVAL=${r.rule.interval}`;
  if (r.rule.days?.length) s += `;BYDAY=${r.rule.days.map((d) => d.slice(0, 2).toUpperCase()).join(',')}`;
  if (r.rule.monthDay) s += `;BYMONTHDAY=${r.rule.monthDay}`;
  if (r.until) s += `;UNTIL=${r.until.replaceAll('-', '')}`;
  return s;
}

/** True when two ISO strings denote the same instant (formats differ between Rust and JS). */
export const sameInstant = (a: string, b: string) => new Date(a).getTime() === new Date(b).getTime();
