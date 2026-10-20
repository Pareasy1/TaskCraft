<script lang="ts">
  import { activate, isSelected, select } from '../lib/selection.svelte';
  import { t as tr } from '../lib/i18n';
  import type { Occurrence, Task, TaskStatus } from '../lib/types';
  import StatusGlyph from '../main/tasks/StatusGlyph.svelte';
  import LabelChip from '../main/tasks/LabelChip.svelte';
  import { fmtDayHeading, fmtTime, ymd } from '../lib/format';
  import { dayGroups } from '../lib/layout';

  let {
    occurrences,
    pinned,
    dated,
    from,
    days,
    onstatus,
    onopen,
  }: {
    occurrences: Occurrence[];
    /** Overdue and today's tasks, shown on top. */
    pinned: Task[];
    /** Upcoming tasks, shown under their due day. */
    dated: Task[];
    from: Date;
    days: number;
    onstatus: (t: Task, s: TaskStatus) => void;
    onopen: (kind: 'event' | 'task', id: number) => void;
  } = $props();

  let groups = $derived(dayGroups(occurrences, dated, from, days));
  const overdue = (t: Task) => !!t.due && (t.dueHasTime ? new Date(t.due) < new Date() : ymd(new Date(t.due)) < ymd(new Date()));
</script>

{#snippet taskRow(t: Task, meta: string, late: boolean)}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (mouse convenience: the row's open button is the keyboard path) -->
  <div class="item task" class:is-selected={isSelected('task', t.id)} onclick={() => select({ kind: 'task', id: t.id })} ondblclick={() => onopen('task', t.id)}>
    <StatusGlyph status={t.status} size={15} onchange={(s) => onstatus(t, s)} />
    <button class="open-btn opener" title={tr('quick.openTask', { title: t.title })} onclick={(e) => { e.stopPropagation(); activate(e, () => onopen('task', t.id), () => select({ kind: 'task', id: t.id })); }}>
      <span class="name">{t.title}</span>
      {#each t.labels as l (l.id)}<LabelChip name={l.name} color={l.color} />{/each}
      <span class="meta" class:late>{meta}</span>
    </button>
  </div>
{/snippet}

<div class="list">
  {#if pinned.length}
    <div class="heading">{tr('group.tasks')}</div>
    {#each pinned as t (t.id)}
      {@render taskRow(t, overdue(t) ? tr('group.overdue') : t.dueHasTime && t.due ? fmtTime(new Date(t.due)) : '', overdue(t))}
    {/each}
  {/if}
  {#each groups as g (g.day.getTime())}
    <div class="heading">{fmtDayHeading(g.day)}</div>
    {#each g.tasks as t (t.id)}
      {@render taskRow(t, t.dueHasTime && t.due ? fmtTime(new Date(t.due)) : '', false)}
    {/each}
    {#each g.events as o (o.eventId + o.start)}
      <div class="item" role="button" tabindex="0" aria-label={tr('quick.openEvent', { title: o.title })}
        class:is-selected={isSelected('event', o.eventId, o.start)}
        onclick={() => select({ kind: 'event', id: o.eventId, occurrence: o.start })} ondblclick={() => onopen('event', o.eventId)}
        onkeydown={(e) => e.key === 'Enter' && onopen('event', o.eventId)}>
        <span class="dot" style="background:{o.color}"></span>
        <span class="name">{o.title}</span>
        <span class="meta">{o.allDay ? tr('grid.allDay') : fmtTime(new Date(o.start))}</span>
      </div>
    {/each}
  {:else}
    {#if !pinned.length}<div class="empty">{tr('quick.nothing')}</div>{/if}
  {/each}
</div>

<style>
  .list { display: grid; gap: 1px; }
  .heading {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    padding: 10px 6px 3px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: 6px;
  }
  .item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .opener { flex: 1; display: flex; align-items: center; gap: 8px; }
  .item:hover { background: var(--hover); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  .late { color: var(--danger); }
  .empty { color: var(--faint); text-align: center; padding: 18px 0; }
</style>
