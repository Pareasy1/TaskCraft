<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { isMod } from '../../lib/platform';
  import { fmtShortDate } from '../../lib/format';
  import { dayDate } from '../../lib/timeline';
  import type { Task } from '../../lib/types';

  // The scope's tasks without dates, on the right of the timeline. A pointer drag onto the grid
  // gives a task that day (all-day); Return puts it on today.
  let { tasks, open = $bindable(true), today, dayAt, onhover, onplace, onopen }: {
    /** Undated tasks of the scope, filters applied. */
    tasks: Task[];
    open?: boolean;
    today: number;
    /** The timeline day under a viewport point, or null when the point is off the grid. */
    dayAt: (clientX: number, clientY: number) => number | null;
    /** The day a drag currently points at (null = none), for the grid's highlight. */
    onhover: (day: number | null) => void;
    onplace: (task: Task, day: number) => void;
    onopen: (task: Task) => void;
  } = $props();

  // ── Keyboard: ↑ / ↓ between rows, Return puts the task on today, Mod+Return opens it ──
  let listEl = $state<HTMLUListElement | undefined>();
  const items = () => [...(listEl?.querySelectorAll<HTMLElement>('[data-undated]') ?? [])];
  /** Roving tab stop: the focused task, else the first. */
  let focusId = $state<number | null>(null);
  let stop = $derived(tasks.some((x) => x.id === focusId) ? focusId : tasks[0]?.id ?? null);

  async function keys(e: KeyboardEvent, i: number, task: Task) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      items()[Math.max(0, Math.min(i + (e.key === 'ArrowDown' ? 1 : -1), tasks.length - 1))]?.focus();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (isMod(e)) return onopen(task);
      onplace(task, today);
      // The task leaves the list: focus stays at the same place (the next task, else the last).
      await tick();
      const left = items();
      left[Math.min(i, left.length - 1)]?.focus();
    }
  }

  // ── Pointer drag onto the grid (pointer events; capture keeps it going outside the panel) ──
  const SLOP = 4;
  let drag = $state<{ task: Task; id: number; el: HTMLElement; x0: number; y0: number; x: number; y: number; moved: boolean; day: number | null } | null>(null);

  function down(e: PointerEvent, task: Task) {
    if (e.button !== 0 || e.ctrlKey) return;
    end(); // a drag whose release never arrived (focus lost, pointer gone) ends here, without placing
    const el = e.currentTarget as HTMLElement;
    drag = { task, id: e.pointerId, el, x0: e.clientX, y0: e.clientY, x: e.clientX, y: e.clientY, moved: false, day: null };
    window.addEventListener('keydown', esc, true);
    window.addEventListener('blur', end);
    try {
      el.setPointerCapture(e.pointerId);
    } catch { /* the pointer is already gone: the button check in move() ends the drag */ }
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    // No primary button held: the release was missed (e.g. it happened in another window).
    if ((e.buttons & 1) === 0) return end();
    if (e.pointerId !== drag.id) return;
    drag.x = e.clientX;
    drag.y = e.clientY;
    if (!drag.moved && Math.hypot(e.clientX - drag.x0, e.clientY - drag.y0) < SLOP) return;
    drag.moved = true;
    const day = dayAt(e.clientX, e.clientY);
    if (day !== drag.day) onhover((drag.day = day));
  }

  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const { task, moved, day } = drag;
    end();
    if (moved && day !== null) onplace(task, day);
  }

  function esc(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !drag) return;
    e.preventDefault();
    e.stopPropagation();
    end();
  }

  /** Ends a drag without placing anything (Esc, lost pointer or window focus) or after a drop. */
  function end() {
    unlisten();
    const d = drag;
    if (!d) return;
    drag = null; // before releasing: the release fires lostpointercapture → end(), now a no-op
    if (d.day != null) onhover(null);
    try {
      if (d.el.hasPointerCapture(d.id)) d.el.releasePointerCapture(d.id);
    } catch { /* already released */ }
  }
  function unlisten() {
    window.removeEventListener('keydown', esc, true);
    window.removeEventListener('blur', end);
  }
  onDestroy(unlisten);

  function toggle() {
    open = !open;
  }
</script>

