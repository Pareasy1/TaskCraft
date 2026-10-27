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
</script>

<script lang="ts">
  import type { Task } from '../../lib/types';
  import { barSpan, dayToX, DAY_WIDTH, type Zoom } from '../../lib/timeline';
  import { isOverdue } from '../../lib/tasks';
  import { progressSegments } from '../../lib/subtasks';
  import BlockedBadge from '../tasks/BlockedBadge.svelte';

  // One task's bar on its row (decorative: the row carries the accessible name).
  let { task, range, zoom, now, blockers }: {
    task: Task;
    range: { from: number; to: number };
    zoom: Zoom;
    now: Date;
    /** Titles of the open tasks this one waits on, when known. */
    blockers?: string[];
  } = $props();

  let span = $derived(barSpan(task));
  let x = $derived(span ? dayToX(span.from, range, zoom) : 0);
  let width = $derived(span ? (span.to - span.from + 1) * DAY_WIDTH[zoom] : 0);
  let overdue = $derived(isOverdue(task, now));
  let blocked = $derived(task.blockedBy > 0);
  let fits = $derived(textWidth(task.title) + 2 * PAD + (blocked ? BADGE_W : 0) <= width);
  let seg = $derived(task.subtaskTotal ? progressSegments(task) : null);
  const pct = (n: number) => `${(n / task.subtaskTotal) * 100}%`;
</script>

{#if span}
  <div class="bar {task.status}" class:overdue class:fits data-task={task.id} style="left:{x}px; width:{width}px" aria-hidden="true">
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
