// Pure helpers for the Grid charts panel: bar scaling and the collapse state.

/** Shortest bar drawn for a non-zero value, so a 1 next to a 1000 still shows. */
const MIN_BAR = 4;

/** Bar lengths for `values`, the largest filling `max`. All zero when every value is zero. */
export function barLengths(values: number[], max: number): number[] {
  const top = Math.max(0, ...values);
  if (top <= 0 || max <= 0) return values.map(() => 0);
  return values.map((v) => (v <= 0 ? 0 : Math.min(max, Math.max(MIN_BAR, (v / top) * max))));
}

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
