<script lang="ts">
  import { activate, isSelected, select } from '../../lib/selection.svelte';
  import { t } from '../../lib/i18n';
  import { Repeat } from 'lucide-svelte';
  import type { Plan, Task, TaskStatus } from '../../lib/types';
  import { checklistLabel, progressLabel } from '../../lib/subtasks';
  import StatusGlyph from './StatusGlyph.svelte';
  import PriorityMark from './PriorityMark.svelte';
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
    sub = false,
    dimmed = false,
    kids = 0,
    collapsed = false,
    ontoggle,
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
    /** A subtask, indented under its parent. */
    sub?: boolean;
    /** A parent shown only because a subtask of it matches the filters. */
    dimmed?: boolean;
    /** Subtasks listed under this row; with any, the row gets a show/hide toggle. */
    kids?: number;
    collapsed?: boolean;
    ontoggle?: () => void;
  } = $props();

  const progress = $derived(progressLabel(task));
  const checklist = $derived(checklistLabel(task));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (mouse convenience: the row's open button is the keyboard path) -->
<div class="row" class:done={task.status === 'done'} class:sub class:dimmed class:is-selected={isSelected('task', task.id)}
  onclick={() => select({ kind: 'task', id: task.id })} ondblclick={onopen}>
  {#if kids}
    <button class="icon-btn caret" aria-expanded={!collapsed} aria-label={`${collapsed ? t('subtasks.expand') : t('subtasks.collapse')}: ${task.title}`}
      title={collapsed ? t('subtasks.expand') : t('subtasks.collapse')}
      onclick={(e) => { e.stopPropagation(); ontoggle?.(); }} ondblclick={(e) => e.stopPropagation()}><span class:down={!collapsed} aria-hidden="true">▸</span></button>
  {:else if !sub}
    <span class="caret-gap" aria-hidden="true"></span>
  {/if}
  <StatusGlyph status={task.status} size={sub ? 15 : undefined} onchange={onstatus} />
  {#if plan}<span class="plan" style="background:{plan.color}" title={plan.name} aria-hidden="true"></span>{/if}
  <PriorityMark priority={task.priority} />
  <button class="open-btn title" onclick={(e) => { e.stopPropagation(); activate(e, onopen, () => select({ kind: 'task', id: task.id })); }}>{task.title}</button>
  {#if task.labels.length}
    <span class="labels">
      {#each task.labels as l (l.id)}
        <LabelChip name={l.name} color={l.color} active={activeLabels.includes(l.id)} onclick={() => onlabel(l.id)} />
      {/each}
    </span>
  {/if}
  {#if progress}<span class="count" title={t('subtasks.title')}>{progress}</span>{/if}
  {#if checklist}<span class="count" title={t('checklist.title')}><span aria-hidden="true">☑</span> <span class="sr">{t('checklist.title')}</span>{checklist}</span>{/if}
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
  /* Subtasks sit under their parent's title (past the toggle and its circle). */
  .row.sub { padding-left: 54px; min-height: 32px; padding-top: 5px; padding-bottom: 5px; }
  .row.dimmed .title, .row.dimmed :global(.btn-glyph) { opacity: 0.55; }
  .caret, .caret-gap { width: 16px; flex: none; margin-right: -6px; }
  .caret { padding: 0; height: 18px; display: grid; place-items: center; font-size: 10px; color: var(--muted); }
  .caret span { display: inline-block; }
  .caret span.down { transform: rotate(90deg); }
  @media (prefers-reduced-motion: no-preference) { .caret span { transition: transform 0.15s; } }
  .count { font-size: 12px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; flex: none; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; }
  .done .title { color: var(--faint); text-decoration: line-through; text-decoration-color: var(--faint); }
  .labels { display: flex; gap: 6px; flex: none; max-width: 40%; overflow: hidden; }
  .row :global(.rep) { color: var(--faint); flex: none; }
  .due { font-size: 12px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .due.late { color: var(--danger); }
  .plan { width: 7px; height: 7px; border-radius: 50%; flex: none; margin: 0 -3px 0 -2px; }
  .cal { width: 6px; height: 6px; border-radius: 50%; flex: none; opacity: 0.8; }
</style>
