<script lang="ts">
  import type { Task } from '../lib/types';
  import { fmtShortDate, fmtTime } from '../lib/format';

  let {
    sections,
    oncomplete,
    onopen,
  }: {
    sections: { name: string; tasks: Task[]; collapsed?: boolean }[];
    oncomplete: (t: Task) => void;
    onopen: (t: Task) => void;
  } = $props();

  let open = $state<Record<string, boolean>>({ Completed: false });
  const due = (t: Task) => (t.due ? `${fmtShortDate(new Date(t.due))}${t.dueHasTime ? ` ${fmtTime(new Date(t.due))}` : ''}` : '');
</script>

{#each sections as s (s.name)}
  {#if s.tasks.length}
    <button class="sec" onclick={() => (open[s.name] = !(open[s.name] ?? true))}>
      <span>{s.name}</span><span class="count">{s.tasks.length}</span>
    </button>
    {#if open[s.name] ?? true}
      {#each s.tasks as t (t.id)}
        <div class="task" class:done={!!t.completedAt} class:overdue={s.name === 'Overdue'} role="button" tabindex="-1" onclick={() => onopen(t)} onkeydown={() => {}}>
          <input type="checkbox" checked={!!t.completedAt} style="accent-color:{t.color}" onclick={(e) => e.stopPropagation()} onchange={() => oncomplete(t)} />
          <span class="name">{t.title}</span>
          <span class="due">{due(t)}</span>
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
    padding: 10px 8px 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .count { font-variant-numeric: tabular-nums; }
  .task { display: flex; align-items: center; gap: 7px; padding: 4px 8px; border-radius: 6px; }
  .task:hover { background: var(--hover); }
  .task input { margin: 0; }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .due { font-size: 11px; color: var(--muted); white-space: nowrap; }
  .overdue .due { color: var(--danger); }
  .done .name { color: var(--faint); text-decoration: line-through; }
</style>
