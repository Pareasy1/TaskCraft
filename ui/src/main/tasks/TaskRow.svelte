<script lang="ts">
  import { activate, isSelected, select } from '../../lib/selection.svelte';
  import { t } from '../../lib/i18n';
  import { Repeat } from 'lucide-svelte';
  import type { Plan, Task, TaskStatus } from '../../lib/types';
  import StatusGlyph from './StatusGlyph.svelte';
  import LabelChip from './LabelChip.svelte';

  let {
    task,
    due,
    late,
    activeLabels,
    plan,
    onstatus,
    onopen,
    onlabel,
  }: {
    task: Task;
    due: string;
    late: boolean;
    activeLabels: number[];
    /** Shown as a dot before the title (All tasks). */
    plan?: Plan;
    onstatus: (s: TaskStatus) => void;
    onopen: () => void;
    onlabel: (id: number) => void;
  } = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (mouse convenience: the row's open button is the keyboard path) -->
<div class="row" class:done={task.status === 'done'} class:is-selected={isSelected('task', task.id)}
  onclick={() => select({ kind: 'task', id: task.id })} ondblclick={onopen}>
  <StatusGlyph status={task.status} onchange={onstatus} />
  {#if plan}<span class="plan" style="background:{plan.color}" title={plan.name} aria-hidden="true"></span>{/if}
  <button class="open-btn title" onclick={(e) => { e.stopPropagation(); activate(e, onopen, () => select({ kind: 'task', id: task.id })); }}>{task.title}</button>
  {#if task.labels.length}
    <span class="labels">
      {#each task.labels as l (l.id)}
        <LabelChip name={l.name} color={l.color} active={activeLabels.includes(l.id)} onclick={() => onlabel(l.id)} />
      {/each}
    </span>
  {/if}
  {#if task.recurring}<Repeat size={12} class="rep" aria-label={t('common.repeats')} />{/if}
  {#if due}<span class="due" class:late>{due}</span>{/if}
  <span class="cal" style="background:{task.color}" title={t('labels.calendarColor')}></span>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    margin: 0 -10px;
    border-radius: 8px;
    min-height: 36px;
  }
  .row:hover { background: var(--hover); }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; }
  .done .title { color: var(--faint); text-decoration: line-through; text-decoration-color: var(--faint); }
  .labels { display: flex; gap: 6px; flex: none; max-width: 40%; overflow: hidden; }
  .row :global(.rep) { color: var(--faint); flex: none; }
  .due { font-size: 12px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .due.late { color: var(--danger); }
  .plan { width: 7px; height: 7px; border-radius: 50%; flex: none; margin: 0 -3px 0 -2px; }
  .cal { width: 6px; height: 6px; border-radius: 50%; flex: none; opacity: 0.8; }
</style>
