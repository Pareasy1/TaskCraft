<script lang="ts">
  import { Repeat } from 'lucide-svelte';
  import { flip } from 'svelte/animate';
  import type { Task, TaskStatus } from '../../lib/types';
  import { boardColumns, isOverdue, STATUSES, STATUS_LABEL } from '../../lib/tasks';
  import { fmtShortDate, fmtTime } from '../../lib/format';
  import StatusGlyph from './StatusGlyph.svelte';
  import LabelChip from './LabelChip.svelte';

  let {
    tasks,
    now,
    activeLabels,
    onstatus,
    onopen,
    onlabel,
  }: {
    tasks: Task[];
    now: Date;
    activeLabels: number[];
    onstatus: (t: Task, s: TaskStatus) => void;
    onopen: (t: Task) => void;
    onlabel: (id: number) => void;
  } = $props();

  let showAllDone = $state(false);
  let dragging = $state<number | null>(null);
  let over = $state<TaskStatus | null>(null);
  let columns = $derived(boardColumns(tasks, { showAllDone }));
  const reduce = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

  const due = (t: Task) =>
    t.due ? `${fmtShortDate(new Date(t.due))}${t.dueHasTime ? `, ${fmtTime(new Date(t.due))}` : ''}` : '';

  function drop(e: DragEvent, status: TaskStatus) {
    e.preventDefault();
    over = null;
    const id = Number(e.dataTransfer?.getData('text/plain'));
    const t = tasks.find((x) => x.id === id);
    if (t && t.status !== status) onstatus(t, status);
  }

  function keys(e: KeyboardEvent, t: Task) {
    const i = STATUSES.indexOf(t.status);
    if (e.key === 'ArrowRight' && i < STATUSES.length - 1) onstatus(t, STATUSES[i + 1]);
    else if (e.key === 'ArrowLeft' && i > 0) onstatus(t, STATUSES[i - 1]);
    else if (e.key === 'Enter') onopen(t);
    else return;
    e.preventDefault();
  }
</script>

<div class="board">
  {#each columns as col (col.status)}
    <section
      class="col"
      class:over={over === col.status}
      aria-label={STATUS_LABEL[col.status]}
      ondragover={(e) => { e.preventDefault(); over = col.status; }}
      ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) over = null; }}
      ondrop={(e) => drop(e, col.status)}
    >
      <header>
        <StatusGlyph status={col.status} size={15} />
        <h3>{STATUS_LABEL[col.status]}</h3>
        <span class="n">{col.tasks.length + col.hidden}</span>
      </header>
      <div class="cards">
        {#each col.tasks as t (t.id)}
          <div
            class="card"
            class:dragging={dragging === t.id}
            role="button"
            tabindex="0"
            draggable="true"
            aria-label="{t.title}, {STATUS_LABEL[t.status]}. Arrow keys move between columns."
            animate:flip={{ duration: reduce ? 0 : 180 }}
            ondragstart={(e) => { dragging = t.id; e.dataTransfer?.setData('text/plain', String(t.id)); }}
            ondragend={() => { dragging = null; over = null; }}
            onclick={() => onopen(t)}
            onkeydown={(e) => keys(e, t)}
          >
            <div class="top">
              <StatusGlyph status={t.status} size={16} onchange={(s) => onstatus(t, s)} />
              <span class="title">{t.title}</span>
            </div>
            {#if t.labels.length || t.due || t.recurring}
              <div class="meta">
                {#each t.labels as l (l.id)}
                  <LabelChip name={l.name} color={l.color} active={activeLabels.includes(l.id)} onclick={() => onlabel(l.id)} />
                {/each}
                <span class="spacer"></span>
                {#if t.recurring}<Repeat size={11} class="rep" aria-label="Repeats" />{/if}
                {#if t.due && t.status !== 'done'}<span class="due" class:late={isOverdue(t, now)}>{due(t)}</span>{/if}
                <span class="cal" style="background:{t.color}"></span>
              </div>
            {/if}
          </div>
        {:else}
          <p class="empty">{col.status === 'inProgress' ? 'Drag a task here when you start it.' : col.status === 'done' ? 'Finished tasks land here.' : 'Nothing to do.'}</p>
        {/each}
        {#if col.status === 'done' && (col.hidden || showAllDone)}
          <button class="more" onclick={() => (showAllDone = !showAllDone)}>
            {showAllDone ? 'Show recent only' : `Show ${col.hidden} older`}
          </button>
        {/if}
      </div>
    </section>
  {/each}
</div>

<style>
  .board { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; height: 100%; padding-bottom: 16px; }
  .col {
    background: var(--bg-sunken);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    box-shadow: inset 0 0 0 1px var(--border);
    transition: box-shadow 0.15s;
  }
  .col.over { box-shadow: inset 0 0 0 2px var(--accent); }
  header { display: flex; align-items: center; gap: 7px; padding: 11px 12px 6px; }
  h3 { margin: 0; font-size: 13px; font-weight: 600; }
  .n { color: var(--faint); font-size: 12px; font-variant-numeric: tabular-nums; }
  .cards { flex: 1; overflow-y: auto; padding: 4px 8px 10px; display: flex; flex-direction: column; gap: 6px; }
  .card {
    background: var(--surface-strong);
    border-radius: 9px;
    padding: 9px 10px;
    box-shadow: 0 0 0 1px var(--border), 0 1px 2px rgba(0, 0, 0, 0.05);
  }
  .card:hover { box-shadow: 0 0 0 1px var(--border), 0 3px 10px rgba(0, 0, 0, 0.08); }
  .card:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .card.dragging { opacity: 0.45; }
  @media (prefers-reduced-motion: no-preference) {
    .card { transition: box-shadow 0.15s, transform 0.15s, opacity 0.15s; }
    .card:active { transform: scale(1.015) rotate(-0.4deg); box-shadow: 0 0 0 1px var(--border), 0 10px 24px rgba(0, 0, 0, 0.16); }
  }
  .top { display: flex; align-items: flex-start; gap: 8px; }
  .top :global(.btn-glyph) { margin-top: -1px; }
  .title { flex: 1; min-width: 0; font-size: 13px; line-height: 1.35; overflow-wrap: anywhere; }
  .meta { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; margin: 6px 0 0 24px; }
  .spacer { flex: 1; }
  .meta :global(.rep) { color: var(--faint); }
  .due { font-size: 11.5px; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .due.late { color: var(--danger); }
  .cal { width: 6px; height: 6px; border-radius: 50%; opacity: 0.8; }
  .empty { color: var(--faint); font-size: 12px; text-align: center; margin: 18px 8px; }
  .more { border: none; background: none; color: var(--accent); font-size: 12px; padding: 6px; border-radius: 6px; }
  .more:hover { background: var(--hover); }
  @media (max-width: 720px) { .board { grid-template-columns: 1fr; height: auto; } }
</style>
