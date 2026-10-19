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
