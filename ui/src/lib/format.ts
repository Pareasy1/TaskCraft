// Display formatting in the app language (see i18n.ts).
import type { ParsedItem, When } from './types';
import { dateLocale, t } from './i18n';

const locale = dateLocale;

export const fmtTime = (d: Date) => d.toLocaleTimeString(locale, { hour: 'numeric', minute: '2-digit' });

export const fmtDayHeading = (d: Date, today = new Date()) => {
  const t = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const diff = Math.round((x.getTime() - t.getTime()) / 86_400_000);
  const base = d.toLocaleDateString(locale, { weekday: 'long', day: 'numeric', month: 'long' });
  if (Math.abs(diff) > 1) return base;
  const rel = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' }).format(diff, 'day');
  return `${rel.charAt(0).toUpperCase()}${rel.slice(1)} · ${base}`;
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
  if (!p.start) return p.kind === 'task' ? t('parsed.noDue') : '';
  const s = whenToDate(p.start);
  if (p.kind === 'task') return t('parsed.due', { when: `${fmtShortDate(s)}${p.start.time ? `, ${fmtTime(s)}` : ''}` });
  if (p.allDay) {
    const e = p.end ? whenToDate(p.end) : s;
    const allDay = t('common.allDay');
    return e.getTime() === s.getTime() ? `${fmtShortDate(s)} · ${allDay}` : `${fmtShortDate(s)} – ${fmtShortDate(e)} · ${allDay}`;
  }
  const e = p.end ? whenToDate(p.end) : s;
  const sameDay = e.toDateString() === s.toDateString();
  return sameDay
    ? `${fmtShortDate(s)}, ${fmtTime(s)} – ${fmtTime(e)}`
    : `${fmtShortDate(s)}, ${fmtTime(s)} – ${fmtShortDate(e)}, ${fmtTime(e)}`;
}

/** Weekday name from the parser's three-letter code ("Mon"). */
function weekdayName(code: string): string {
  const i = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].indexOf(code);
  return i < 0 ? code : new Date(2026, 9, 4 + i).toLocaleDateString(locale, { weekday: 'long' });
}

export function describeRecurrence(p: ParsedItem): string | null {
  const r = p.recurrence;
  if (!r) return null;
  const unit = ({ daily: 'day', weekly: 'week', monthly: 'month', yearly: 'year' } as const)[r.rule.freq as 'daily'] ?? 'day';
  let s = r.rule.interval > 1
    ? t('repeat.everyN', { n: r.rule.interval, unit: t(`unit.${unit}s`) })
    : t('repeat.every', { unit: t(`unit.${unit}`) });
  if (r.rule.days?.length) s = t('repeat.onDays', { rule: s, days: r.rule.days.map(weekdayName).join(', ') });
  if (r.rule.monthDay) s = t('repeat.onDay', { rule: s, day: r.rule.monthDay });
  if (r.until) s = t('repeat.until', { rule: s, date: fmtShortDate(new Date(`${r.until}T00:00`)) });
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
