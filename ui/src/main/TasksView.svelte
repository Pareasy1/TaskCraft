<script lang="ts">
  import { t } from '../lib/i18n';
  import { Rows3, Columns3, ChevronDown } from 'lucide-svelte';
  import type { Bucket, ParsedItem, Plan, Task, TaskLabel, TaskStatus } from '../lib/types';
  import { api, errorMessage } from '../lib/api';
  import { describeParsed, fmtDayHeading, fmtShortDate, fmtTime } from '../lib/format';
  import {
    applyFilters, boardColumns, filterTasks, isOverdue, STATUSES, STATUS_LABEL, taskCounts, taskPageGroups,
    type StatusFilter, type TaskGroup, type TaskView,
  } from '../lib/tasks';
  import { dueColumns, labelColumns, planOrder, quickAddNeedsMove, type DueKey, type GroupBy } from '../lib/plans';
  import { PopupMenu } from '../lib/menu.svelte';
  import Menu from './Menu.svelte';
  import HighlightInput from '../quick/HighlightInput.svelte';
  import { changeStatus } from '../lib/actions';
  import TaskRow from './tasks/TaskRow.svelte';
  import TaskBoard, { type BoardColumn } from './tasks/TaskBoard.svelte';
  import BucketBoard from './tasks/BucketBoard.svelte';
  import Doodle from './tasks/Doodle.svelte';
  import LabelChip from './tasks/LabelChip.svelte';

  let {
    tasks,
    labels,
    scope,
    plans,
    buckets,
    weekStart,
    onopen,
    onerror,
    onreload,
    input = $bindable<HTMLInputElement | undefined>(),
    active = $bindable<number[]>([]),
  }: {
    /** The tasks in scope (one plan's, or every task in All tasks). */
    tasks: Task[];
    labels: TaskLabel[];
    scope: number | 'all';
    plans: Plan[];
    /** The scoped plan's buckets (empty in All tasks). */
    buckets: Bucket[];
    /** First day of the week: 0 = Sunday, 1 = Monday. */
    weekStart: 0 | 1;
    onopen: (t: Task) => void;
    onerror: (msg: string) => void;
    /** Re-reads tasks and buckets from the store. */
    onreload: () => void;
    input?: HTMLInputElement;
    /** Label filter (ids); the sidebar's label list edits it too. */
    active?: number[];
  } = $props();

  const VIEW_NAMES: Record<TaskView, string> = {
    all: t('view.all'), today: t('view.today'), upcoming: t('view.upcoming'), nodate: t('view.nodate'), completed: t('view.completed'),
  };
  const STATUS_FILTERS: [StatusFilter, string][] = [
    ['any', t('status.any')], ['todo', t('status.todo')], ['inProgress', t('status.inProgress')], ['done', t('status.done')],
  ];
  const LAYOUT_KEY = 'taskcraft.tasks.layout';

  const stored = (() => {
    try {
      return localStorage.getItem(LAYOUT_KEY);
    } catch {
      return null;
    }
  })();
  let layout = $state<'list' | 'board'>(stored === 'board' ? 'board' : 'list');
  let plan = $derived(scope === 'all' ? null : plans.find((p) => p.id === scope) ?? null);

  // Group by (Board inside a plan), remembered per plan.
  const GROUPS: [GroupBy, string][] = [
    ['bucket', t('groupBy.bucket')], ['status', t('groupBy.status')], ['due', t('groupBy.due')], ['label', t('groupBy.label')],
  ];
  const groupKey = (id: number) => `taskcraft.groupBy.${id}`;
  function storedGroup(id: number): GroupBy {
    try {
      const v = localStorage.getItem(groupKey(id));
      return GROUPS.some(([g]) => g === v) ? (v as GroupBy) : 'bucket';
    } catch {
      return 'bucket';
    }
  }
  let groupBy = $state<GroupBy>('bucket');
  $effect.pre(() => {
    if (plan) groupBy = storedGroup(plan.id);
  });
  function setGroup(g: GroupBy) {
    groupBy = g;
    if (!plan) return;
    try {
      localStorage.setItem(groupKey(plan.id), g);
    } catch {
      // Storage unavailable: the choice just isn't remembered.
    }
  }
  const groupMenu = new PopupMenu<null>();
  let view = $state<TaskView>('all');
  let status = $state<StatusFilter>('any');
  let text = $state('');
  let parsed = $state<ParsedItem | null>(null);

  // Re-evaluated on every data refresh so "overdue" follows the clock.
  let now = $derived.by(() => (void tasks, new Date()));
  let labelled = $derived(applyFilters(tasks, { status: 'any', labels: active }));
  // Inside a plan, the "No date" group follows the board's order.
  let groups = $derived.by(() => {
    const gs = taskPageGroups(tasks, { view, status, labels: active }, now);
    if (!plan) return gs;
    const order = planOrder(buckets);
    return gs.map((g) => (g.kind === 'nodate' ? { ...g, tasks: [...g.tasks].sort(order) } : g));
  });
  // Date-tab counts follow the status filter, except Done (which has no date tabs).
  let counts = $derived(taskCounts(layout === 'list' && status !== 'done' ? applyFilters(tasks, { status, labels: active }) : labelled, now));
  // The board shows open tasks for the date filter plus every finished one.
  let boardTasks = $derived([...filterTasks(labelled, view, now), ...labelled.filter((t) => t.completedAt)]);
  // Four date tabs; finished tasks live under the Done status (or the board's Done column).
  const views: TaskView[] = ['all', 'today', 'upcoming', 'nodate'];

  // Board columns for Status (every scope), Due date and Label (inside a plan).
  let showAllDone = $state(false);
  const STATUS_EMPTY = { todo: t('board.emptyTodo'), inProgress: t('board.emptyProgress'), done: t('board.emptyDone') };
  const DUE_TITLE: Record<DueKey, string> = {
    overdue: t('group.overdue'), today: t('view.today'), thisWeek: t('due.thisWeek'), later: t('due.later'), nodate: t('view.nodate'),
  };
  let columns = $derived.by((): BoardColumn[] => {
    const mode = plan ? groupBy : 'status';
    if (mode === 'due') {
      const order = planOrder(buckets);
      return dueColumns(boardTasks, now, weekStart).map((c) => ({
        key: c.key, title: DUE_TITLE[c.key], danger: c.key === 'overdue', empty: t('board.emptyGroup'),
        // Dated columns run by due date; undated ones follow the board's order.
        tasks: c.key === 'nodate' ? c.tasks.sort(order) : c.tasks,
      }));
    }
    if (mode === 'label') {
      const order = planOrder(buckets);
      // Only labels in use here (plus "No label"), open tasks like the Due board.
      return labelColumns(boardTasks.filter((x) => !x.completedAt), labels)
        .filter((c) => !c.label || c.tasks.length)
        .map((c) => ({
          key: c.key, title: c.label?.name ?? t('label.none'), color: c.label?.color, empty: t('board.emptyGroup'),
          tasks: [...c.tasks].sort(order),
        }));
    }
    return boardColumns(boardTasks, { showAllDone }).map((c) => ({
      key: c.status, title: STATUS_LABEL[c.status], status: c.status, tasks: c.tasks, empty: STATUS_EMPTY[c.status],
      count: c.tasks.length + c.hidden,
      more: c.status === 'done' && (c.hidden || showAllDone)
        ? { label: showAllDone ? t('board.showRecent') : t('board.showOlder', { n: c.hidden }), onclick: () => (showAllDone = !showAllDone) }
        : undefined,
    }));
  });
  let statusBoard = $derived(!plan || groupBy === 'status');

  /** Status board: a card dropped on another column, or moved with the arrow keys. */
  function dropOnStatus(task: Task, key: string) {
    const s = key as TaskStatus;
    if (task.status !== s) setStatus(task, s);
  }
  function stepStatus(task: Task, by: -1 | 1): boolean {
    const next = STATUSES[STATUSES.indexOf(task.status) + by];
    if (!next) return false;
    setStatus(task, next);
    return true;
  }

  $effect(() => {
    try {
      localStorage.setItem(LAYOUT_KEY, layout);
    } catch {
      // Storage unavailable: the choice just isn't remembered.
    }
    if (layout === 'board' && status === 'done') status = 'any';
  });

  // Drop filters for labels that were deleted.
  $effect(() => {
    const ids = new Set(labels.map((l) => l.id));
    if (active.some((id) => !ids.has(id))) active = active.filter((id) => ids.has(id));
  });

  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const t = text;
    clearTimeout(timer);
    if (!t.trim()) return void (parsed = null);
    timer = setTimeout(async () => {
      try {
        parsed = await api.parsePreview(t, true);
      } catch {
        parsed = null;
      }
    }, 80);
  });

  async function add(e: KeyboardEvent) {
    if (e.key === 'Escape') return void (text = '');
    if (e.key !== 'Enter' || !text.trim()) return;
    try {
      const typed = text;
      const scopePlan = plan;
      const first = buckets[0];
      // Parse now rather than trusting the debounced preview, which may lag behind a fast Enter.
      const hint = scopePlan ? (await api.parsePreview(typed, true)).plan : null;
      const ref = await api.createFromText(typed, true);
      text = '';
      parsed = null;
      if (!scopePlan || !first || ref.kind !== 'task') return;
      // Inside a plan, a task without its own plan hint goes to the end of this plan's first bucket.
      const [created, defaultPlanId] = await Promise.all([api.getTask(ref.id), api.getDefaultPlan()]);
      if (quickAddNeedsMove({ hint, taskPlanId: created.planId, scopePlanId: scopePlan.id, defaultPlanId })) {
        const last = tasks.filter((x) => x.bucketId === first.id).sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id).at(-1);
        await api.moveTask(ref.id, first.id, last?.id ?? null, null);
      }
    } catch (err) {
      onerror(errorMessage(err));
    }
  }

  async function setStatus(t: Task, s: TaskStatus) {
    try {
      await changeStatus(t, s);
    } catch (err) {
      onerror(errorMessage(err));
    }
  }

  const planOf = (task: Task) => plans.find((p) => p.id === task.planId);
  const toggleLabel = (id: number) => (active = active.includes(id) ? active.filter((x) => x !== id) : [...active, id]);
  const labelColor = (name: string) => labels.find((l) => l.name.toLowerCase() === name.toLowerCase())?.color ?? 'var(--accent)';

  const heading = (g: TaskGroup<Task>) =>
    g.kind === 'overdue' ? t('group.overdue') : g.kind === 'nodate' ? t('view.nodate') : g.kind === 'completed' ? t('view.completed') : fmtDayHeading(g.day!, now);

  const dueLabel = (t: Task, g: TaskGroup<Task>) => {
    if (!t.due) return '';
    const d = new Date(t.due);
    // Inside a day group the date is in the heading; show only the time.
    if (g.kind === 'day') return t.dueHasTime ? fmtTime(d) : '';
    return `${fmtShortDate(d)}${t.dueHasTime ? ` ${fmtTime(d)}` : ''}`;
  };

  const emptyText = () =>
    active.length || (status !== 'any' && status !== 'done')
      ? t('tasks.emptyFiltered')
      : status === 'done'
        ? t('tasks.emptyCompleted')
        : t('tasks.empty');
