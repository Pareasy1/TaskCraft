<script lang="ts" module>
  // Measures a title in the bar's font, to decide whether it fits inside the bar.
  let ctx: CanvasRenderingContext2D | null | undefined;
  function textWidth(s: string): number {
    if (ctx === undefined) {
      try {
        ctx = document.createElement('canvas').getContext('2d');
        if (ctx) ctx.font = '500 12px -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif';
      } catch {
        ctx = null;
      }
    }
    return ctx ? ctx.measureText(s).width : s.length * 6.5;
  }
  const PAD = 7;
  const BADGE_W = 30;
  /** Width of the start / due drag handles at the bar's ends. */
  const EDGE = 6;
</script>

<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { Task } from '../../lib/types';
  import type { TaskDates } from '../../lib/actions';
  import { barSpan, dayToX, DAY_WIDTH, dragResult, type DragMode, type Zoom } from '../../lib/timeline';
  import { isOverdue } from '../../lib/tasks';
  import { progressSegments } from '../../lib/subtasks';
  import { fmtShortDate } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import BlockedBadge from '../tasks/BlockedBadge.svelte';

  // One task's bar on its row (decorative: the row carries the accessible name and the keyboard
  // moves). Pointer drags move it (body) or change start / due (edges), in whole local days.
  let { task, range, zoom, now, blockers, onpreview, onsave }: {
    task: Task;
    range: { from: number; to: number };
    zoom: Zoom;
    now: Date;
    /** Titles of the open tasks this one waits on, when known. */
    blockers?: string[];
    /** Shows the bar at in-progress dates while dragging (null = back to the task's own). */
    onpreview: (id: number, next: TaskDates | null) => void;
    /** Saves a finished drag; `task` is the task as it was when the drag began. */
    onsave: (task: Task, next: TaskDates) => void;
  } = $props();

  let span = $derived(barSpan(task));
  let x = $derived(span ? dayToX(span.from, range, zoom) : 0);
  let width = $derived(span ? (span.to - span.from + 1) * DAY_WIDTH[zoom] : 0);
  let overdue = $derived(isOverdue(task, now));
  let blocked = $derived(task.blockedBy > 0);
  let fits = $derived(textWidth(task.title) + 2 * PAD + (blocked ? BADGE_W : 0) <= width);
  let seg = $derived(task.subtaskTotal ? progressSegments(task) : null);
  const pct = (n: number) => `${(n / task.subtaskTotal) * 100}%`;
  /** Edge handles only on bars wide enough to keep a body to grab. */
  let handles = $derived(width >= 3 * EDGE);

  // ── Dragging ──
  let drag = $state<{ mode: DragMode; orig: Task; x0: number; id: number; next: TaskDates | null } | null>(null);
  /** When the last real drag ended: the double-click that may follow is not an "open". */
  let endedAt = 0;

  function down(e: PointerEvent) {
    if (e.button !== 0 || e.ctrlKey || drag) return; // Ctrl-click is the macOS context click
    const edge = (e.target as HTMLElement).closest<HTMLElement>('[data-edge]')?.dataset.edge as DragMode | undefined;
    drag = { mode: edge ?? 'move', orig: task, x0: e.clientX, id: e.pointerId, next: null };
    window.addEventListener('keydown', esc, true);
    try {
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    } catch { /* the pointer is already gone: up / cancel end the drag */ }
  }

  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    // Whole days from the pointer's travel; under one day it is still a click.
    const delta = Math.trunc((e.clientX - drag.x0) / DAY_WIDTH[zoom]);
    if (delta === 0 && !drag.next) return;
    drag.next = dragResult(drag.orig, drag.mode, delta);
    onpreview(drag.orig.id, drag.next);
  }

  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const { orig, next } = drag;
    end();
    if (!next) return; // a click: the row's click / double-click handle it
    endedAt = performance.now();
    if (same(orig, next)) onpreview(orig.id, null);
    else onsave(orig, next);
  }

  /** Esc, a cancelled pointer or a lost capture: the bar goes back where it was. */
  function cancel() {
    if (!drag) return;
    const { orig, next } = drag;
    end();
    if (next) {
      endedAt = performance.now();
      onpreview(orig.id, null);
    }
  }

  function esc(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !drag) return;
    e.preventDefault();
    e.stopPropagation();
    cancel();
  }

  function end() {
    window.removeEventListener('keydown', esc, true);
    drag = null;
  }
  onDestroy(() => window.removeEventListener('keydown', esc, true));

  const same = (a: TaskDates, b: TaskDates) =>
    a.start === b.start && a.due === b.due && a.startHasTime === b.startHasTime && a.dueHasTime === b.dueHasTime;

  function dblclick(e: MouseEvent) {
    if (performance.now() - endedAt < 500) e.stopPropagation();
  }

  /** The floating label: the dates the bar would get. */
  let dragLabel = $derived.by(() => {
    if (!drag?.next || !span) return '';
    const s = drag.next.start ?? drag.next.due, d = drag.next.due ?? drag.next.start;
    if (!s || !d) return '';
    const a = new Date(s), b = new Date(d);
    return a.toDateString() === b.toDateString() ? fmtShortDate(a) : t('planner.dragLabel', { start: fmtShortDate(a), due: fmtShortDate(b) });
  });
