<script lang="ts" module>
  export interface Bar { id: string; label: string; n: number; color: string; pressed?: boolean }
</script>

<script lang="ts">
  // A horizontal bar chart: label, bar, value at the tip. Each bar is a toggle button that filters the table.
  import { t } from '../../lib/i18n';
  import { barLengths, fmtCount } from '../../lib/charts';

  let { title, dimension, bars, onpick }: {
    title: string;
    /** The column header for the text alternative ("Status", "Bucket"…). */
    dimension: string;
    bars: Bar[];
    onpick: (id: string) => void;
  } = $props();

  const LABEL_W = 96;
  const VALUE_ROOM = 32;
  const H = 12;
  /** Row padding: the hover and pressed background reach past the label and the value. */
  const PAD_X = 4;
  let width = $state(0);
  let lengths = $derived(barLengths(bars.map((b) => b.n), Math.max(0, width - 2 * PAD_X - LABEL_W - 8 - VALUE_ROOM)));
  let anyPressed = $derived(bars.some((b) => b.pressed));

  /** A bar square at the baseline with a 4px rounded data end. */
  function barPath(len: number): string {
    const r = Math.min(4, len / 2, H / 2);
    return `M0,0H${len - r}A${r},${r} 0 0 1 ${len},${r}V${H - r}A${r},${r} 0 0 1 ${len - r},${H}H0Z`;
  }
</script>

<figure class="chart">
  <figcaption>{title}</figcaption>
  <div class="rows" bind:clientWidth={width}>
    {#each bars as b, i (b.id)}
      <button type="button" class="row" class:on={b.pressed} class:dim={anyPressed && !b.pressed}
        aria-pressed={!!b.pressed} aria-label={t('charts.bar', { label: b.label, n: fmtCount(b.n) })} onclick={() => onpick(b.id)}>
        <span class="name" title={b.label} style:width="{LABEL_W}px">{b.label}</span>
        <svg class="track" height={H} aria-hidden="true">
          {#if lengths[i] > 0}<path d={barPath(lengths[i])} style:fill={b.color} />{/if}
          <text x={lengths[i] + 6} y={H / 2} dominant-baseline="central">{fmtCount(b.n)}</text>
        </svg>
      </button>
    {/each}
  </div>
  <table class="sr">
    <caption>{title}</caption>
    <thead><tr><th scope="col">{dimension}</th><th scope="col">{t('charts.tasks')}</th></tr></thead>
    <tbody>
      {#each bars as b (b.id)}<tr><th scope="row">{b.label}</th><td>{fmtCount(b.n)}</td></tr>{/each}
    </tbody>
  </table>
</figure>

<style>
  .chart { margin: 0; min-width: 0; }
  figcaption { font-size: 12px; font-weight: 600; color: var(--muted); margin-bottom: 6px; padding-left: 4px; }
  .rows { display: flex; flex-direction: column; gap: 2px; }
  .row { display: flex; align-items: center; gap: 8px; width: 100%; box-sizing: border-box; padding: 3px 4px; margin: 0; border: none; background: none; border-radius: 6px; color: var(--text); font: inherit; text-align: left; }
  .row { cursor: pointer; }
  .row:hover { background: var(--hover); }
  .row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .row.on { background: var(--accent-soft); }
  .row.dim .track path { opacity: 0.35; }
  .name { flex: none; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .track { flex: 1; min-width: 0; overflow: visible; }
  text { font-size: 11px; fill: var(--muted); font-variant-numeric: tabular-nums; }
  .row.on text { fill: var(--text); font-weight: 600; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
</style>
