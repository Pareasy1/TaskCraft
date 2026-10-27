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
  /** The connector dot's centre, past the bar's end (clear of the due handle). */
  const DOT = 7;
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
  let { task, range, zoom, now, blockers, onpreview, onsave, onlink }: {
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
    /** A connector drag dropped on another task's bar: that task now depends on this one. */
    onlink: (source: Task, targetId: number) => void;
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
  let drag = $state<{ mode: DragMode; orig: Task; x0: number; id: number; el: HTMLElement; next: TaskDates | null } | null>(null);
  /** When the last real drag ended: the double-click that may follow is not an "open". */
  let endedAt = 0;
  /** The floating label goes under the bar when there is no room above it (under the sticky header). */
  let labelBelow = $state(false);

  function down(e: PointerEvent) {
    if (e.button !== 0 || e.ctrlKey) return; // Ctrl-click is the macOS context click
    // A drag whose release never arrived (focus lost, pointer gone) ends here. Its original task is
    // the base: `task` may still carry that drag's preview until the next update.
    const base = drag?.orig.id === task.id ? drag.orig : task;
    cancel();
    const el = e.currentTarget as HTMLElement;
    const edge = (e.target as HTMLElement).closest<HTMLElement>('[data-edge]')?.dataset.edge as DragMode | undefined;
    drag = { mode: edge ?? 'move', orig: base, x0: e.clientX, id: e.pointerId, el, next: null };
    const header = el.closest('.scroller')?.querySelector('.header')?.getBoundingClientRect();
    labelBelow = !!header && el.getBoundingClientRect().top - header.bottom < 18;
    window.addEventListener('keydown', esc, true);
    window.addEventListener('blur', cancel);
    try {
      el.setPointerCapture(e.pointerId);
    } catch { /* the pointer is already gone: the button check in move() ends the drag */ }
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    // No primary button held: the release was missed (e.g. it happened in another window).
    if ((e.buttons & 1) === 0) return cancel();
    if (e.pointerId !== drag.id) return;
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

  /** Esc, a cancelled or lost pointer, a lost window focus: the bar goes back where it was, nothing is saved. */
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
    unlisten();
    const d = drag;
    drag = null; // before releasing: the release fires lostpointercapture → cancel(), now a no-op
    try {
      if (d?.el.hasPointerCapture(d.id)) d.el.releasePointerCapture(d.id);
    } catch { /* already released */ }
  }
  function unlisten() {
    window.removeEventListener('keydown', esc, true);
    window.removeEventListener('blur', cancel);
  }
  onDestroy(unlisten);

  // ── Drawing a dependency: drag the connector dot at the bar's right end onto another bar ──
  let link = $state<{ id: number; el: HTMLElement; lane: HTMLElement; cx: number; cy: number; px: number; py: number; moved: boolean } | null>(null);
  let linkTarget: HTMLElement | null = null;

  function linkDown(e: PointerEvent) {
    if (e.button !== 0 || e.ctrlKey) return;
    e.stopPropagation();
    e.preventDefault();
    linkCancel();
    const el = e.currentTarget as HTMLElement;
    const lane = el.parentElement!;
    const r = lane.getBoundingClientRect();
    link = { id: e.pointerId, el, lane, cx: e.clientX, cy: e.clientY, px: e.clientX - r.left, py: e.clientY - r.top, moved: false };
    window.addEventListener('keydown', linkEsc, true);
    window.addEventListener('blur', linkCancel);
    try {
      el.setPointerCapture(e.pointerId);
    } catch { /* the pointer is already gone: the button check in linkMove() ends the drag */ }
  }

  /** The bar of another task under a viewport point, if any. */
  function barAt(cx: number, cy: number): HTMLElement | null {
    const hit = document.elementFromPoint(cx, cy)?.closest<HTMLElement>('.bar[data-task]') ?? null;
    return hit && Number(hit.dataset.task) !== task.id ? hit : null;
  }
  function mark(el: HTMLElement | null) {
    if (el === linkTarget) return;
    linkTarget?.classList.remove('link-target');
    el?.classList.add('link-target');
    linkTarget = el;
  }

  function linkMove(e: PointerEvent) {
    if (!link) return;
    if ((e.buttons & 1) === 0) return linkCancel();
    if (e.pointerId !== link.id) return;
    const r = link.lane.getBoundingClientRect();
    link.px = e.clientX - r.left;
    link.py = e.clientY - r.top;
    link.moved ||= Math.hypot(e.clientX - link.cx, e.clientY - link.cy) > 3;
    mark(link.moved ? barAt(e.clientX, e.clientY) : null);
  }

  function linkUp(e: PointerEvent) {
    if (!link || e.pointerId !== link.id) return;
    const moved = link.moved;
    linkEnd();
    if (!moved) return; // a click on the dot: nothing to draw
    // Any other bar, self included: the store refuses a self link with a message.
    const hit = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('.bar[data-task]');
    if (hit) onlink(task, Number(hit.dataset.task));
  }

  /** Esc, a cancelled or lost pointer, a lost window focus: nothing is drawn. */
  function linkCancel() {
    if (link) linkEnd();
  }
  function linkEsc(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !link) return;
    e.preventDefault();
    e.stopPropagation();
    linkCancel();
  }
  function linkEnd() {
    linkUnlisten();
    mark(null);
    const l = link;
    link = null; // before releasing: the release fires lostpointercapture → linkCancel(), now a no-op
    try {
      if (l?.el.hasPointerCapture(l.id)) l.el.releasePointerCapture(l.id);
    } catch { /* already released */ }
  }
  function linkUnlisten() {
    window.removeEventListener('keydown', linkEsc, true);
    window.removeEventListener('blur', linkCancel);
  }
  onDestroy(() => {
    linkUnlisten();
    mark(null);
  });

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
    <div class="drag-label" class:below={labelBelow} style="left:{x}px">{dragLabel}</div>
  {/if}
  <span class="dot" class:linking={!!link} style="left:{x + width + DOT}px" title={t('dep.connector')} aria-hidden="true"
    onpointerdown={linkDown} onpointermove={linkMove} onpointerup={linkUp} onpointercancel={linkCancel} onlostpointercapture={linkCancel}></span>
  {#if link?.moved}
    <svg class="link" aria-hidden="true"><line x1={x + width + DOT} y1={16} x2={link.px} y2={link.py} /></svg>
  {/if}
  {#if !fits}
    <div class="after" style="left:{x + width + 2 * DOT + 2}px" aria-hidden="true">
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
    position: absolute; top: -15px; z-index: 2; pointer-events: none; white-space: nowrap;
    padding: 1px 6px; border-radius: 4px; font-size: 11px; font-variant-numeric: tabular-nums;
    background: var(--surface-strong); color: var(--text); border: 1px solid var(--border);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }
  .drag-label.below { top: 29px; }
  .bar.fits { padding: 0 7px; }
  .bar.inProgress {
    background: color-mix(in srgb, var(--progress-active) 42%, var(--bg));
    border-color: var(--progress-active);
  }
  .bar.done { opacity: 0.5; }
  .bar.overdue { border-color: var(--danger); box-shadow: 0 0 0 1px var(--danger); }
  .bar:global(.link-target) { box-shadow: 0 0 0 2px var(--accent); }
  /* The connector: shown on the row's hover / focus, above the arrows (z 1), under the sticky names (z 3). */
  .dot {
    position: absolute; top: 11px; z-index: 2; width: 10px; height: 10px; margin-left: -5px;
    border-radius: 50%; background: var(--bg); border: 2px solid var(--accent); box-sizing: border-box;
    cursor: crosshair; opacity: 0; touch-action: none;
  }
  :global(.row:hover) > :global(.lane) > .dot, :global(.row:focus-visible) > :global(.lane) > .dot, .dot.linking { opacity: 1; }
  .link { position: absolute; left: 0; top: 0; width: 1px; height: 1px; overflow: visible; z-index: 2; pointer-events: none; }
  .link line { stroke: var(--accent); stroke-width: 1.5; stroke-dasharray: 4 3; }
  .title { font-size: 12px; font-weight: 500; color: var(--text); white-space: nowrap; }
  .after { position: absolute; top: 6px; height: 20px; display: flex; align-items: center; gap: 4px; pointer-events: none; }
  .after .title { color: var(--muted); }
  .progress { position: absolute; left: 0; right: 0; bottom: 0; height: 3px; display: flex; background: color-mix(in srgb, var(--chart-neutral) 35%, transparent); }
  .progress span { flex: none; height: 100%; }
  .progress .done { background: var(--status-done); }
  .progress .doing { background: var(--progress-active); }
</style>
