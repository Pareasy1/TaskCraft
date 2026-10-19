<script lang="ts">
  import type { Occurrence } from '../lib/types';
  import { addDays, monthGrid, sameDay } from '../lib/layout';
  import { fmtTime } from '../lib/format';

  let {
    month,
    weekStart,
    occurrences,
    onselect,
    onday,
  }: {
    month: Date;
    weekStart: 0 | 1;
    occurrences: Occurrence[];
    onselect: (o: Occurrence) => void;
    onday: (d: Date) => void;
  } = $props();

  let days = $derived(monthGrid(month.getFullYear(), month.getMonth(), weekStart));
  const today = new Date();
  const itemsOn = (d: Date) => occurrences.filter((o) => new Date(o.start) < addDays(d, 1) && new Date(o.end) > d);
  const names = $derived(
    Array.from({ length: 7 }, (_, i) => days[i].toLocaleDateString(undefined, { weekday: 'short' })),
  );
</script>

<div class="month">
  <div class="names">{#each names as n}<span>{n}</span>{/each}</div>
  <div class="cells">
    {#each days as d (d.getTime())}
      {@const items = itemsOn(d)}
      <div class="cell" class:other={d.getMonth() !== month.getMonth()} role="gridcell" tabindex="-1" ondblclick={() => onday(d)}>
        <button class="num" class:today={sameDay(d, today)} onclick={() => onday(d)}>{d.getDate()}</button>
        {#each items.slice(0, 3) as o (o.eventId + o.start)}
          <button class="item" style="--c:{o.color}" class:allday={o.allDay} onclick={() => onselect(o)}>
            {#if !o.allDay}<span class="dot" style="background:{o.color}"></span>{/if}
            <span class="t">{o.title}</span>
            {#if !o.allDay}<span class="time">{fmtTime(new Date(o.start))}</span>{/if}
          </button>
        {/each}
        {#if items.length > 3}<button class="more" onclick={() => onday(d)}>+{items.length - 3} more</button>{/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .month { display: flex; flex-direction: column; height: 100%; }
  .names { display: grid; grid-template-columns: repeat(7, 1fr); border-bottom: 1px solid var(--border); }
  .names span { text-align: right; padding: 5px 8px; font-size: 12px; color: var(--muted); }
  .cells { flex: 1; display: grid; grid-template-columns: repeat(7, 1fr); grid-template-rows: repeat(6, 1fr); }
  .cell {
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    padding: 3px 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    overflow: hidden;
  }
  .cell.other { background: var(--bg-sunken); }
  .cell.other .num { color: var(--faint); }
  .num {
    align-self: flex-end;
    border: none;
    background: none;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    border-radius: 11px;
    min-width: 22px;
    padding: 1px 5px;
  }
  .num.today { background: var(--today); color: white; font-weight: 700; }
  .item {
    display: flex;
    align-items: center;
    gap: 4px;
    border: none;
    background: none;
    border-radius: 4px;
    padding: 1px 4px;
    font-size: 11.5px;
    text-align: left;
    min-width: 0;
  }
  .item:hover { background: var(--hover); }
  .item.allday { background: color-mix(in srgb, var(--c) 20%, var(--bg)); color: color-mix(in srgb, var(--c) 70%, var(--text)); }
  .t { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .time { color: var(--muted); font-size: 10.5px; }
  .more { border: none; background: none; font-size: 11px; color: var(--muted); text-align: left; padding: 0 4px; }
</style>
