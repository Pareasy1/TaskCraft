<script lang="ts">
  import { t } from '../lib/i18n';
  import type { Occurrence, Task, TaskStatus } from '../lib/types';
  import { tasksForDay } from '../lib/calendarTasks';
  import { STATUS_LABEL } from '../lib/tasks';
  import StatusGlyph from './tasks/StatusGlyph.svelte';
  import { activate, isSelected, select } from '../lib/selection.svelte';
  import { addDays, monthBars, monthGrid, sameDay } from '../lib/layout';
  import { fmtTime, fmtWeekdayShort } from '../lib/format';

  let {
    month,
    weekStart,
    occurrences,
    onselect,
    onday,
    tasks = [],
    ontaskopen = () => {},
    ontaskstatus = () => {},
    parentTitles = new Map(),
  }: {
    month: Date;
    weekStart: 0 | 1;
    occurrences: Occurrence[];
    onselect: (o: Occurrence) => void;
    onday: (d: Date) => void;
    tasks?: Task[];
    ontaskopen?: (t: Task) => void;
    ontaskstatus?: (t: Task, s: TaskStatus) => void;
    /** Task id to title, to label subtasks "Subtask · Parent". */
    parentTitles?: Map<number, string>;
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
  // Tasks due on the day (date-only first), listed under its events.
  const tasksOn = (d: Date) => {
    const r = tasksForDay(tasks, d, today);
    return [...r.allDay.filter((x) => x.span || sameDay(new Date(x.task.due!), d)), ...r.timed];
  };
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
      {@const dayTasks = tasksOn(d)}
      {@const room = LANES - rowLanes[Math.floor(i / 7)]}
      {@const taskRoom = Math.max(0, room - items.length)}
      {@const more = Math.max(0, items.length + dayTasks.length - room) + hiddenBarsAt(i)}
      <div class="cell" class:other={d.getMonth() !== month.getMonth()} role="gridcell" tabindex="-1"
        style="grid-row:{Math.floor(i / 7) + 1}; grid-column:{(i % 7) + 1}" ondblclick={() => onday(d)}>
        <button class="num" class:today={sameDay(d, today)} onclick={() => onday(d)}>{d.getDate()}</button>
        <div class="items" style="margin-top:{rowLanes[Math.floor(i / 7)] * 20}px">
          {#each items.slice(0, room) as o (o.eventId + o.start)}
            <button class="item" class:is-selected={isSelected('event', o.eventId, o.start)} style="--c:{o.color}"
              onclick={(e) => activate(e, () => onselect(o), () => select({ kind: 'event', id: o.eventId, occurrence: o.start }))}
              ondblclick={() => onselect(o)}>
              <span class="dot" style="background:{o.color}"></span>
              <span class="t">{o.title}</span>
              <span class="time">{fmtTime(new Date(o.start))}</span>
            </button>
          {/each}
          {#each dayTasks.slice(0, taskRoom) as x (x.task.id)}
            {@const label = (x.task.parentId != null ? t('subtasks.calTitle', { parent: parentTitles.get(x.task.parentId) ?? '' }) + ' — ' : '') + x.task.title}
            <div class="titem" class:span-start={x.span === 'start'} class:span-middle={x.span === 'middle'} class:span-end={x.span === 'end'} class:is-selected={isSelected('task', x.task.id)} class:done={x.task.status === 'done'} class:overdue={x.overdue}>
              <StatusGlyph status={x.task.status} size={12} onchange={(st) => ontaskstatus(x.task, st)} />
              <button class="t" onclick={(e) => activate(e, () => ontaskopen(x.task), () => select({ kind: 'task', id: x.task.id }))}
                ondblclick={() => ontaskopen(x.task)}
                aria-label="{label}{x.span ? ` (${t(`span.${x.span}`)})` : ''}, {STATUS_LABEL[x.task.status]}">{#if x.span}<span aria-hidden="true">{x.span === 'start' ? '↦' : x.span === 'middle' ? '→' : '⇥'} </span>{/if}{label}</button>
              {#if x.task.dueHasTime}<span class="time">{fmtTime(new Date(x.task.due!))}</span>{/if}
            </div>
          {/each}
          {#if more}<button class="more" onclick={() => onday(d)}>{t('common.more', { n: more })}</button>{/if}
        </div>
      </div>
    {/each}
    {#each bars.filter((b) => b.lane < LANES) as b (b.item.eventId + b.item.start + b.row)}
      <button class="bar" class:from-prev={b.fromPrev} class:to-next={b.toNext}
        style="--c:{b.item.color}; grid-row:{b.row + 1}; grid-column:{b.col + 1} / span {b.span}; margin-top:{26 + b.lane * 20}px"
        class:is-selected={isSelected('event', b.item.eventId, b.item.start)}
        onclick={(e) => activate(e, () => onselect(b.item), () => select({ kind: 'event', id: b.item.eventId, occurrence: b.item.start }))}
        ondblclick={() => onselect(b.item)}>
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
  .titem { display: flex; align-items: center; gap: 4px; padding: 1px 4px 1px 3px; min-width: 0; border-radius: 4px; }
  .titem.span-start { border-top-right-radius: 0; border-bottom-right-radius: 0; }
  .titem.span-middle { border-radius: 0; }
  .titem.span-end { border-top-left-radius: 0; border-bottom-left-radius: 0; }
  .titem.span-start, .titem.span-middle, .titem.span-end { background: var(--hover); }
  .titem:hover { background: var(--hover); }
  .titem .t { border: none; background: none; padding: 0; text-align: left; font-size: 11.5px; color: var(--text); }
  .titem.done .t { color: var(--muted); text-decoration: line-through; }
  .titem.overdue .time { color: var(--danger); }
  .more { border: none; background: none; font-size: 11px; color: var(--muted); text-align: left; padding: 0 4px; }
</style>
