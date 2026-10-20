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

export interface Bar<T> {
  item: T;
  /** Week row (0–5) and starting column (0–6) in the 42-day grid. */
  row: number;
  col: number;
  span: number;
  /** Stacking position within the row; overlapping bars get different lanes. */
  lane: number;
  /** The event continues from the previous week / into the next one. */
  fromPrev: boolean;
  toNext: boolean;
}

/**
 * Month-view bars for all-day and multi-day events: one bar per week row the
 * event covers, stacked into lanes. Timed events within a single day are not
 * bars (the day cells list them).
 */
export function monthBars<T extends { start: string; end: string; allDay: boolean }>(items: T[], grid: Date[]): Bar<T>[] {
  const overlaps = (t: T, d: Date) => new Date(t.start) < addDays(d, 1) && new Date(t.end) > d;
  const bars: Bar<T>[] = [];
  for (const item of items) {
    const idx = grid.flatMap((d, i) => (overlaps(item, d) ? [i] : []));
    if (!idx.length || (!item.allDay && idx.length < 2)) continue;
    const first = idx[0];
    const last = idx[idx.length - 1];
    for (let row = Math.floor(first / 7); row <= Math.floor(last / 7); row++) {
      const a = Math.max(first, row * 7);
      const b = Math.min(last, row * 7 + 6);
      bars.push({ item, row, col: a % 7, span: b - a + 1, lane: 0, fromPrev: a > first, toNext: b < last });
    }
  }
  bars.sort((x, y) => x.row - y.row || x.col - y.col || y.span - x.span);
  // Greedy lanes per row: the lowest lane whose last bar ends before this one starts.
  let row = -1;
  let laneEnd: number[] = [];
  for (const b of bars) {
    if (b.row !== row) {
      row = b.row;
      laneEnd = [];
    }
    let lane = laneEnd.findIndex((end) => end < b.col);
    if (lane < 0) lane = laneEnd.length;
    laneEnd[lane] = b.col + b.span - 1;
    b.lane = lane;
  }
  return bars;
}

/**
 * Keyboard movement inside a calendar grid (WAI-ARIA grid pattern):
 * arrows move a day or a week, Home/End go to the week's first/last day,
 * PageUp/PageDown change month (clamped to its length). Null for other keys.
 */
export function gridKeyMove(d: Date, key: string, weekStart: 0 | 1): Date | null {
  const y = d.getFullYear();
  const m = d.getMonth();
  const day = d.getDate();
  const offset = (d.getDay() - weekStart + 7) % 7;
  const monthShift = (n: number) => {
    const last = new Date(y, m + n + 1, 0).getDate();
    return new Date(y, m + n, Math.min(day, last));
  };
  switch (key) {
    case 'ArrowLeft': return new Date(y, m, day - 1);
    case 'ArrowRight': return new Date(y, m, day + 1);
    case 'ArrowUp': return new Date(y, m, day - 7);
    case 'ArrowDown': return new Date(y, m, day + 7);
    case 'Home': return new Date(y, m, day - offset);
    case 'End': return new Date(y, m, day - offset + 6);
    case 'PageUp': return monthShift(-1);
    case 'PageDown': return monthShift(1);
    default: return null;
  }
}