{#if open}
  <aside class="panel" aria-labelledby="undated-title">
    <div class="head">
      <h2 id="undated-title">{t('planner.noDates')}</h2>
      <span class="count">{tasks.length}</span>
      <button class="toggle" aria-expanded="true" aria-label={t('planner.hideNoDates')} title={t('planner.hideNoDates')} onclick={toggle}><ChevronRight size={14} /></button>
    </div>
    {#if tasks.length}
      <p id="undated-help" class="hint">{t('planner.undatedHelp')}</p>
      <ul bind:this={listEl} role="listbox" aria-labelledby="undated-title" aria-describedby="undated-help">
        {#each tasks as task, i (task.id)}
          <li data-undated role="option" aria-selected={stop === task.id} tabindex={stop === task.id ? 0 : -1}
            onfocus={() => (focusId = task.id)} class:lifted={drag?.moved && drag.task.id === task.id}
            onkeydown={(e) => keys(e, i, task)} ondblclick={() => onopen(task)}
            onpointerdown={(e) => down(e, task)} onpointermove={move} onpointerup={up} onpointercancel={end} onlostpointercapture={end}>
            <span class="dot {task.status}" aria-hidden="true"></span>
            <span class="title" class:done={task.status === 'done'}>{task.title}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">{t('planner.allDated')}</p>
    {/if}
  </aside>
{:else}
  <aside class="rail" aria-label={t('planner.noDates')}>
    <button class="toggle" aria-expanded="false" aria-label={t('planner.showNoDates')} title={t('planner.showNoDates')} onclick={toggle}>
      <ChevronLeft size={14} />
      <span class="count">{tasks.length}</span>
    </button>
  </aside>
{/if}

{#if drag?.moved}
  <div class="ghost" style="left:{drag.x + 12}px; top:{drag.y + 10}px" aria-hidden="true">
    <span class="title">{drag.task.title}</span>
    <span class="when">{drag.day !== null ? fmtShortDate(dayDate(drag.day)) : t('planner.dropHere')}</span>
  </div>
{/if}

<style>
  .panel { flex: none; width: 220px; display: flex; flex-direction: column; min-height: 0; border-left: 1px solid var(--border); border-top: 1px solid var(--border); background: var(--bg); }
  .rail { flex: none; width: 32px; border-left: 1px solid var(--border); border-top: 1px solid var(--border); background: var(--bg); display: flex; justify-content: center; padding-top: 8px; }
  .head { display: flex; align-items: center; gap: 6px; padding: 10px 8px 6px 12px; }
  h2 { margin: 0; font-size: 13px; font-weight: 600; }
  .count { color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  .toggle { margin-left: auto; display: grid; place-items: center; gap: 2px; padding: 3px; border: none; background: none; border-radius: 4px; color: var(--muted); }
  .rail .toggle { margin: 0; }
  .toggle:hover { background: var(--hover); color: var(--text); }
  .toggle:focus-visible { outline: 2px solid var(--accent); }
  .hint { margin: 0 12px 6px; color: var(--muted); font-size: 11px; line-height: 1.35; }
  ul { list-style: none; margin: 0; padding: 0 6px 12px; overflow-y: auto; flex: 1; min-height: 0; }
  li {
    display: flex; align-items: center; gap: 6px; height: 28px; padding: 0 6px; border-radius: 6px;
    cursor: grab; user-select: none; -webkit-user-select: none; touch-action: none; outline: none;
  }
  li:hover { background: var(--hover); }
  li:focus-visible { background: var(--accent-soft); box-shadow: inset 2px 0 0 var(--accent); }
  li.lifted { opacity: 0.5; cursor: grabbing; }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .title.done { color: var(--muted); }
  .dot { flex: none; width: 8px; height: 8px; border-radius: 50%; background: var(--chart-neutral); }
  .dot.inProgress { background: var(--progress-active); }
  .dot.done { background: var(--status-done); }
  .empty { margin: 4px 12px; color: var(--muted); font-size: 12px; }
  .ghost {
    position: fixed; z-index: 50; pointer-events: none; display: flex; flex-direction: column; gap: 1px; max-width: 220px;
    padding: 4px 8px; border-radius: 6px; background: var(--surface-strong); border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18); font-size: 12px;
  }
  .ghost .title { font-size: 12px; font-weight: 500; }
  .ghost .when { color: var(--muted); font-size: 11px; font-variant-numeric: tabular-nums; }
  @media (max-width: 720px) {
    .panel { width: 160px; }
  }
</style>
