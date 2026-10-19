<script lang="ts">
  import type { Occurrence, Task } from '../lib/types';
  import { fmtDayHeading, fmtTime, ymd } from '../lib/format';
  import { dayGroups } from '../lib/layout';

  let {
    occurrences,
    pinned,
    dated,
    from,
    days,
    oncomplete,
    onopen,
  }: {
    occurrences: Occurrence[];
    /** Overdue and today's tasks, shown on top. */
    pinned: Task[];
    /** Upcoming tasks, shown under their due day. */
    dated: Task[];
    from: Date;
    days: number;
    oncomplete: (t: Task) => void;
    onopen: (kind: 'event' | 'task', id: number) => void;
  } = $props();

  let groups = $derived(dayGroups(occurrences, dated, from, days));
  const overdue = (t: Task) => !!t.due && (t.dueHasTime ? new Date(t.due) < new Date() : ymd(new Date(t.due)) < ymd(new Date()));
</script>

{#snippet taskRow(t: Task, meta: string, late: boolean)}
  <div class="item task" role="button" tabindex="-1" ondblclick={() => onopen('task', t.id)}>
    <input type="checkbox" checked={!!t.completedAt} style="accent-color:{t.color}" onchange={() => oncomplete(t)} />
    <span class="name">{t.title}</span>
    <span class="meta" class:late>{meta}</span>
  </div>
{/snippet}

<div class="list">
  {#if pinned.length}
    <div class="heading">Tasks</div>
    {#each pinned as t (t.id)}
      {@render taskRow(t, overdue(t) ? 'Overdue' : t.dueHasTime && t.due ? fmtTime(new Date(t.due)) : '', overdue(t))}
    {/each}
  {/if}
  {#each groups as g (g.day.getTime())}
    <div class="heading">{fmtDayHeading(g.day)}</div>
    {#each g.tasks as t (t.id)}
      {@render taskRow(t, t.dueHasTime && t.due ? fmtTime(new Date(t.due)) : '', false)}
    {/each}
    {#each g.events as o (o.eventId + o.start)}
      <div class="item" role="button" tabindex="-1" ondblclick={() => onopen('event', o.eventId)}>
        <span class="dot" style="background:{o.color}"></span>
        <span class="name">{o.title}</span>
        <span class="meta">{o.allDay ? 'all-day' : fmtTime(new Date(o.start))}</span>
      </div>
    {/each}
  {:else}
    {#if !pinned.length}<div class="empty">Nothing scheduled</div>{/if}
  {/each}
</div>

<style>
  .list { display: grid; gap: 1px; }
  .heading {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 8px 6px 3px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: 6px;
  }
  .item:hover { background: var(--hover); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  .late { color: var(--danger); }
  .task input { margin: 0; }
  .empty { color: var(--faint); text-align: center; padding: 18px 0; }
</style>
