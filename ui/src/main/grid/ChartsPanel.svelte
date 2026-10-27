<script lang="ts">
  // Above the grid: four stat tiles and four charts for the tasks shown (before the chart filter).
  import { ChevronDown } from 'lucide-svelte';
  import { dateLocale, t } from '../../lib/i18n';
  import { barLengths, loadChartsOpen, saveChartsOpen } from '../../lib/charts';
  import type { ChartFilter } from '../../lib/grid';
  import type { GridStats } from '../../lib/gridStats';
  import { STATUS_LABEL } from '../../lib/tasks';
  import type { Priority, TaskStatus } from '../../lib/types';
  import BarChart, { type Bar } from './BarChart.svelte';
  import StatTile from './StatTile.svelte';

  let { stats, inPlan, empty, chart, onchart }: {
    stats: GridStats;
    /** Inside a plan the groups are buckets; in All tasks they are plans. */
    inPlan: boolean;
    /** The scope has no tasks at all. */
    empty: boolean;
    chart: ChartFilter | null;
    onchart: (f: ChartFilter) => void;
  } = $props();

  let open = $state(loadChartsOpen());
  function toggle() {
    open = !open;
    saveChartsOpen(open);
  }

  const STATUS_COLOR: Record<TaskStatus, string> = {
    todo: 'var(--chart-neutral)', inProgress: 'var(--status-progress)', done: 'var(--status-done)',
  };
  const PRIORITY_COLOR: Record<Priority, string> = {
    urgent: 'var(--danger)', important: 'var(--warning)', medium: 'var(--chart-neutral)', low: 'var(--chart-neutral)',
  };
  const on = (kind: ChartFilter['kind'], value: string | number) => chart?.kind === kind && String(chart.value) === String(value);

  let statusBars = $derived<Bar[]>(stats.byStatus.map((s) => ({
    id: s.key, label: STATUS_LABEL[s.key], n: s.n, color: STATUS_COLOR[s.key], pressed: on('status', s.key),
  })));
  let priorityBars = $derived<Bar[]>(stats.byPriority.map((p) => ({
    id: p.key, label: t(`priority.${p.key}`), n: p.n, color: PRIORITY_COLOR[p.key], pressed: on('priority', p.key),
  })));
  let groupBars = $derived<Bar[]>(stats.byGroup.map((g) => ({
    id: String(g.key), label: g.label, n: g.n, color: g.color ?? 'var(--chart-neutral)', pressed: on('group', g.key),
  })));

  // ── Completed per week: columns, value on the cap, not interactive ──
  const PLOT_H = 72, TOP = 16, BOTTOM = 20, COL_MAX = 24;
  let weekW = $state(0);
  let band = $derived(weekW / Math.max(1, stats.perWeek.length));
  let colW = $derived(Math.max(4, Math.min(COL_MAX, band * 0.6)));
  let heights = $derived(barLengths(stats.perWeek.map((w) => w.n), PLOT_H));
  const tick = (d: Date) => d.toLocaleDateString(dateLocale, { day: 'numeric', month: 'numeric' });
  const longWeek = (d: Date) => d.toLocaleDateString(dateLocale, { day: 'numeric', month: 'short', year: 'numeric' });
  /** A column square at the baseline with a 4px rounded top. */
  function colPath(x: number, h: number): string {
    const r = Math.min(4, h / 2, colW / 2);
    const y = TOP + PLOT_H - h, b = TOP + PLOT_H;
    return `M${x},${b}V${y + r}A${r},${r} 0 0 1 ${x + r},${y}H${x + colW - r}A${r},${r} 0 0 1 ${x + colW},${y + r}V${b}Z`;
  }
</script>

