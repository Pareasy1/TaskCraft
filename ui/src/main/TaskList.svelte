<script lang="ts">
  import { activate, isSelected, select } from '../lib/selection.svelte';
  import type { Task, TaskStatus } from '../lib/types';
  import { fmtShortDate, fmtTime } from '../lib/format';
  import StatusGlyph from './tasks/StatusGlyph.svelte';
  import LabelChip from './tasks/LabelChip.svelte';
  import { progressLabel } from '../lib/subtasks';

  let {
    sections,
    onstatus,
    onopen,
  }: {
    sections: { key: string; name: string; tasks: Task[] }[];
    onstatus: (t: Task, s: TaskStatus) => void;
    onopen: (t: Task) => void;
  } = $props();

  let open = $state<Record<string, boolean>>({ completed: false });
  const due = (t: Task) => (t.due ? `${fmtShortDate(new Date(t.due))}${t.dueHasTime ? ` ${fmtTime(new Date(t.due))}` : ''}` : '');
</script>

{#each sections as s (s.key)}
  {#if s.tasks.length}
    <button class="sec" aria-expanded={open[s.key] ?? true} onclick={() => (open[s.key] = !(open[s.key] ?? true))}>
      <span>{s.name}</span><span class="count">{s.tasks.length}</span>
    </button>
    {#if open[s.key] ?? true}
      {#each s.tasks as t (t.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (mouse convenience: the row's open button is the keyboard path) -->
        <div class="task" class:done={t.status === 'done'} class:is-selected={isSelected('task', t.id)}
          onclick={() => select({ kind: 'task', id: t.id })} ondblclick={() => onopen(t)}>
          <StatusGlyph status={t.status} size={15} onchange={(st) => onstatus(t, st)} />
          <button class="open-btn body" onclick={(e) => { e.stopPropagation(); activate(e, () => onopen(t), () => select({ kind: 'task', id: t.id })); }}>
            <span class="name">{t.title}</span>
            {#if t.due || t.labels.length || t.subtaskTotal}
              <span class="meta">
                {#if t.due}<span class="due" class:overdue={s.key === 'overdue'}>{due(t)}</span>{/if}
                {#if t.subtaskTotal}<span class="prog">{progressLabel(t)}</span>{/if}
                {#each t.labels as l (l.id)}<LabelChip name={l.name} color={l.color} />{/each}
              </span>
            {/if}
          </button>
        </div>
      {/each}
    {/if}
  {/if}
{/each}

<style>
  .sec {
    width: 100%;
    display: flex;
    justify-content: space-between;
    border: none;
    background: none;
    padding: 12px 6px 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .count { font-variant-numeric: tabular-nums; font-weight: 400; }
  .task { display: flex; align-items: flex-start; gap: 8px; padding: 5px 6px; border-radius: 6px; }
  .task:hover { background: var(--hover); }
  .task :global(.wrap) { margin-top: 1px; }
  .body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .name {
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .meta { display: flex; flex-wrap: wrap; align-items: baseline; gap: 2px 8px; font-size: 11.5px; }
  .meta :global(.chip) { font-size: 11.5px; }
  .due { color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .prog { color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .due.overdue { color: var(--danger); }
  .done .name { color: var(--muted); text-decoration: line-through; }
</style>