</script>


<div class="page" class:board={layout === 'board'}>
  <div class="head">
    <div class="add">
      <HighlightInput label={t('common.newTask')} bind:value={text} bind:input spans={parsed?.spans ?? []} placeholder={t('tasks.addPlaceholder')} onkeydown={add} />
      <div class="hint">
        {#if parsed}
          <strong>{parsed.title || t('common.untitled')}</strong>
          <span>{describeParsed(parsed)}</span>
          {#each parsed.tags as tag}<LabelChip name={tag} color={labelColor(tag)} />{/each}
        {:else}
          {t('tasks.addHint')}
        {/if}
      </div>
    </div>
    <div class="layout" role="radiogroup" aria-label={t('tasks.layout')}>
      <button role="radio" aria-checked={layout === 'list'} class:on={layout === 'list'} onclick={() => (layout = 'list')} title={t('tasks.list')}><Rows3 size={15} /><span>{t('tasks.list')}</span></button>
      <button role="radio" aria-checked={layout === 'board'} class:on={layout === 'board'} onclick={() => (layout = 'board')} title={t('tasks.board')}><Columns3 size={15} /><span>{t('tasks.board')}</span></button>
    </div>
  </div>

  <div class="filters">
    {#if !(layout === 'list' && status === 'done')}
      <div class="views" role="group" aria-label={t('tasks.due')}>
        {#each views as v}
          <button aria-pressed={view === v} class:on={view === v} onclick={() => (view = v)}>
            {VIEW_NAMES[v]}<span class="n">{counts[v]}</span>
          </button>
        {/each}
      </div>
    {/if}
    {#if layout === 'board' && plan}
      <button class="group-by" aria-haspopup="menu" aria-expanded={!!groupMenu.current}
        onclick={(e) => groupMenu.openBelow(null, e.currentTarget, () => true)}>
        {t('groupBy.current', { name: GROUPS.find(([g]) => g === groupBy)?.[1] ?? '' })}<ChevronDown size={13} aria-hidden="true" />
      </button>
    {/if}
    {#if layout === 'list'}
      <div class="status" role="group" aria-label={t('inspector.status')}>
        {#each STATUS_FILTERS as [v, name]}
          <button aria-pressed={status === v} class:on={status === v} onclick={() => (status = v)}>{name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="content">
    {#if layout === 'board'}
      {#if plan && groupBy === 'bucket'}
        <BucketBoard {plan} {buckets} tasks={boardTasks} {now} activeLabels={active} onstatus={setStatus} {onopen} onlabel={toggleLabel} {onerror} {onreload} />
      {:else}
        <TaskBoard {columns} {now} activeLabels={active} plans={plan ? [] : plans} onstatus={setStatus} {onopen} onlabel={toggleLabel}
          ondrop={statusBoard ? dropOnStatus : undefined} onkeymove={statusBoard ? stepStatus : undefined} />
      {/if}
    {:else}
      {#each groups as g (g.key)}
        <section>
          <h2 class:danger={g.kind === 'overdue'}>{heading(g)}<span class="n">{g.tasks.length}</span></h2>
          {#each g.tasks as t (t.id)}
            <TaskRow task={t} due={dueLabel(t, g)} late={isOverdue(t, now)} activeLabels={active} plan={plan ? undefined : planOf(t)}
              onstatus={(s) => setStatus(t, s)} onopen={() => onopen(t)} onlabel={toggleLabel} />
          {/each}
        </section>
      {:else}
        <div class="empty"><Doodle /><p>{emptyText()}</p></div>
      {/each}
    {/if}
  </div>
</div>

{#if groupMenu.current}
  <Menu menu={groupMenu} label={t('groupBy.title')}>
    {#each GROUPS as [g, name] (g)}
      <button role="menuitemradio" aria-checked={groupBy === g} onclick={() => groupMenu.act(() => setGroup(g))}>
        <span class="check" aria-hidden="true">{groupBy === g ? '✓' : ''}</span>{name}
      </button>
    {/each}
  </Menu>
{/if}

<style>
  .page { height: 100%; display: flex; flex-direction: column; width: 100%; max-width: 780px; margin: 0 auto; padding: 16px 24px 0; }
  .page.board { max-width: none; }
  .head { display: flex; gap: 12px; align-items: flex-start; }
  .add { flex: 1; min-width: 0; }
  .hint { display: flex; align-items: baseline; gap: 8px; font-size: 12px; color: var(--muted); padding: 6px 4px 0; min-height: 22px; white-space: nowrap; overflow: hidden; }
  .hint strong { color: var(--text); font-weight: 600; overflow: hidden; text-overflow: ellipsis; }
  .layout { display: flex; background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 9px; padding: 2px; flex: none; margin-top: 3px; }
  .layout button { display: flex; align-items: center; gap: 5px; border: none; background: none; padding: 5px 10px; border-radius: 7px; color: var(--muted); }
  .layout button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .layout button:focus-visible { outline: 2px solid var(--accent); }

  .filters { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; padding: 10px 0 12px; border-bottom: 1px solid var(--border); }
  .views { display: flex; gap: 2px; }
  .views button { border: none; background: none; padding: 4px 9px; border-radius: 7px; color: var(--muted); display: flex; gap: 6px; align-items: baseline; }
  .views button:hover { color: var(--text); }
  .views button.on { color: var(--text); background: var(--hover); font-weight: 600; }
  .views button:focus-visible { outline: 2px solid var(--accent); }
  .n { font-variant-numeric: tabular-nums; font-size: 12px; color: var(--faint); font-weight: 400; }
  /* Status: a small segmented control at the end of the row. */
  .status { display: flex; margin-left: auto; background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 8px; padding: 2px; }
  .status button { border: none; background: none; padding: 3px 9px; border-radius: 6px; color: var(--muted); font-size: 12px; }
  .group-by { margin-left: auto; display: flex; align-items: center; gap: 4px; border: 1px solid var(--border); background: var(--bg-sunken); border-radius: 8px; padding: 3px 8px 3px 10px; color: var(--text); font-size: 12px; }
  .group-by:hover { background: var(--hover); }
  .group-by:focus-visible { outline: 2px solid var(--accent); }
  .check { display: inline-block; width: 16px; color: var(--accent); }
  .status button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }

  .content { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 0 24px; }
  .board .content { padding-top: 14px; overflow: hidden; }
  h2 { font-size: 13px; font-weight: 600; margin: 20px 0 4px; display: flex; gap: 8px; align-items: baseline; }
  h2.danger { color: var(--danger); }
  .empty { color: var(--muted); text-align: center; margin-top: 48px; }
  .empty p { margin: 0; }

  @media (max-width: 640px) {
    .page { padding: 12px 16px 0; }
    .layout span { display: none; }
    .status { margin-left: 0; }
  }
</style>