<section class="panel" aria-label={t('charts.label')}>
  <button class="toggle" aria-expanded={open} aria-controls="grid-charts" onclick={toggle}>
    <ChevronDown size={13} aria-hidden="true" class={open ? '' : 'closed'} />{t('charts.toggle')}
  </button>
  {#if open}
    <div id="grid-charts" class="body">
      {#if empty}
        <p class="empty">{t('charts.empty')}</p>
      {:else}
        <div class="tiles">
          <StatTile label={t('charts.open')} value={stats.open} />
          <StatTile label={t('charts.inProgress')} value={stats.inProgress} />
          <StatTile label={t('charts.overdue')} value={stats.overdue} alert />
          <StatTile label={t('charts.completed7d')} value={stats.completed7d} />
        </div>
        <div class="charts">
          <BarChart title={t('charts.byStatus')} dimension={t('table.col.status')} bars={statusBars}
            onpick={(id) => onchart({ kind: 'status', value: id as TaskStatus })} />
          <BarChart title={t('charts.byPriority')} dimension={t('table.col.priority')} bars={priorityBars}
            onpick={(id) => onchart({ kind: 'priority', value: id as Priority })} />
          <BarChart title={inPlan ? t('charts.byBucket') : t('charts.byPlan')} dimension={inPlan ? t('table.col.bucket') : t('table.col.plan')}
            bars={groupBars} onpick={(id) => onchart({ kind: 'group', value: Number(id) })} />
          <figure class="week">
            <figcaption>{t('charts.perWeek')}</figcaption>
            <div bind:clientWidth={weekW}>
              <svg width={weekW} height={TOP + PLOT_H + BOTTOM} aria-hidden="true">
                <line class="base" x1="0" x2={weekW} y1={TOP + PLOT_H + 0.5} y2={TOP + PLOT_H + 0.5} />
                {#each stats.perWeek as w, i (w.weekStart.getTime())}
                  {@const x = i * band + (band - colW) / 2}
                  {#if heights[i] > 0}<path d={colPath(x, heights[i])} />{/if}
                  <text class="val" x={x + colW / 2} y={TOP + PLOT_H - heights[i] - 4} text-anchor="middle">{w.n}</text>
                  <text class="tick" x={x + colW / 2} y={TOP + PLOT_H + 14} text-anchor="middle">{tick(w.weekStart)}</text>
                {/each}
              </svg>
            </div>
            <table class="sr">
              <caption>{t('charts.perWeek')}</caption>
              <thead><tr><th scope="col">{t('charts.week')}</th><th scope="col">{t('charts.completed')}</th></tr></thead>
              <tbody>
                {#each stats.perWeek as w (w.weekStart.getTime())}<tr><th scope="row">{longWeek(w.weekStart)}</th><td>{w.n}</td></tr>{/each}
              </tbody>
            </table>
          </figure>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel { padding-bottom: 10px; }
  .toggle { display: inline-flex; align-items: center; gap: 4px; border: none; background: none; padding: 2px 6px 2px 2px; margin-left: -2px; border-radius: 6px; color: var(--muted); font-size: 12px; font-weight: 600; }
  .toggle:hover { background: var(--hover); color: var(--text); }
  .toggle:focus-visible { outline: 2px solid var(--accent); }
  .toggle :global(.closed) { transform: rotate(-90deg); }
  .body { padding-top: 8px; max-height: 45vh; overflow: auto; }
  .empty { margin: 0; padding: 8px 0; color: var(--muted); font-size: 13px; }
  .tiles { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; margin-bottom: 14px; }
  .charts { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px 24px; }
  .week { margin: 0; min-width: 0; }
  .week figcaption { font-size: 12px; font-weight: 600; color: var(--muted); margin-bottom: 6px; }
  .week svg { display: block; overflow: visible; }
  .week path { fill: var(--status-done); }
  .base { stroke: var(--border); stroke-width: 1; }
  .val { font-size: 11px; fill: var(--muted); font-variant-numeric: tabular-nums; }
  .tick { font-size: 10px; fill: var(--faint); font-variant-numeric: tabular-nums; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  @media (max-width: 720px) {
    .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
