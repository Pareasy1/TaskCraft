<script lang="ts">
  import { dayDate, dayToX, headerTicks, DAY_WIDTH, type Zoom } from '../../lib/timeline';
  import { fmtMonthShort, monthTitle } from '../../lib/format';

  // The sticky date header: months (years when zoomed out) above, the zoom's ticks below.
  let { range, zoom, weekStart, today, nameW, height }: {
    range: { from: number; to: number };
    zoom: Zoom;
    weekStart: 0 | 1;
    /** Today's day number. */
    today: number;
    nameW: number;
    height: number;
  } = $props();

  let w = $derived(DAY_WIDTH[zoom]);
  let width = $derived((range.to - range.from + 1) * w);
  let ticks = $derived(headerTicks(range, zoom, weekStart).map((tk) => {
    const d = dayDate(tk.day);
    return { x: dayToX(tk.day, range, zoom), major: tk.major, label: zoom === 'months' ? fmtMonthShort(d) : String(d.getDate()) };
  }));
  // The upper line: each month (each year in Months), plus the period the range starts in.
  let spans = $derived.by(() => {
    const out: { x: number; label: string }[] = [];
    for (let n = range.from; n <= range.to; n++) {
      const d = dayDate(n);
      const starts = zoom === 'months' ? d.getMonth() === 0 && d.getDate() === 1 : d.getDate() === 1;
      if (n === range.from || starts) out.push({ x: dayToX(n, range, zoom), label: zoom === 'months' ? String(d.getFullYear()) : monthTitle(d) });
    }
    return out;
  });
  let todayX = $derived(dayToX(today, range, zoom) + w / 2);
</script>

<div class="header" style="height:{height}px; width:{nameW + width}px">
  <div class="corner" style="width:{nameW}px"></div>
  <div class="scale" style="left:{nameW}px; width:{width}px" aria-hidden="true">
    {#each spans as s (s.x)}
      <span class="span" style="left:{s.x}px">{s.label}</span>
    {/each}
    {#each ticks as tk (tk.x)}
      <span class="tick" class:major={tk.major} style="left:{tk.x}px; {zoom === 'days' ? `width:${w}px` : ''}" class:day={zoom === 'days'}>{tk.label}</span>
    {/each}
    <span class="now" style="left:{todayX - 1}px"></span>
  </div>
</div>

<style>
  .header { position: sticky; top: 0; z-index: 4; background: var(--bg); border-bottom: 1px solid var(--border); }
  .corner { position: sticky; left: 0; z-index: 5; height: 100%; background: var(--bg); border-right: 1px solid var(--border); }
  .scale { position: absolute; top: 0; bottom: 0; overflow: hidden; }
  .span { position: absolute; top: 3px; padding-left: 4px; font-size: 11.5px; font-weight: 600; color: var(--text); white-space: nowrap; border-left: 1px solid var(--border); line-height: 16px; }
  .tick { position: absolute; bottom: 2px; padding-left: 3px; font-size: 11px; color: var(--muted); white-space: nowrap; line-height: 16px; border-left: 1px solid var(--border); font-variant-numeric: tabular-nums; }
  .tick.day { padding-left: 0; text-align: center; border-left: none; }
  .tick.major { color: var(--text); font-weight: 600; }
  .now { position: absolute; bottom: 0; width: 2px; height: 6px; background: var(--today); border-radius: 1px; }
</style>
