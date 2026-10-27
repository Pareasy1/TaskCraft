<script lang="ts">
  import { tick } from 'svelte';
  import { Paperclip } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { isMod, modLabel } from '../../lib/platform';
  import { nextCell, type GridColumn, type GridRow, type GridSort } from '../../lib/grid';
  import { isOverdue, nextStatus, STATUSES, STATUS_LABEL } from '../../lib/tasks';
  import { progressLabel } from '../../lib/subtasks';
  import { fmtShortDate, fmtTime } from '../../lib/format';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { PRIORITIES, type Bucket, type Plan, type Priority, type Task, type TaskStatus } from '../../lib/types';
  import Menu from '../Menu.svelte';
  import StatusGlyph from '../tasks/StatusGlyph.svelte';
  import PriorityMark from '../tasks/PriorityMark.svelte';
  import LabelChip from '../tasks/LabelChip.svelte';
  import ProgressBar from '../tasks/ProgressBar.svelte';
  import GridTitleCell from './GridTitleCell.svelte';
  import GridDateCell from './GridDateCell.svelte';

  let {
    rows,
    columns,
    sort,
    plans,
    bucketsOf,
    onsort,
    ontoggle,
    onstatus,
    onpriority,
    onbucket,
    onsave,
    onopen,
  }: {
    rows: GridRow<Task>[];
    columns: GridColumn[];
    sort: GridSort;
    plans: Plan[];
    /** Buckets by plan id, in board order. */
    bucketsOf: Map<number, Bucket[]>;
    onsort: (c: GridColumn) => void;
    ontoggle: (id: number) => void;
    onstatus: (task: Task, s: TaskStatus) => void;
    onpriority: (task: Task, p: Priority) => void;
    onbucket: (task: Task, bucketId: number) => void;
    /** Saves a title or date; resolves to an error message, or null when saved. */
    onsave: (task: Task, field: 'title' | 'start' | 'due', value: string) => Promise<string | null>;
    onopen: (task: Task) => void;
  } = $props();

  let now = $derived.by(() => (void rows, new Date()));
  let planById = $derived(new Map(plans.map((p) => [p.id, p])));
  let bucketById = $derived(new Map([...bucketsOf.values()].flat().map((b) => [b.id, b])));

  // The active cell, by task (null = header row) and column, so it survives re-sorting.
  let active = $state<{ id: number | null; col: GridColumn }>({ id: null, col: 'title' });
  let editing = $state<{ id: number; col: 'title' | 'start' | 'due' } | null>(null);
  let tableEl = $state<HTMLTableElement | undefined>();

  let rowIndex = $derived.by(() => {
    if (active.id == null) return 0;
    const i = rows.findIndex((r) => r.task.id === active.id);
    return i < 0 ? -1 : i + 1;
  });
  /** The active row's last position (1-based), so a row that went away hands over to its neighbour. */
  let lastRow = 1;
  $effect.pre(() => {
    if (rowIndex > 0) lastRow = rowIndex;
  });
  // The tab stop: the active cell, or (if its row or column went away) the cell nearest to it.
  let stop = $derived.by(() => {
    const col = columns.includes(active.col) ? active.col : columns[0];
    if (rowIndex >= 0) return { id: active.id, col };
    return { id: rows[Math.min(lastRow, rows.length) - 1]?.task.id ?? null, col };
  });
  const key = (id: number | null, col: GridColumn) => `${id ?? 'h'}:${col}`;
  const isStop = (id: number | null, col: GridColumn) => stop.id === id && stop.col === col;

  async function focusCell(id: number | null, col: GridColumn) {
    active = { id, col };
    await tick();
    tableEl?.querySelector<HTMLElement>(`[data-cell="${key(id, col)}"]`)?.focus();
  }
  /** Keeps the active cell in step with mouse clicks and Tab. */
  /** True while keyboard focus is in the grid (a removed row's cell drops it to <body>). */
  let focused = false;
  function focusout(e: FocusEvent) {
    // A null target is a removed element or the window losing focus: the grid keeps its claim.
    // The grid's own popup menu counts as inside: its command may remove the row it was opened on.
    const to = e.relatedTarget as Node | null;
    if (to && !tableEl?.contains(to) && !menu.el?.contains(to)) focused = false;
  }
  // When an edit or a refresh removes the focused cell, focus moves to the new tab stop.
  $effect(() => {
    const s = stop;
    if (!focused || editing) return;
    tick().then(() => {
      const a = document.activeElement;
      if (focused && (!a || a === document.body)) tableEl?.querySelector<HTMLElement>(`[data-cell="${key(s.id, s.col)}"]`)?.focus();
    });
  });
  function focusin(e: FocusEvent) {
    focused = true;
    const cell = (e.target as HTMLElement).closest<HTMLElement>('[data-cell]');
    if (!cell) return;
    const [id, col] = cell.dataset.cell!.split(':');
    active = { id: id === 'h' ? null : Number(id), col: col as GridColumn };
  }

  // ── Menus (status, priority, bucket) ──
  type MenuKind = 'status' | 'priority' | 'bucket';
  const menu = new PopupMenu<{ task: Task; kind: MenuKind }>();
  function openMenu(task: Task, kind: MenuKind, cell: HTMLElement) {
    const r = cell.getBoundingClientRect();
    menu.open({ task, kind }, r.left, r.bottom + 2, cell);
  }
  /** Runs a menu command on the task as it is now (data may have refreshed since the menu opened). */
  const fresh = (fn: (task: Task) => void) => ({ task }: { task: Task }) => {
    const now = rows.find((r) => r.task.id === task.id)?.task;
    if (now) fn(now);
  };
  const cellEl = (id: number, col: GridColumn) => tableEl?.querySelector<HTMLElement>(`[data-cell="${key(id, col)}"]`);

  const editable = (r: GridRow<Task>, col: GridColumn) =>
    col === 'title' || col === 'status' || col === 'priority' || col === 'start' || col === 'due' || (col === 'bucket' && r.task.parentId == null);

  function edit(r: GridRow<Task>, col: GridColumn) {
    const task = r.task;
    if (col === 'title' || col === 'start' || col === 'due') editing = { id: task.id, col };
    else if (col === 'status' || col === 'priority' || (col === 'bucket' && task.parentId == null)) {
      const el = cellEl(task.id, col);
      if (el) openMenu(task, col, el);
    } else onopen(task);
  }
  function endEdit() {
    const e = editing;
    editing = null;
    if (e) focusCell(e.id, e.col);
  }

  function keys(e: KeyboardEvent) {
    if (editing || (e.target as HTMLElement).closest('input')) return;
    const col = stop.col;
    const ci = columns.indexOf(col);
    const ri = stop.id == null ? 0 : rows.findIndex((r) => r.task.id === stop.id) + 1;
    const r = ri > 0 ? rows[ri - 1] : null;
    if (e.key === 'Enter' && isMod(e)) {
      if (r) onopen(r.task);
    } else if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) {
      if (e.altKey || isMod(e)) return;
      // Left/Right on a parent's title collapse or expand it first (dimmed parents always stay open).
      if (r && col === 'title' && r.hasChildren && !r.dimmed && ((e.key === 'ArrowLeft' && !r.collapsed) || (e.key === 'ArrowRight' && r.collapsed))) {
        ontoggle(r.task.id);
      } else {
        const next = nextCell({ row: ri, col: ci }, e.key, { rows: rows.length + 1, cols: columns.length });
        focusCell(next.row === 0 ? null : rows[next.row - 1].task.id, columns[next.col]);
      }
    } else if (e.key === 'Enter' || e.key === 'F2') {
      if (!r) {
        if (e.key === 'Enter') onsort(col);
      } else edit(r, col);
    } else if (e.key === ' ') {
      if (!r) onsort(col);
      else if (col === 'status') onstatus(r.task, nextStatus(r.task.status));
      // Elsewhere Space does nothing (and must not scroll the table).
    } else return;
    e.preventDefault();
  }

  // ── Display helpers ──
  const when = (iso: string | null, hasTime: boolean) => (iso ? `${fmtShortDate(new Date(iso))}${hasTime ? `, ${fmtTime(new Date(iso))}` : ''}` : '');
  const sortOf = (c: GridColumn) => (sort?.column === c ? (sort.dir === 'asc' ? 'ascending' : 'descending') : 'none');
  const sortHint = (c: GridColumn) => {
    const name = t(`table.col.${c}`);
    return sort?.column !== c ? t('table.sortNone', { col: name }) : sort.dir === 'asc' ? t('table.sortAsc', { col: name }) : t('table.sortDesc', { col: name });
  };
