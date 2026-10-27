<script lang="ts" module>
  import { SvelteSet } from 'svelte/reactivity';
  /** Grid parents whose subtasks are hidden, remembered for the session. */
  const collapsed = new SvelteSet<number>();
</script>

<script lang="ts">
  import { t } from '../../lib/i18n';
  import { ChevronDown, X } from 'lucide-svelte';
  import { api, errorMessage } from '../../lib/api';
  import type { Bucket, Plan, Priority, Task, TaskLabel, TaskStatus } from '../../lib/types';
  import { ALL_COLUMNS, gridRows, loadColumns, nextSort, saveColumns, type ChartFilter, type GridColumn, type GridSort } from '../../lib/grid';
  import { bucketMove, bucketUndo, editInput } from '../../lib/gridEdit';
  import { startAfterDue, taskInput } from '../../lib/edit';
  import { changePriority, changeStatus, quickAddTask } from '../../lib/actions';
  import { statusCtx } from '../../lib/subtasks';
  import { STATUS_LABEL, type StatusFilter } from '../../lib/tasks';
  import { notify } from '../../lib/toast.svelte';
  import { PopupMenu } from '../../lib/menu.svelte';
  import Menu from '../Menu.svelte';
  import HighlightInput from '../../quick/HighlightInput.svelte';
  import Doodle from '../tasks/Doodle.svelte';
  import GridTable from './GridTable.svelte';
  import type { Span } from '../../lib/types';

  let {
    tasks,
    plans,
    buckets,
    scope,
    onopen,
    onerror,
    input = $bindable<HTMLInputElement | undefined>(),
    active = $bindable<number[]>([]),
  }: {
    /** Every task; the grid picks the scope's. */
    tasks: Task[];
    plans: Plan[];
    /** The scoped plan's buckets (empty in All tasks). */
    buckets: Bucket[];
    /** Every label, and the first day of the week: for the charts panel (Task 5). */
    labels: TaskLabel[];
    weekStart: 0 | 1;
    scope: number | 'all';
    onopen: (t: Task) => void;
    onerror: (msg: string) => void;
    /** The "New task" input, focused by Mod+N. */
    input?: HTMLInputElement;
    /** Label filter (ids), shared with the sidebar and the Tasks page. */
    active?: number[];
  } = $props();

  const STATUS_FILTERS: [StatusFilter, string][] = [
    ['any', t('status.any')], ['todo', t('status.todo')], ['inProgress', t('status.inProgress')], ['done', t('status.done')],
  ];
  let plan = $derived(scope === 'all' ? null : plans.find((p) => p.id === scope) ?? null);
  let status = $state<StatusFilter>('any');
  let sort = $state<GridSort>(null);
  let chart = $state<ChartFilter | null>(null);
  let columns = $state<GridColumn[]>([]);
  // Per scope: its remembered columns (Plan only in All tasks); the chart filter starts clear.
  $effect.pre(() => {
    const s = scope;
    columns = loadColumns(s).filter((c) => s === 'all' || c !== 'plan');
    chart = null;
    if (sort?.column === 'plan' && s !== 'all') sort = null;
  });

  // In All tasks every plan's buckets are needed (names and order); inside a plan Main has them.
  let allBuckets = $state<Bucket[]>([]);
  $effect(() => {
    if (scope !== 'all') return;
    void tasks; // a refresh may have renamed or added buckets
    const ids = plans.map((p) => p.id);
    Promise.all(ids.map((id) => api.listBuckets(id)))
      .then((lists) => (allBuckets = lists.flat()))
      .catch((e) => onerror(errorMessage(e)));
  });
  let shownBuckets = $derived(scope === 'all' ? allBuckets : buckets);
  let bucketsOf = $derived.by(() => {
    const m = new Map<number, Bucket[]>();
    for (const b of [...shownBuckets].sort((a, b) => a.sort - b.sort)) m.set(b.planId, [...(m.get(b.planId) ?? []), b]);
    return m;
  });

  let rows = $derived(gridRows(tasks, { scope, buckets: shownBuckets, plans, labels: active, status, chart, sort, collapsed }));
  let filtered = $derived(status !== 'any' || active.length > 0 || !!chart);

  // Chip for the chart filter (the charts panel sets it).
  let chip = $derived.by(() => {
    if (!chart) return null;
    if (chart.kind === 'status') return t('table.chartFilter', { chart: t('table.col.status'), value: STATUS_LABEL[chart.value] });
    if (chart.kind === 'priority') return t('table.chartFilter', { chart: t('table.col.priority'), value: t(`priority.${chart.value}`) });
    const id = chart.value;
    const name = scope === 'all' ? plans.find((p) => p.id === id)?.name : shownBuckets.find((b) => b.id === id)?.name;
    return t('table.chartFilter', { chart: scope === 'all' ? t('table.col.plan') : t('table.col.bucket'), value: name ?? '' });
  });

  // ── Columns menu ──
  const colMenu = new PopupMenu<null>();
  let choosable = $derived(ALL_COLUMNS.filter((c) => scope === 'all' || c !== 'plan'));
  function toggleColumn(c: GridColumn) {
    if (c === 'title') return;
    const on = new Set(columns.includes(c) ? columns.filter((x) => x !== c) : [...columns, c]);
    columns = ALL_COLUMNS.filter((x) => on.has(x));
    saveColumns(scope, columns);
  }

  // ── New task ──
  let text = $state('');
  let spans = $state<Span[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const v = text;
    clearTimeout(timer);
    if (!v.trim()) return void (spans = []);
    timer = setTimeout(() => api.parsePreview(v, true).then((p) => (spans = p.spans), () => (spans = [])), 80);
  });
  async function add(e: KeyboardEvent) {
    if (e.key === 'Escape') return void (text = '');
    if (e.key !== 'Enter' || !text.trim()) return;
    try {
      await quickAddTask(text, plan, buckets[0], tasks);
      text = '';
    } catch (err) {
      onerror(errorMessage(err));
    }
  }

  // ── Edits (each offers Undo) ──
  const fail = (e: unknown) => onerror(errorMessage(e));
  async function setStatus(task: Task, s: TaskStatus) {
    try {
      await changeStatus(task, s, statusCtx(task, tasks));
    } catch (e) {
      fail(e);
    }
  }
  async function setPriority(task: Task, p: Priority) {
    try {
      await changePriority(task, p);
    } catch (e) {
      fail(e);
    }
  }
  async function setBucket(task: Task, bucketId: number) {
    const place = bucketMove(task, bucketId, tasks);
    if (!place) return;
    const back = bucketUndo(task, tasks);
    const name = shownBuckets.find((b) => b.id === bucketId)?.name ?? '';
    try {
      await api.moveTask(task.id, place.bucketId, place.before, place.after);
      notify(t('msg.movedTo', { title: task.title, where: name }), () => api.moveTask(task.id, back.bucketId, back.before, back.after));
    } catch (e) {
      fail(e);
    }
  }
  /** Saves a title or date edit; returns an error to show in the cell, or null when saved. */
  async function saveField(task: Task, field: 'title' | 'start' | 'due', value: string): Promise<string | null> {
    const next = editInput(task, field, value);
    if (field === 'title' && !next.title) return t('inspector.titleRequired');
    if (startAfterDue(next)) return t('error.start_after_due');
    const before = taskInput(task);
    try {
      await api.updateTask(task.id, next);
      notify(t('msg.updated', { title: next.title }), () => api.updateTask(task.id, before));
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  }
</script>

<div class="page">
  <div class="head">
    <div class="add">
      <HighlightInput label={t('common.newTask')} bind:value={text} bind:input {spans} placeholder={t('tasks.addPlaceholder')} onkeydown={add} />
    </div>
    <div class="status" role="group" aria-label={t('inspector.status')}>
      {#each STATUS_FILTERS as [v, name]}
        <button aria-pressed={status === v} class:on={status === v} onclick={() => (status = v)}>{name}</button>
      {/each}
    </div>
    <button class="cols" aria-haspopup="menu" aria-expanded={!!colMenu.current}
      onclick={(e) => colMenu.openBelow(null, e.currentTarget, () => true)}>
      {t('table.columns')}<ChevronDown size={13} aria-hidden="true" />
    </button>
  </div>
  {#if chip}
    <div class="chips">
      <span class="chip">{chip}<button class="icon-btn" aria-label={t('table.clearFilter', { filter: chip })} title={t('table.clearFilter', { filter: chip })}
        onclick={() => (chart = null)}><X size={12} aria-hidden="true" /></button></span>
    </div>
  {/if}

  <div class="content">
    {#if rows.length}
      <GridTable {rows} {columns} {sort} {plans} {bucketsOf}
        onsort={(c) => (sort = nextSort(sort, c))}
        ontoggle={(id) => (collapsed.has(id) ? collapsed.delete(id) : collapsed.add(id))}
        onstatus={setStatus} onpriority={setPriority} onbucket={setBucket} onsave={saveField} {onopen} />
    {:else}
      <div class="empty"><Doodle /><p>{filtered ? t('table.emptyFiltered') : t('table.empty')}</p></div>
    {/if}
  </div>
</div>

{#if colMenu.current}
  <Menu menu={colMenu} label={t('table.columnsMenu')}>
    {#each choosable as c (c)}
      <button role="menuitemcheckbox" aria-checked={columns.includes(c)} disabled={c === 'title'} onclick={() => toggleColumn(c)}>
        <span class="check" aria-hidden="true">{columns.includes(c) ? '✓' : ''}</span>{t(`table.col.${c}`)}
      </button>
    {/each}
  </Menu>
{/if}

<style>
  .page { height: 100%; display: flex; flex-direction: column; padding: 16px 24px 0; min-width: 0; }
  .head { display: flex; gap: 12px; align-items: flex-start; padding-bottom: 12px; }
  .add { flex: 1; min-width: 0; max-width: 560px; }
  .status { display: flex; margin-left: auto; margin-top: 3px; background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 8px; padding: 2px; flex: none; }
  .status button { border: none; background: none; padding: 3px 9px; border-radius: 6px; color: var(--muted); font-size: 12px; }
  .status button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .status button:focus-visible { outline: 2px solid var(--accent); }
  .cols { flex: none; margin-top: 3px; display: flex; align-items: center; gap: 4px; border: 1px solid var(--border); background: var(--bg-sunken); border-radius: 8px; padding: 4px 8px 4px 10px; color: var(--text); font-size: 12px; }
  .cols:hover { background: var(--hover); }
  .cols:focus-visible { outline: 2px solid var(--accent); }
  .check { display: inline-block; width: 16px; color: var(--accent); }
  .chips { display: flex; gap: 6px; padding-bottom: 10px; }
  .chip { display: inline-flex; align-items: center; gap: 2px; font-size: 12px; color: var(--accent); background: var(--accent-soft); border-radius: 6px; padding: 1px 1px 1px 8px; }
  .chip .icon-btn { padding: 2px; color: inherit; }
  .content { flex: 1; min-height: 0; overflow: auto; border-top: 1px solid var(--border); }
  .empty { color: var(--muted); text-align: center; margin-top: 48px; }
  .empty p { margin: 0; }
  @media (max-width: 720px) {
    .page { padding: 12px 16px 0; }
    .head { flex-wrap: wrap; }
    .add { flex-basis: 100%; max-width: none; }
    .status { margin-left: 0; }
  }
</style>
