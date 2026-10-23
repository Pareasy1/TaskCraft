<script lang="ts">
  import type { Snippet } from 'svelte';
  import { activate, isSelected, select } from '../../lib/selection.svelte';
  import { t as tr } from '../../lib/i18n';
  import { Repeat } from 'lucide-svelte';
  import type { Plan, Task, TaskStatus } from '../../lib/types';
  import { isOverdue } from '../../lib/tasks';
  import { fmtShortDate, fmtTime } from '../../lib/format';
  import StatusGlyph from './StatusGlyph.svelte';
  import LabelChip from './LabelChip.svelte';

  // One card on a board. The board wraps it in the keyed, draggable element.
  let {
    task: t,
    now,
    activeLabels,
    plan,
    titleLabel,
    struck = false,
    onstatus,
    onopen,
    onlabel,
    ontitlekey,
    oncontextmenu,
    actions,
  }: {
    task: Task;
    now: Date;
    activeLabels: number[];
    /** Shown as a dot before the title (All tasks). */
    plan?: Plan;
    titleLabel?: string;
    /** Draw a finished task struck through (cards that stay in their bucket). */
    struck?: boolean;
    onstatus: (s: TaskStatus) => void;
    onopen: () => void;
    onlabel: (id: number) => void;
    ontitlekey?: (e: KeyboardEvent) => void;
    oncontextmenu?: (e: MouseEvent) => void;
    /** Extra controls at the end of the title row (the "…" button). */
    actions?: Snippet;
  } = $props();

  const due = $derived(t.due ? `${fmtShortDate(new Date(t.due))}${t.dueHasTime ? `, ${fmtTime(new Date(t.due))}` : ''}` : '');
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (mouse convenience: the card's open button is the keyboard path) -->
<div
  class="card"
  class:struck={struck && t.status === 'done'}
  class:is-selected={isSelected('task', t.id)}
  onclick={() => select({ kind: 'task', id: t.id })}
  ondblclick={onopen}
  {oncontextmenu}
>
  <div class="top">
    <StatusGlyph status={t.status} size={16} onchange={onstatus} />
    {#if plan}<span class="plan" style="background:{plan.color}" title={plan.name} aria-hidden="true"></span>{/if}
    <button class="open-btn title" data-card={t.id} aria-label={titleLabel}
      onclick={(e) => { e.stopPropagation(); activate(e, onopen, () => select({ kind: 'task', id: t.id })); }} onkeydown={ontitlekey}>{t.title}</button>
    {@render actions?.()}
  </div>
  {#if t.labels.length || t.due || t.recurring}
    <div class="meta">
      {#each t.labels as l (l.id)}
        <LabelChip name={l.name} color={l.color} active={activeLabels.includes(l.id)} onclick={() => onlabel(l.id)} />
      {/each}
      <span class="spacer"></span>
      {#if t.recurring}<Repeat size={11} class="rep" aria-label={tr('common.repeats')} />{/if}
      {#if t.due && t.status !== 'done'}<span class="due" class:late={isOverdue(t, now)}>{due}</span>{/if}
      <span class="cal" style="background:{t.color}"></span>
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--surface-strong);
    border-radius: 9px;
    padding: 9px 10px;
    box-shadow: 0 0 0 1px var(--border), 0 1px 2px rgba(0, 0, 0, 0.05);
  }
  .card:hover { box-shadow: 0 0 0 1px var(--border), 0 3px 10px rgba(0, 0, 0, 0.08); }
  .card:has(.title:focus-visible) { box-shadow: 0 0 0 2px var(--accent); }
  .card .title:focus-visible { outline: none; }
  @media (prefers-reduced-motion: no-preference) {
    .card { transition: box-shadow 0.15s, transform 0.15s, opacity 0.15s; }
    .card:active { transform: scale(1.015) rotate(-0.4deg); box-shadow: 0 0 0 1px var(--border), 0 10px 24px rgba(0, 0, 0, 0.16); }
  }
  .top { display: flex; align-items: flex-start; gap: 8px; }
  .top :global(.btn-glyph) { margin-top: -1px; }
  .title { flex: 1; min-width: 0; font-size: 13px; line-height: 1.35; overflow-wrap: anywhere; }
  .struck .title { color: var(--faint); text-decoration: line-through; text-decoration-color: var(--faint); }
  .plan { width: 7px; height: 7px; border-radius: 50%; flex: none; margin-top: 5px; }
  .meta { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; margin: 6px 0 0 24px; }
  .spacer { flex: 1; }
  .meta :global(.rep) { color: var(--faint); }
  .due { font-size: 11.5px; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .due.late { color: var(--danger); }
  .cal { width: 6px; height: 6px; border-radius: 50%; opacity: 0.8; }
</style>