</script>

<p id="grid-help" class="sr">{t('table.help', { mod: modLabel('↩') })}</p>
<!-- svelte-ignore a11y_no_noninteractive_element_interactions (an ARIA grid takes its own arrow keys) -->
<table bind:this={tableEl} role="grid" aria-label={t('table.label')} aria-describedby="grid-help" aria-rowcount={rows.length + 1} aria-colcount={columns.length}
  onkeydown={keys} onfocusin={focusin} onfocusout={focusout}>
  <thead>
    <tr aria-rowindex={1}>
      {#each columns as c, i (c)}
        <th class="col-{c}" aria-sort={sortOf(c)} aria-colindex={i + 1}>
          <button data-cell={key(null, c)} tabindex={isStop(null, c) ? 0 : -1} title={sortHint(c)} onclick={() => onsort(c)}>
            {t(`table.col.${c}`)}<span class="arrow" aria-hidden="true">{sort?.column === c ? (sort.dir === 'asc' ? '↑' : '↓') : ''}</span>
          </button>
        </th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each rows as r, ri (r.task.id)}
      {@const task = r.task}
      <tr aria-rowindex={ri + 2} class:dimmed={r.dimmed} class:sub={r.depth === 1} ondblclick={() => onopen(task)}>
        {#each columns as c, ci (c)}
          <td class="col-{c}" data-cell={key(task.id, c)} tabindex={isStop(task.id, c) ? 0 : -1} aria-colindex={ci + 1}
            aria-readonly={editable(r, c) ? undefined : 'true'}
            aria-expanded={c === 'title' && r.hasChildren && !r.dimmed ? !r.collapsed : undefined}>
            {#if c === 'title'}
              <GridTitleCell row={r} editing={editing?.id === task.id && editing.col === 'title'} ontoggle={() => ontoggle(task.id)}
                onsave={(v) => onsave(task, 'title', v)} ondone={endEdit} />
            {:else if c === 'status'}
              <button class="cellbtn" tabindex="-1" onclick={(e) => openMenu(task, 'status', e.currentTarget.closest('td')!)}>
                <StatusGlyph status={task.status} size={14} /><span>{STATUS_LABEL[task.status]}</span>
              </button>
            {:else if c === 'priority'}
              <button class="cellbtn" tabindex="-1" onclick={(e) => openMenu(task, 'priority', e.currentTarget.closest('td')!)}>
                <PriorityMark priority={task.priority} /><span>{t(`priority.${task.priority}`)}</span>
              </button>
            {:else if c === 'bucket'}
              {@const name = bucketById.get(task.bucketId)?.name ?? ''}
              {#if task.parentId == null}
                <button class="cellbtn" tabindex="-1" onclick={(e) => openMenu(task, 'bucket', e.currentTarget.closest('td')!)}>{name}</button>
              {:else}
                <span class="faint" title={t('table.bucketFollows')}>{name}</span>
              {/if}
            {:else if c === 'plan'}
              {@const p = planById.get(task.planId)}
              {#if p}<span class="plan"><span class="dot" style="background:{p.color}" aria-hidden="true"></span>{p.name}</span>{/if}
            {:else if c === 'labels'}
              <span class="labels">{#each task.labels as l (l.id)}<LabelChip name={l.name} color={l.color} />{/each}</span>
            {:else if c === 'start' || c === 'due'}
              {#if editing?.id === task.id && editing.col === c}
                <GridDateCell iso={c === 'start' ? task.start : task.due} label={t(`table.col.${c}`)} onsave={(v) => onsave(task, c, v)} ondone={endEdit} />
              {:else if c === 'start'}
                <span class="date">{when(task.start, task.startHasTime)}</span>
              {:else}
                <span class="date" class:late={isOverdue(task, now)}>{when(task.due, task.dueHasTime)}</span>
              {/if}
            {:else if c === 'progress'}
              {#if task.subtaskTotal}
                <span class="progress"><span class="bar"><ProgressBar {task} /></span><span class="n">{progressLabel(task)}</span></span>
              {/if}
            {:else if c === 'attachments'}
              {#if task.attachmentCount}<span class="n"><Paperclip size={11} aria-hidden="true" /> {task.attachmentCount}</span>{/if}
            {/if}
          </td>
        {/each}
      </tr>
    {/each}
  </tbody>
</table>

{#if menu.current}
  {@const m = menu.current.target}
  <Menu {menu} label={m.kind === 'status' ? t('table.col.status') : m.kind === 'priority' ? t('table.col.priority') : t('table.col.bucket')}>
    {#if m.kind === 'status'}
      {#each STATUSES as s (s)}
        <button role="menuitemradio" aria-checked={m.task.status === s} onclick={() => menu.act(fresh((task) => task.status !== s && onstatus(task, s)))}>
          <span class="check" aria-hidden="true">{m.task.status === s ? '✓' : ''}</span>{STATUS_LABEL[s]}
        </button>
      {/each}
    {:else if m.kind === 'priority'}
      {#each PRIORITIES as p (p)}
        <button role="menuitemradio" aria-checked={m.task.priority === p} onclick={() => menu.act(fresh((task) => onpriority(task, p)))}>
          <span class="check" aria-hidden="true">{m.task.priority === p ? '✓' : ''}</span>{t(`priority.${p}`)}
        </button>
      {/each}
    {:else}
      {#each bucketsOf.get(m.task.planId) ?? [] as b (b.id)}
        <button role="menuitemradio" aria-checked={m.task.bucketId === b.id} onclick={() => menu.act(fresh((task) => onbucket(task, b.id)))}>
          <span class="check" aria-hidden="true">{m.task.bucketId === b.id ? '✓' : ''}</span>{b.name}
        </button>
      {/each}
    {/if}
  </Menu>
{/if}

<style>
  table { border-collapse: separate; border-spacing: 0; width: 100%; font-size: 13px; }
  th {
    position: sticky; top: 0; z-index: 1; background: var(--bg); text-align: left; font-weight: 600; font-size: 12px; color: var(--muted);
    border-bottom: 1px solid var(--border); padding: 0; white-space: nowrap;
  }
  th button { border: none; background: none; color: inherit; font: inherit; padding: 8px 10px; width: 100%; text-align: left; display: flex; gap: 4px; }
  th button:hover { color: var(--text); }
  th[aria-sort='ascending'], th[aria-sort='descending'] { color: var(--text); }
  .arrow { color: var(--accent); min-width: 8px; }
  td { padding: 5px 10px; border-bottom: 1px solid var(--border); vertical-align: middle; white-space: nowrap; height: 32px; }
  td.col-title { white-space: normal; min-width: 240px; width: 40%; }
  th button:focus-visible, td:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  td:focus { outline: 2px solid var(--accent); outline-offset: -2px; }
  tr:hover td { background: var(--hover); }
  tr.dimmed td { color: var(--faint); }
  .cellbtn { border: none; background: none; padding: 0; color: inherit; font: inherit; width: 100%; display: flex; align-items: center; gap: 6px; text-align: left; }
  .faint { color: var(--faint); }
  .plan { display: inline-flex; align-items: center; gap: 6px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; flex: none; }
  .labels { display: inline-flex; gap: 6px; }
  .date { font-variant-numeric: tabular-nums; color: var(--muted); }
  .date.late { color: var(--danger); }
  .progress { display: inline-flex; align-items: center; gap: 8px; }
  .bar { width: 56px; }
  .n { display: inline-flex; align-items: center; gap: 3px; font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .check { display: inline-block; width: 16px; color: var(--accent); }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
</style>
