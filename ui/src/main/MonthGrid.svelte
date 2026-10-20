<script lang="ts">
  import { t } from '../lib/i18n';
  import type { Occurrence } from '../lib/types';
  import { addDays, monthBars, monthGrid, sameDay } from '../lib/layout';
  import { fmtTime, fmtWeekdayShort } from '../lib/format';

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
  // All-day and multi-day events are drawn as bars across the days they cover.
  const LANES = 3;
  let bars = $derived(monthBars(occurrences, days));
  let barItems = $derived(new Set(bars.map((b) => b.item)));
  let rowLanes = $derived(
    Array.from({ length: 6 }, (_, r) => Math.min(LANES, Math.max(0, ...bars.filter((b) => b.row === r).map((b) => b.lane + 1)))),
  );
  const timedOn = (d: Date) => itemsOn(d).filter((o) => !barItems.has(o));
  const hiddenBarsAt = (i: number) =>
    bars.filter((b) => b.lane >= LANES && b.row === Math.floor(i / 7) && b.col <= i % 7 && i % 7 < b.col + b.span).length;
  const names = $derived(
    Array.from({ length: 7 }, (_, i) => fmtWeekdayShort(days[i])),
  );
</script>

<div class="month">
  <div class="names">{#each names as n}<span>{n}</span>{/each}</div>
  <div class="cells">
    {#each days as d, i (d.getTime())}
      {@const items = timedOn(d)}
      {@const room = LANES - rowLanes[Math.floor(i / 7)]}
      {@const more = Math.max(0, items.length - room) + hiddenBarsAt(i)}
      <div class="cell" class:other={d.getMonth() !== month.getMonth()} role="gridcell" tabindex="-1"
        style="grid-row:{Math.floor(i / 7) + 1}; grid-column:{(i % 7) + 1}" ondblclick={() => onday(d)}>
        <button class="num" class:today={sameDay(d, today)} onclick={() => onday(d)}>{d.getDate()}</button>
        <div class="items" style="margin-top:{rowLanes[Math.floor(i / 7)] * 20}px">
          {#each items.slice(0, room) as o (o.eventId + o.start)}
            <button class="item" style="--c:{o.color}" onclick={() => onselect(o)}>
              <span class="dot" style="background:{o.color}"></span>
              <span class="t">{o.title}</span>
              <span class="time">{fmtTime(new Date(o.start))}</span>
            </button>
          {/each}
          {#if more}<button class="more" onclick={() => onday(d)}>{t('common.more', { n: more })}</button>{/if}
        </div>
      </div>
    {/each}
    {#each bars.filter((b) => b.lane < LANES) as b (b.item.eventId + b.item.start + b.row)}
      <button class="bar" class:from-prev={b.fromPrev} class:to-next={b.toNext}
        style="--c:{b.item.color}; grid-row:{b.row + 1}; grid-column:{b.col + 1} / span {b.span}; margin-top:{26 + b.lane * 20}px"
        onclick={() => onselect(b.item)}>
        <span class="t">{b.item.title}</span>
        {#if !b.item.allDay && !b.fromPrev}<span class="time">{fmtTime(new Date(b.item.start))}</span>{/if}
      </button>
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
  .num.today { background: var(--today); color: var(--on-today); font-weight: 700; }
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
  .items { display: flex; flex-direction: column; gap: 1px; min-height: 0; }
  .bar {
    align-self: start;
    z-index: 1;
    height: 18px;
    margin-left: 3px;
    margin-right: 3px;
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    border: none;
    border-radius: 4px;
    padding: 0 6px;
    font-size: 11.5px;
    text-align: left;
    background: color-mix(in srgb, var(--c) 20%, var(--bg));
    color: color-mix(in srgb, var(--c) 40%, var(--text));
  }
  .bar:hover { background: color-mix(in srgb, var(--c) 28%, var(--bg)); }
  /* Joined edges where the event continues into the previous/next week. */
  .bar.from-prev { margin-left: 0; border-top-left-radius: 0; border-bottom-left-radius: 0; }
  .bar.to-next { margin-right: 0; border-top-right-radius: 0; border-bottom-right-radius: 0; }
  .t { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .time { color: var(--muted); font-size: 11px; }
  .more { border: none; background: none; font-size: 11px; color: var(--muted); text-align: left; padding: 0 4px; }
</style>
