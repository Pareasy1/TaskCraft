<script lang="ts">
  import { tick } from 'svelte';
  import { ChevronRight } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { hotkeyLabel, isMod, modLabel } from '../../lib/platform';
  import { isMenuKey } from '../../lib/menu.svelte';
  import { barSpan, dayDate, dayNumber, dayToX, DAY_WIDTH, dragResult, ROW_H, type TimelineRow, type Zoom } from '../../lib/timeline';
  import type { TaskDates } from '../../lib/actions';
  import { isOverdue, STATUS_LABEL } from '../../lib/tasks';
  import { fmtSpan } from '../../lib/format';
  import type { Task } from '../../lib/types';
  import PlannerBar from './PlannerBar.svelte';

  // The name column (sticky) and the bar lanes, one ROW_H-tall row each: row i sits at y = i × ROW_H.
  let { rows, range, zoom, now, nameW, blockers, onopen, ontoggle, onreveal, onpreview, onsave, onlink, onmenu }: {
    rows: TimelineRow<Task>[];
    range: { from: number; to: number };
    zoom: Zoom;
    now: Date;
    nameW: number;
    /** Titles of each task's open blockers, by task id. */
    blockers: Map<number, string[]>;
    onopen: (t: Task) => void;
    /** Collapses or expands a group (by key) or a parent (by id). */
    ontoggle: (key: string | number) => void;
    /** Scrolls row `i` into view vertically. */
    onreveal: (i: number) => void;
    /** Shows a bar at in-progress dates while it is dragged (null = back to its own). */
    onpreview: (id: number, next: TaskDates | null) => void;
    /** Saves a task's new dates (a finished drag or a keyboard move). */
    onsave: (task: Task, next: TaskDates) => void;
    /** A connector drag from `source`'s bar dropped on task `targetId`'s bar. */
    onlink: (source: Task, targetId: number) => void;
    /** Opens a task's menu: right-click, the menu key or Shift+F10 on its row (`row`). */
    onmenu: (task: Task, e: MouseEvent | KeyboardEvent, row: HTMLElement) => void;
  } = $props();

  let w = $derived(DAY_WIDTH[zoom]);
  let laneW = $derived((range.to - range.from + 1) * w);
  let today = $derived(dayNumber(now));

  // Weekend tint: one block per Saturday–Sunday pair (clipped to the range).
  let weekends = $derived.by(() => {
    const out: { x: number; width: number }[] = [];
    for (let n = range.from; n <= range.to; n++) {
      const wd = dayDate(n).getDay();
      if (wd === 6 || (wd === 0 && n === range.from)) {
        const days = wd === 6 && n < range.to ? 2 : 1;
        out.push({ x: dayToX(n, range, zoom), width: days * w });
      }
    }
    return out;
  });

  const keyOf = (r: TimelineRow<Task>) => (r.kind === 'group' ? r.key : `t:${r.task.id}`);
  let focusKey = $state<string | null>(null);
  let stop = $derived(rows.some((r) => keyOf(r) === focusKey) ? focusKey : rows.length ? keyOf(rows[0]) : null);
  let listEl = $state<HTMLDivElement | undefined>();

  async function focusRow(i: number) {
    const r = rows[Math.max(0, Math.min(i, rows.length - 1))];
    if (!r) return;
    focusKey = keyOf(r);
    await tick();
    const idx = rows.indexOf(r);
    onreveal(idx);
    // preventScroll: a plain focus would scroll the timeline back to the row's left edge.
    listEl?.querySelector<HTMLElement>(`[data-row="${CSS.escape(keyOf(r))}"]`)?.focus({ preventScroll: true });
  }

  function label(r: TimelineRow<Task>): string {
    if (r.kind === 'group') return `${r.label || '—'}, ${r.count}`;
    const x = r.task;
    const span = barSpan(x);
    const parts = [x.title, STATUS_LABEL[x.status], span ? fmtSpan(dayDate(span.from), dayDate(span.to)) : t('planner.noDates')];
    if (isOverdue(x, now)) parts.push(t('planner.overdue'));
    if (x.blockedBy > 0) parts.push(t('dep.openCount', { n: x.blockedBy }));
    return parts.join(', ');
  }

  const toggleable = (r: TimelineRow<Task>) => r.kind === 'group' || (r.hasChildren && !r.dimmed);
  const toggleKey = (r: TimelineRow<Task>) => (r.kind === 'group' ? r.key : r.task.id);

  /** Alt+← / → moves the focused task's bar a day; with Shift it changes the due instead. */
  function moveKeys(e: KeyboardEvent, r: TimelineRow<Task>) {
    if (r.kind !== 'task' || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') || e.ctrlKey || e.metaKey) return;
    e.preventDefault();
    const next = dragResult(r.task, e.shiftKey ? 'due' : 'move', e.key === 'ArrowLeft' ? -1 : 1);
    const x = r.task;
    if (next.start !== x.start || next.due !== x.due || next.startHasTime !== x.startHasTime || next.dueHasTime !== x.dueHasTime) onsave(x, next);
  }

  function keys(e: KeyboardEvent) {
    const i = rows.findIndex((r) => keyOf(r) === stop);
    const r = rows[i];
    if (!r) return;
    if (isMenuKey(e)) {
      if (r.kind === 'task') onmenu(r.task, e, e.target as HTMLElement);
      return;
    }
    if (e.altKey) return moveKeys(e, r);
    switch (e.key) {
      case 'ArrowUp': focusRow(i - 1); break;
      case 'ArrowDown': focusRow(i + 1); break;
      case 'Home': focusRow(0); break;
      case 'End': focusRow(rows.length - 1); break;
      case 'ArrowLeft':
        if (toggleable(r) && !r.collapsed) ontoggle(toggleKey(r));
        else {
          // Otherwise move to the row that holds this one: the parent, else the group.
          const depth = r.kind === 'group' ? -1 : r.depth;
          for (let j = i - 1; j >= 0; j--) {
            const p = rows[j];
            if (depth === 1 ? p.kind === 'task' && p.depth === 0 : depth === 0 && p.kind === 'group') return void (focusRow(j), e.preventDefault());
          }
        }
        break;
      case 'ArrowRight':
        if (toggleable(r) && r.collapsed) ontoggle(toggleKey(r));
        else if (toggleable(r)) focusRow(i + 1);
        break;
      case 'Enter':
        if (r.kind === 'task') onopen(r.task);
        else if (!isMod(e)) ontoggle(r.key);
        break;
      default: return;
    }
    e.preventDefault();
  }
</script>

<p id="planner-help" class="sr">{t('planner.help', { mod: modLabel('↩'), alt: hotkeyLabel('Alt') })}</p>
<div class="rows" role="tree" aria-label={t('planner.label')} aria-describedby="planner-help" tabindex="-1"
  bind:this={listEl} onkeydown={keys} style="height:{rows.length * ROW_H}px; width:{nameW + laneW}px">
  <div class="bg" style="left:{nameW}px; width:{laneW}px" aria-hidden="true">
    {#each weekends as wk (wk.x)}<span class="weekend" style="left:{wk.x}px; width:{wk.width}px"></span>{/each}
    {#if today >= range.from && today <= range.to}
      <span class="now" style="left:{dayToX(today, range, zoom) + w / 2 - 1}px"></span>
    {/if}
  </div>
  {#each rows as r (keyOf(r))}
    {@const key = keyOf(r)}
    {#if r.kind === 'group'}
      <div class="row group" role="treeitem" aria-level="1" aria-expanded={!r.collapsed} aria-selected={stop === key} aria-label={label(r)}
        tabindex={stop === key ? 0 : -1} data-row={key} style="height:{ROW_H}px"
        onfocus={() => (focusKey = key)}>
        <div class="name" style="width:{nameW}px">
          <button class="chev" class:open={!r.collapsed} tabindex="-1" aria-hidden="true" onclick={() => ontoggle(r.key)}><ChevronRight size={14} /></button>
          <span class="text">{r.label}</span><span class="count">{r.count}</span>
        </div>
      </div>
    {:else}
      <div class="row" class:dimmed={r.dimmed} role="treeitem" aria-level={r.depth + 2} aria-expanded={r.hasChildren && !r.dimmed ? !r.collapsed : undefined}
        aria-selected={stop === key} aria-label={label(r)}
        tabindex={stop === key ? 0 : -1} data-row={key} style="height:{ROW_H}px"
        onfocus={() => (focusKey = key)} ondblclick={() => onopen(r.task)}
        oncontextmenu={(e) => { e.preventDefault(); onmenu(r.task, e, e.currentTarget as HTMLElement); }}>
        <div class="name" style="width:{nameW}px; padding-left:{10 + r.depth * 18}px">
          {#if r.hasChildren && !r.dimmed}
            <button class="chev" class:open={!r.collapsed} tabindex="-1" aria-hidden="true" onclick={() => ontoggle(r.task.id)}><ChevronRight size={14} /></button>
          {:else}
            <span class="chev-space"></span>
          {/if}
          <span class="text" class:done={r.task.status === 'done'}>{r.task.title}</span>
        </div>
        <div class="lane">
          <PlannerBar task={r.task} {range} {zoom} {now} blockers={blockers.get(r.task.id)} {onpreview} {onsave} {onlink} />
        </div>
      </div>
    {/if}
  {/each}
</div>

<style>
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .rows { position: relative; outline: none; }
  .bg { position: absolute; top: 0; bottom: 0; pointer-events: none; }
  .weekend { position: absolute; top: 0; bottom: 0; background: var(--bg-sunken); }
  .now { position: absolute; top: 0; bottom: 0; width: 2px; background: var(--today); z-index: 1; }
  .row { position: relative; display: flex; outline: none; }
  .row:hover .name { background: linear-gradient(var(--hover), var(--hover)), var(--bg); }
  .row:focus-visible .name, .row:focus .name { background: linear-gradient(var(--accent-soft), var(--accent-soft)), var(--bg); }
  .row:focus-visible .name { box-shadow: inset 2px 0 0 var(--accent); }
  .name {
    position: sticky; left: 0; z-index: 3; flex: none; height: 100%;
    display: flex; align-items: center; gap: 4px; padding: 0 10px; min-width: 0;
    background: var(--bg); border-right: 1px solid var(--border);
  }
  .group .name { font-weight: 600; border-top: 1px solid var(--border); }
  .text { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .text.done { color: var(--muted); }
  .count { flex: none; color: var(--muted); font-weight: 400; font-size: 12px; font-variant-numeric: tabular-nums; }
  .dimmed .text { color: var(--muted); }
  .dimmed .lane { opacity: 0.45; }
  .chev { flex: none; display: grid; place-items: center; width: 18px; height: 18px; padding: 0; border: none; background: none; border-radius: 4px; color: var(--muted); }
  .chev:hover { background: var(--hover); }
  .chev :global(svg) { transition: transform 0.12s; }
  .chev.open :global(svg) { transform: rotate(90deg); }
  .chev-space { flex: none; width: 18px; }
  .lane { position: relative; flex: 1; height: 100%; }
  @media (prefers-reduced-motion: reduce) { .chev :global(svg) { transition: none; } }
</style>
