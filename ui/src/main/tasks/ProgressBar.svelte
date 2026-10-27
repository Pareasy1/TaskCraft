<script lang="ts">
  import type { TaskStatus } from '../../lib/types';
  import { barTrack, progressSegments } from '../../lib/subtasks';

  // A task's subtask bar (board cards, the Grid's Progress cell). Decorative: callers give the text.
  let { task }: { task: { status: TaskStatus; subtaskTotal: number; subtaskOpen: number; subtaskInProgress: number } } = $props();
  const seg = $derived(progressSegments(task));
  const pct = (n: number) => `${(n / task.subtaskTotal) * 100}%`;
</script>

<!-- Done (green), then in progress (yellow), then the track for what is still to do;
     the track is tinted yellow when the task itself is in progress but no subtask is. -->
<div class="bar" class:started={barTrack(task) === 'started'} aria-hidden="true">
  {#if seg.done}<span class="done" style="width:{pct(seg.done)}"></span>{/if}
  {#if seg.inProgress}<span class="doing" style="width:{pct(seg.inProgress)}"></span>{/if}
</div>

<style>
  .bar { display: flex; gap: 1px; height: 5px; border-radius: 3px; background: var(--border); overflow: hidden; }
  .bar.started { background: color-mix(in srgb, var(--progress-active) 35%, var(--border)); }
  .bar span { flex: none; height: 100%; border-radius: 3px; }
  .bar .done { background: var(--status-done); }
  .bar .doing { background: var(--progress-active); }
</style>
