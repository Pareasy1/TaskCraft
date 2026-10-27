// Planner timeline geometry: pure helpers mapping tasks to day columns and back.
import { monthTitle } from './format';

export type Zoom = 'days' | 'weeks' | 'months';
export type DragMode = 'move' | 'start' | 'due';

export const DAY_WIDTH: Record<Zoom, number> = { days: 32, weeks: 14, months: 4 };

const ZOOM_KEY = 'taskcraft.planner.zoom';
const UNDATED_KEY = 'taskcraft.planner.undated';
const ZOOMS: Zoom[] = ['days', 'weeks', 'months'];
const MS_DAY = 86_400_000;

export function loadZoom(): Zoom {
  try {
    const v = localStorage.getItem(ZOOM_KEY);
    if (ZOOMS.includes(v as Zoom)) return v as Zoom;
  } catch { /* storage unavailable */ }
  return 'weeks';
}

export function saveZoom(z: Zoom) {
  try { localStorage.setItem(ZOOM_KEY, z); } catch { /* storage unavailable */ }
}

export function loadUndatedOpen(): boolean {
  try {
    const v = localStorage.getItem(UNDATED_KEY);
    if (v === 'true') return true;
    if (v === 'false') return false;
  } catch { /* storage unavailable */ }
  return true;
}

export function saveUndatedOpen(b: boolean) {
  try { localStorage.setItem(UNDATED_KEY, String(b)); } catch { /* storage unavailable */ }
}

/** Local calendar days since 1970-01-01 (DST-safe: built from local y/m/d). */
export const dayNumber = (d: Date): number => Math.round(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / MS_DAY);

/** Local midnight of a day number. */
export function dayDate(n: number): Date {
  const u = new Date(n * MS_DAY);
  return new Date(u.getUTCFullYear(), u.getUTCMonth(), u.getUTCDate());
}

export function barSpan(t: { start: string | null; due: string | null }): { from: number; to: number } | null {
  const s = t.start ? dayNumber(new Date(t.start)) : null;
  const d = t.due ? dayNumber(new Date(t.due)) : null;
  if (s === null && d === null) return null;
  if (s === null) return { from: d!, to: d! };
  if (d === null) return { from: s, to: s };
  return { from: Math.min(s, d), to: Math.max(s, d) };
}

const MIN_DAYS = 92;

export function timelineRange(
  tasks: { start: string | null; due: string | null }[],
  today: Date,
): { from: number; to: number } {
  const t = dayNumber(today);
  let lo = t, hi = t;
  for (const task of tasks) {
    const s = barSpan(task);
    if (!s) continue;
    lo = Math.min(lo, s.from);
    hi = Math.max(hi, s.to);
  }
  const from = lo - 14;
  let to = hi + 28;
  if (to - from + 1 < MIN_DAYS) to = from + MIN_DAYS - 1;
  return { from, to };
}

export const dayToX = (day: number, range: { from: number }, zoom: Zoom): number =>
  (day - range.from) * DAY_WIDTH[zoom];

export const xToDay = (x: number, range: { from: number }, zoom: Zoom): number =>
  range.from + Math.floor(x / DAY_WIDTH[zoom]);

type Dates = { start: string | null; startHasTime: boolean; due: string | null; dueHasTime: boolean };

/** The same time of day on another local calendar day. */
function onDay(iso: string, day: number): string {
  const d = new Date(iso);
  const t = dayDate(day);
  return new Date(t.getFullYear(), t.getMonth(), t.getDate(), d.getHours(), d.getMinutes(), d.getSeconds(), d.getMilliseconds()).toISOString();
}

const allDay = (day: number) => dayDate(day).toISOString();

export function dragResult(t: Dates, mode: DragMode, deltaDays: number): Dates {
  const out: Dates = { start: t.start, startHasTime: t.startHasTime, due: t.due, dueHasTime: t.dueHasTime };
  const sDay = t.start ? dayNumber(new Date(t.start)) : null;
  const dDay = t.due ? dayNumber(new Date(t.due)) : null;

  if (mode === 'move') {
    if (sDay !== null) out.start = onDay(t.start!, sDay + deltaDays);
    if (dDay !== null) out.due = onDay(t.due!, dDay + deltaDays);
  } else if (mode === 'start') {
    // Candidate start; if it lands after due as an instant, it takes due's exact instant and flag.
    const cand = sDay !== null ? onDay(t.start!, sDay + deltaDays) : dDay !== null ? allDay(dDay + deltaDays) : null;
    if (cand !== null) {
      const over = t.due !== null && new Date(cand).getTime() > new Date(t.due).getTime();
      out.start = over ? t.due : cand;
      out.startHasTime = over ? t.dueHasTime : sDay !== null ? t.startHasTime : false;
    }
  } else {
    // Candidate due; if it lands before start as an instant, it takes start's exact instant and flag.
    const cand = dDay !== null ? onDay(t.due!, dDay + deltaDays) : sDay !== null ? allDay(sDay + deltaDays) : null;
    if (cand !== null) {
      const under = t.start !== null && new Date(cand).getTime() < new Date(t.start).getTime();
      out.due = under ? t.start : cand;
      out.dueHasTime = under ? t.startHasTime : dDay !== null ? t.dueHasTime : false;
    }
  }
  return out;
}

export function placeUndated(day: number): { start: string; startHasTime: false; due: string; dueHasTime: false } {
  const iso = allDay(day);
  return { start: iso, startHasTime: false, due: iso, dueHasTime: false };
}

export function headerTicks(
  range: { from: number; to: number },
  zoom: Zoom,
  weekStart: 0 | 1,
): { day: number; label: string; major: boolean }[] {
  const ticks: { day: number; label: string; major: boolean }[] = [];
  for (let n = range.from; n <= range.to; n++) {
    const d = dayDate(n);
    const monthStart = d.getDate() === 1;
    if (zoom === 'days') {
      ticks.push({ day: n, label: monthStart ? monthTitle(d) : String(d.getDate()), major: monthStart });
    } else if (zoom === 'weeks') {
      if (d.getDay() !== weekStart) continue;
      const major = d.getDate() <= 7;
      ticks.push({ day: n, label: major ? monthTitle(d) : String(d.getDate()), major });
    } else if (monthStart) {
      ticks.push({ day: n, label: monthTitle(d), major: d.getMonth() === 0 });
    }
  }
  return ticks;
}
