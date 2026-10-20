// Pure date/layout helpers shared by the calendar views.

/** 42 consecutive days (6 weeks) covering `month` (0–11), starting on `weekStart` (0 = Sun, 1 = Mon). */
export function monthGrid(year: number, month: number, weekStart: 0 | 1): Date[] {
  const first = new Date(year, month, 1);
  const offset = (first.getDay() - weekStart + 7) % 7;
  // Build with the Date constructor (not +24h steps) so DST changes can't skip or repeat a day.
  return Array.from({ length: 42 }, (_, i) => new Date(year, month, 1 - offset + i));
}

export function sameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

export function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n, d.getHours(), d.getMinutes());
}

export interface Positioned {
  id: string;
  /** Minutes from the start of the day. */
  top: number;
  height: number;
  col: number;
  cols: number;
}

const MIN_HEIGHT = 15;

/** Lays out timed items for one day: overlapping items (transitively) share the width. */
export function layoutDay(items: { id: string; start: Date; end: Date }[], dayStart: Date): Positioned[] {
  const dayEnd = addDays(dayStart, 1);
  const minutes = (d: Date) => (d.getTime() - dayStart.getTime()) / 60_000;
  const dayLen = minutes(dayEnd);
  const rows = items
    .map((it) => {
      const top = Math.max(0, minutes(it.start));
      const bottom = Math.min(dayLen, minutes(it.end));
      return { id: it.id, top, height: Math.max(MIN_HEIGHT, bottom - top) };
    })
    .sort((a, b) => a.top - b.top || b.height - a.height);

  const out: Positioned[] = [];
  let cluster: (Positioned & { bottom: number })[] = [];
  let clusterEnd = -Infinity;
  const flush = () => {
    const cols = Math.max(0, ...cluster.map((c) => c.col)) + 1;
    for (const c of cluster) out.push({ id: c.id, top: c.top, height: c.height, col: c.col, cols });
    cluster = [];
  };
  for (const r of rows) {
    if (r.top >= clusterEnd && cluster.length) flush();
    const bottom = r.top + r.height;
    const used = new Set(cluster.filter((c) => c.bottom > r.top).map((c) => c.col));
    let col = 0;
    while (used.has(col)) col++;
    cluster.push({ ...r, col, cols: 1, bottom });
    clusterEnd = Math.max(cluster.length === 1 ? -Infinity : clusterEnd, bottom);
  }
  if (cluster.length) flush();
  return out;
}

/** The 7 days of the week containing `anchor`. */
export function weekDays(anchor: Date, weekStart: 0 | 1): Date[] {
  const offset = (anchor.getDay() - weekStart + 7) % 7;
  return Array.from({ length: 7 }, (_, i) => new Date(anchor.getFullYear(), anchor.getMonth(), anchor.getDate() - offset + i));
}

/** Rounds minutes to the nearest 15, never below 0. */
export function snap15(minutes: number): number {
  return Math.max(0, Math.round(minutes / 15) * 15) || 0;
}

/** Events overlapping each day and tasks due on it, for `days` days from `from`; empty days are omitted. */
export function dayGroups<E extends { start: string; end: string }, T extends { due: string | null }>(
  events: E[],
  tasks: T[],
  from: Date,
  days: number,
): { day: Date; events: E[]; tasks: T[] }[] {
  const out: { day: Date; events: E[]; tasks: T[] }[] = [];
  for (let i = 0; i < days; i++) {
    const day = addDays(startOfDay(from), i);
    const next = addDays(day, 1);
    const ev = events.filter((o) => new Date(o.start) < next && new Date(o.end) > day);
    const tk = tasks.filter((t) => t.due && new Date(t.due) >= day && new Date(t.due) < next);
    if (ev.length || tk.length) out.push({ day, events: ev, tasks: tk });
  }
  return out;
}

/**
 * Keyboard move/resize for a timed event: ↑/↓ move 15 minutes, Shift+↑/↓
 * change the length (at least 15 minutes), ←/→ move a day at the same local time.
 */
export function nudgeEvent(start: Date, end: Date, key: string, shift: boolean): { start: Date; end: Date } | null {
  const STEP = 15 * 60_000;
  const len = end.getTime() - start.getTime();
  switch (key) {
    case 'ArrowUp':
    case 'ArrowDown': {
      const d = key === 'ArrowUp' ? -STEP : STEP;
      if (shift) return { start, end: new Date(Math.max(start.getTime() + STEP, end.getTime() + d)) };
      return { start: new Date(start.getTime() + d), end: new Date(end.getTime() + d) };
    }
    case 'ArrowLeft':
    case 'ArrowRight': {
      const ns = addDays(start, key === 'ArrowLeft' ? -1 : 1);
      const ne = addDays(end, key === 'ArrowLeft' ? -1 : 1);
      return { start: ns, end: ne.getTime() > ns.getTime() ? ne : new Date(ns.getTime() + len) };
    }
    default:
      return null;
  }
}
