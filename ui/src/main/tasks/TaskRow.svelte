<script lang="ts">
  import { t } from '../../lib/i18n';
  import { Repeat } from 'lucide-svelte';
  import type { Task, TaskStatus } from '../../lib/types';
  import StatusGlyph from './StatusGlyph.svelte';
  import LabelChip from './LabelChip.svelte';

  let {
    task,
    due,
    late,
    activeLabels,
    onstatus,
    onopen,
    onlabel,
  }: {
    task: Task;
    due: string;
    late: boolean;
    activeLabels: number[];
    onstatus: (s: TaskStatus) => void;
    onopen: () => void;
    onlabel: (id: number) => void;
  } = $props();
</script>

<div class="row" class:done={task.status === 'done'} role="button" tabindex="0" onclick={onopen} onkeydown={(e) => e.key === 'Enter' && onopen()}>
  <StatusGlyph status={task.status} onchange={onstatus} />
  <span class="title">{task.title}</span>
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
  .row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; }
  .done .title { color: var(--faint); text-decoration: line-through; text-decoration-color: var(--faint); }
  .labels { display: flex; gap: 6px; flex: none; max-width: 40%; overflow: hidden; }
  .row :global(.rep) { color: var(--faint); flex: none; }
  .due { font-size: 12px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .due.late { color: var(--danger); }
  .cal { width: 6px; height: 6px; border-radius: 50%; flex: none; opacity: 0.8; }
</style>
