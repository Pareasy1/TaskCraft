// Display formatting, using the system locale.
import type { ParsedItem, When } from './types';

const locale = navigator.language || 'en-US';

export const fmtTime = (d: Date) => d.toLocaleTimeString(locale, { hour: 'numeric', minute: '2-digit' });

export const fmtDayHeading = (d: Date, today = new Date()) => {
  const t = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const diff = Math.round((x.getTime() - t.getTime()) / 86_400_000);
  const base = d.toLocaleDateString(locale, { weekday: 'long', day: 'numeric', month: 'long' });
  if (diff === 0) return `Today · ${base}`;
  if (diff === 1) return `Tomorrow · ${base}`;
  if (diff === -1) return `Yesterday · ${base}`;
  return base;
};

export const fmtShortDate = (d: Date) =>
  d.toLocaleDateString(locale, { weekday: 'short', day: 'numeric', month: 'short' });

export const monthTitle = (d: Date) => d.toLocaleDateString(locale, { month: 'long', year: 'numeric' });

export function weekdayInitials(weekStart: 0 | 1): string[] {
  const base = new Date(2026, 9, 4 + weekStart); // a Sunday + weekStart
  return Array.from({ length: 7 }, (_, i) =>
    new Date(base.getFullYear(), base.getMonth(), base.getDate() + i).toLocaleDateString(locale, { weekday: 'narrow' }),
  );
}

/** Local Date from a parser `When`. */
export function whenToDate(w: When): Date {
  const [y, m, d] = w.date.split('-').map(Number);
  const [hh, mm] = (w.time ?? '00:00').split(':').map(Number);
  return new Date(y, m - 1, d, hh, mm);
}

export function describeParsed(p: ParsedItem): string {
  if (!p.start) return p.kind === 'task' ? 'No due date' : '';
  const s = whenToDate(p.start);
  if (p.kind === 'task') return `Due ${fmtShortDate(s)}${p.start.time ? `, ${fmtTime(s)}` : ''}`;
  if (p.allDay) {
    const e = p.end ? whenToDate(p.end) : s;
    return e.getTime() === s.getTime() ? `${fmtShortDate(s)} · all day` : `${fmtShortDate(s)} – ${fmtShortDate(e)} · all day`;
  }
  const e = p.end ? whenToDate(p.end) : s;
  const sameDay = e.toDateString() === s.toDateString();
  return sameDay
    ? `${fmtShortDate(s)}, ${fmtTime(s)} – ${fmtTime(e)}`
    : `${fmtShortDate(s)}, ${fmtTime(s)} – ${fmtShortDate(e)}, ${fmtTime(e)}`;
}

export function describeRecurrence(p: ParsedItem): string | null {
  const r = p.recurrence;
  if (!r) return null;
  const unit = { daily: 'day', weekly: 'week', monthly: 'month', yearly: 'year' }[r.rule.freq] ?? r.rule.freq;
  let s = r.rule.interval > 1 ? `Every ${r.rule.interval} ${unit}s` : `Every ${unit}`;
  if (r.rule.days?.length) s += ` on ${r.rule.days.join(', ')}`;
  if (r.rule.monthDay) s += ` on day ${r.rule.monthDay}`;
  if (r.until) s += ` until ${r.until}`;
  return s;
}

export const ymd = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

/** Converts a UTF-8 byte offset (from Rust) into a JS string index. */
export function byteToIndex(text: string, byte: number): number {
  let bytes = 0;
  let i = 0;
  for (const ch of text) {
    if (bytes >= byte) break;
    const cp = ch.codePointAt(0)!;
    bytes += cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4;
    i += ch.length;
  }
  return i;
}
