<script lang="ts" module>
  import { SvelteSet } from 'svelte/reactivity';
  /** Cards whose subtasks are shown, remembered for the session. */
  const expanded = new SvelteSet<number>();
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { activate, isSelected, select } from '../../lib/selection.svelte';
  import { t as tr } from '../../lib/i18n';
  import { Paperclip, Repeat } from 'lucide-svelte';
  import type { Plan, Task, TaskStatus } from '../../lib/types';
  import { isOverdue } from '../../lib/tasks';
  import { fmtShortDate, fmtSpan, fmtTime } from '../../lib/format';
  import { checklistLabel, progressLabel } from '../../lib/subtasks';
  import StatusGlyph from './StatusGlyph.svelte';
  import PriorityMark from './PriorityMark.svelte';
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
    children = [],
    onchildstatus,
    onchildopen,
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
    /** Its subtasks, in order (shown when the card is expanded). */
    children?: Task[];
    onchildstatus?: (c: Task, s: TaskStatus) => void;
    onchildopen?: (c: Task) => void;
  } = $props();

  const due = $derived.by(() => {
    if (!t.due) return '';
    // With a start date the card shows the span ("24 → 28 Oct"), without times.
    if (t.start) return fmtSpan(new Date(t.start), new Date(t.due));
    return `${fmtShortDate(new Date(t.due))}${t.dueHasTime ? `, ${fmtTime(new Date(t.due))}` : ''}`;
  });
  const progress = $derived(progressLabel(t));
  const checklist = $derived(checklistLabel(t));
  const open = $derived(expanded.has(t.id) && children.length > 0);
  const toggle = () => (expanded.has(t.id) ? expanded.delete(t.id) : expanded.add(t.id));
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
    <PriorityMark priority={t.priority} />
    <button class="open-btn title" data-card={t.id} aria-label={titleLabel}
      onclick={(e) => { e.stopPropagation(); activate(e, onopen, () => select({ kind: 'task', id: t.id })); }} onkeydown={ontitlekey}>{t.title}</button>
    {@render actions?.()}
  </div>
  {#if t.labels.length || t.due || t.recurring || progress || checklist || t.attachmentCount}
    <div class="meta">
      {#if progress}
        <button class="count prog" aria-expanded={open} disabled={!children.length}
          aria-label={`${open ? tr('subtasks.collapse') : tr('subtasks.expand')}, ${progress}`}
          title={open ? tr('subtasks.collapse') : tr('subtasks.expand')}
          onclick={(e) => { e.stopPropagation(); toggle(); }} ondblclick={(e) => e.stopPropagation()}>
          <span class="caret" class:down={open} aria-hidden="true">▸</span>{progress}
        </button>
      {/if}
      {#if checklist}<span class="count" title={tr('checklist.title')}><span aria-hidden="true">☑</span> <span class="sr">{tr('checklist.title')}</span>{checklist}</span>{/if}
      {#if t.attachmentCount}<span class="count" title={tr('attachments.title')}><Paperclip size={11} aria-hidden="true" /><span class="sr">{tr('attachments.title')}</span>{t.attachmentCount}</span>{/if}
      {#each t.labels as l (l.id)}
        <LabelChip name={l.name} color={l.color} active={activeLabels.includes(l.id)} onclick={() => onlabel(l.id)} />
      {/each}
      <span class="spacer"></span>
      {#if t.recurring}<Repeat size={11} class="rep" aria-label={tr('common.repeats')} />{/if}
      {#if t.due && t.status !== 'done'}<span class="due" class:late={isOverdue(t, now)}>{due}</span>{/if}
      <span class="cal" style="background:{t.color}"></span>
    </div>
  {/if}
  {#if progress}
    <div class="bar" aria-hidden="true"><span style="width:{((t.subtaskTotal - t.subtaskOpen) / t.subtaskTotal) * 100}%"></span></div>
  {/if}
  {#if open}
    <ul class="subs" aria-label={tr('subtasks.title')}>
      {#each children as c (c.id)}
        <li class:done={c.status === 'done'}>
          <StatusGlyph status={c.status} size={14} onchange={(s) => onchildstatus?.(c, s)} />
          <button class="open-btn sub-title" onclick={(e) => { e.stopPropagation(); onchildopen?.(c); }} ondblclick={(e) => e.stopPropagation()}>{c.title}</button>
        </li>
      {/each}
    </ul>
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
  .count { display: inline-flex; align-items: center; gap: 3px; font-size: 11.5px; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .prog { border: none; background: none; padding: 1px 4px 1px 2px; margin-left: -2px; border-radius: 5px; }
  .prog:hover:not(:disabled) { background: var(--hover); color: var(--text); }
  .prog:focus-visible { outline: 2px solid var(--accent); }
  .caret { display: inline-block; font-size: 10px; }
  .caret.down { transform: rotate(90deg); }
  @media (prefers-reduced-motion: no-preference) { .caret { transition: transform 0.15s; } }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .bar { height: 3px; border-radius: 2px; background: var(--border); margin: 6px 0 0 24px; overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--status-done); border-radius: 2px; }
  .subs { list-style: none; margin: 6px 0 0 18px; padding: 4px 0 0; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 2px; }
  .subs li { display: flex; align-items: flex-start; gap: 6px; padding: 2px; border-radius: 6px; }
  .subs li:hover { background: var(--hover); }
  .sub-title { flex: 1; min-width: 0; font-size: 12.5px; line-height: 1.35; overflow-wrap: anywhere; text-align: left; }
  .subs li.done .sub-title { color: var(--faint); text-decoration: line-through; }
  .cal { width: 6px; height: 6px; border-radius: 50%; opacity: 0.8; }
</style>
