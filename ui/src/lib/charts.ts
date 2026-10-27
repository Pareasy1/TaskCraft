// Pure helpers for the Grid charts panel: bar scaling, tick thinning, counts and the collapse state.
import { dateLocale } from './i18n';

/** Shortest bar drawn for a non-zero value, so a 1 next to a 1000 still shows. */
const MIN_BAR = 4;

/** Bar lengths for `values`, the largest filling `max`. All zero when every value is zero. */
export function barLengths(values: number[], max: number): number[] {
  const top = Math.max(0, ...values);
  if (top <= 0 || max <= 0) return values.map(() => 0);
  return values.map((v) => (v <= 0 ? 0 : Math.min(max, Math.max(MIN_BAR, (v / top) * max))));
}

/** Width one weekly tick label ("26/10", "10/26") needs to stay clear of its neighbours. */
const TICK_W = 36;

/**
 * Which of `n` evenly spaced tick labels fit in `width`: counted back from the last (the current
 * week), every `step`-th one; the first is added when it is a full step from the next shown one.
 */
export function shownTicks(n: number, width: number): number[] {
  if (n <= 0 || width <= 0) return [];
  const step = Math.max(1, Math.ceil(TICK_W / (width / n)));
  const shown: number[] = [];
  for (let i = n - 1; i >= 0; i -= step) shown.unshift(i);
  if (shown[0] !== 0 && shown[0] >= step) shown.unshift(0);
  return shown;
}

const counts = new Intl.NumberFormat(dateLocale);
/** A count in the app language ("1,284", "1.284"). */
export const fmtCount = (n: number): string => counts.format(n);

const KEY = 'taskcraft.grid.charts';

/** Whether the charts panel is expanded (the default). */
export function loadChartsOpen(): boolean {
  try {
    return localStorage.getItem(KEY) !== 'collapsed';
  } catch {
    return true;
  }
}

export function saveChartsOpen(open: boolean): void {
  try {
    localStorage.setItem(KEY, open ? 'open' : 'collapsed');
  } catch { /* storage unavailable */ }
}