</script>

{#if span}
  <div class="bar {task.status}" class:overdue class:fits class:dragging={!!drag?.next} class:resizing={!!drag?.next && drag.mode !== 'move'}
    data-task={task.id} style="left:{x}px; width:{width}px" aria-hidden="true"
    onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={cancel} onlostpointercapture={cancel} ondblclick={dblclick}>
    {#if handles}
      <span class="edge start" data-edge="start"></span>
      <span class="edge due" data-edge="due"></span>
    {/if}
    {#if fits}
      <span class="title">{task.title}</span>
      {#if blocked}<BlockedBadge count={task.blockedBy} titles={blockers} />{/if}
    {/if}
    {#if seg}
      <span class="progress">
        {#if seg.done}<span class="done" style="width:{pct(seg.done)}"></span>{/if}
        {#if seg.inProgress}<span class="doing" style="width:{pct(seg.inProgress)}"></span>{/if}
      </span>
    {/if}
  </div>
  {#if dragLabel}
    <div class="drag-label" style="left:{x}px">{dragLabel}</div>
  {/if}
  {#if !fits}
    <div class="after" style="left:{x + width + 6}px" aria-hidden="true">
      <span class="title">{task.title}</span>
      {#if blocked}<BlockedBadge count={task.blockedBy} titles={blockers} />{/if}
    </div>
  {/if}
{/if}

<style>
  .bar {
    position: absolute; top: 6px; height: 20px; min-width: 3px;
    display: flex; align-items: center; gap: 4px; overflow: hidden;
    border-radius: 5px;
    background: color-mix(in srgb, var(--chart-neutral) 28%, var(--bg));
    border: 1px solid color-mix(in srgb, var(--chart-neutral) 70%, var(--bg));
    cursor: grab; user-select: none; -webkit-user-select: none; touch-action: none;
  }
  .bar.dragging { cursor: grabbing; box-shadow: 0 2px 6px rgba(0, 0, 0, 0.18); }
  .bar.resizing { cursor: ew-resize; }
  .edge { position: absolute; top: 0; bottom: 0; width: 6px; z-index: 1; cursor: ew-resize; }
  .edge.start { left: 0; }
  .edge.due { right: 0; }
  .edge:hover { background: color-mix(in srgb, var(--text) 18%, transparent); }
  .drag-label {
    position: absolute; top: -15px; z-index: 1; pointer-events: none; white-space: nowrap;
    padding: 1px 6px; border-radius: 4px; font-size: 11px; font-variant-numeric: tabular-nums;
    background: var(--surface-strong); color: var(--text); border: 1px solid var(--border);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }
  .bar.fits { padding: 0 7px; }
  .bar.inProgress {
    background: color-mix(in srgb, var(--progress-active) 42%, var(--bg));
    border-color: var(--progress-active);
  }
  .bar.done { opacity: 0.5; }
  .bar.overdue { border-color: var(--danger); box-shadow: 0 0 0 1px var(--danger); }
  .title { font-size: 12px; font-weight: 500; color: var(--text); white-space: nowrap; }
  .after { position: absolute; top: 6px; height: 20px; display: flex; align-items: center; gap: 4px; pointer-events: none; }
  .after .title { color: var(--muted); }
  .progress { position: absolute; left: 0; right: 0; bottom: 0; height: 3px; display: flex; background: color-mix(in srgb, var(--chart-neutral) 35%, transparent); }
  .progress span { flex: none; height: 100%; }
  .progress .done { background: var(--status-done); }
  .progress .doing { background: var(--progress-active); }
</style>
