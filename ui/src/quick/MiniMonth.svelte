<script lang="ts">
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { monthGrid, sameDay } from '../lib/layout';
  import { monthTitle, weekdayInitials, ymd } from '../lib/format';

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
</script>

<div class="mini">
  <div class="head">
    <button class="title" onclick={() => { month = new Date(today.getFullYear(), today.getMonth(), 1); onselect?.(null); }}>{monthTitle(month)}</button>
    <div>
      <button class="icon-btn" aria-label="Previous month" onclick={() => shift(-1)}><ChevronLeft size={15} /></button>
      <button class="icon-btn" aria-label="Next month" onclick={() => shift(1)}><ChevronRight size={15} /></button>
    </div>
  </div>
  <div class="grid">
    {#each weekdayInitials(weekStart) as w}<span class="wd">{w}</span>{/each}
    {#each days as d}
      <button
        class="day"
        class:other={d.getMonth() !== month.getMonth()}
        class:today={sameDay(d, today)}
        class:sel={selected && sameDay(d, selected)}
        onclick={() => onselect?.(selected && sameDay(d, selected) ? null : d)}
      >
        {d.getDate()}
        {#if marked.has(ymd(d))}<span class="mark"></span>{/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .mini { padding: 2px 4px; }
  .head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
  .title { border: none; background: none; font-weight: 600; font-size: 13px; padding: 2px 4px; border-radius: 5px; }
  .title:hover { background: var(--hover); }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px; text-align: center; }
  .wd { font-size: 10px; color: var(--faint); font-weight: 600; padding-bottom: 2px; }
  .day {
    position: relative;
    border: none;
    background: none;
    height: 26px;
    border-radius: 13px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .day:hover { background: var(--hover); }
  .other { color: var(--faint); }
  .today { color: var(--today); font-weight: 700; }
  .sel { background: var(--accent) !important; color: white; }
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
