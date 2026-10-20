<script lang="ts">
  import { t } from '../lib/i18n';
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { gridKeyMove, monthGrid, sameDay } from '../lib/layout';
  import { dayLabel, monthTitle, weekdayInitials, ymd } from '../lib/format';

  let {
    month = $bindable(new Date()),
    selected = null,
    marked = new Set<string>(),
    weekStart = 1,
    onselect,
  }: {
    month?: Date;
    selected?: Date | null;
    marked?: Set<string>;
    weekStart?: 0 | 1;
    onselect?: (d: Date | null) => void;
  } = $props();

  let days = $derived(monthGrid(month.getFullYear(), month.getMonth(), weekStart));
  const today = new Date();
  const shift = (n: number) => (month = new Date(month.getFullYear(), month.getMonth() + n, 1));

  // One Tab stop for the whole grid; arrows move between days (roving tabindex).
  let gridEl: HTMLDivElement | undefined = $state();
  let cursor = $state<Date | null>(null);
  const inMonth = (d: Date) => d.getFullYear() === month.getFullYear() && d.getMonth() === month.getMonth();
  let focusDay = $derived(
    cursor && inMonth(cursor) ? cursor : selected && inMonth(selected) ? selected : inMonth(today) ? today : new Date(month.getFullYear(), month.getMonth(), 1),
  );
  let weeks = $derived(Array.from({ length: 6 }, (_, w) => days.slice(w * 7, w * 7 + 7)));

  async function keys(e: KeyboardEvent) {
    const next = gridKeyMove(focusDay, e.key, weekStart);
    if (!next) return;
    e.preventDefault();
    if (!inMonth(next)) month = new Date(next.getFullYear(), next.getMonth(), 1);
    cursor = next;
    await tick();
    gridEl?.querySelector<HTMLButtonElement>(`[data-day="${ymd(next)}"]`)?.focus();
  }
</script>

<div class="mini">
  <div class="head">
    <button class="title" onclick={() => { month = new Date(today.getFullYear(), today.getMonth(), 1); onselect?.(null); }}>{monthTitle(month)}</button>
    <div>
      <button class="icon-btn" aria-label={t('common.previousMonth')} onclick={() => shift(-1)}><ChevronLeft size={15} /></button>
      <button class="icon-btn" aria-label={t('common.nextMonth')} onclick={() => shift(1)}><ChevronRight size={15} /></button>
    </div>
  </div>
  <div class="grid" role="grid" aria-label={monthTitle(month)} tabindex="-1" bind:this={gridEl} onkeydown={keys}>
    <div class="row" role="row">
      {#each weekdayInitials(weekStart) as w}<span class="wd" role="columnheader" aria-hidden="true">{w}</span>{/each}
    </div>
    {#each weeks as week}
      <div class="row" role="row">
        {#each week as d (d.getTime())}
          <div class="cell" role="gridcell" aria-selected={!!selected && sameDay(d, selected)}>
            <button
              class="day"
              class:other={d.getMonth() !== month.getMonth()}
              class:today={sameDay(d, today)}
              class:sel={selected && sameDay(d, selected)}
              tabindex={sameDay(d, focusDay) ? 0 : -1}
              data-day={ymd(d)}
              aria-label={dayLabel(d, today, marked.has(ymd(d)))}
              aria-current={sameDay(d, today) ? 'date' : undefined}
              onclick={() => { cursor = d; onselect?.(selected && sameDay(d, selected) ? null : d); }}
            >
              {d.getDate()}
              {#if marked.has(ymd(d))}<span class="mark"></span>{/if}
            </button>
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .mini { padding: 2px 4px; }
  .head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
  .title { border: none; background: none; font-weight: 600; font-size: 13px; padding: 2px 4px; border-radius: 5px; }
  .title:hover { background: var(--hover); }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px; text-align: center; }
  .wd { font-size: 12px; color: var(--faint); font-weight: 600; padding-bottom: 2px; }
  .day {
    position: relative;
    border: none;
    background: none;
    height: 26px;
    border-radius: 13px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .row, .cell { display: contents; }
  .day:hover { background: var(--hover); }
  .other { color: var(--faint); }
  .today { color: var(--today); font-weight: 700; }
  .sel { background: var(--accent) !important; color: var(--on-accent); }
  /* Keep "today" visible when it is also the selected day. */
  .sel.today { box-shadow: inset 0 0 0 2px var(--today); }
  .mark {
    position: absolute;
    bottom: 2px;
    left: 50%;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.55;
  }
</style>
