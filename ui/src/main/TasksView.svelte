<script lang="ts">
  import { t } from '../lib/i18n';
  import { Rows3, Columns3 } from 'lucide-svelte';
  import type { ParsedItem, Task, TaskLabel, TaskStatus } from '../lib/types';
  import { api, errorMessage } from '../lib/api';
  import { describeParsed, fmtDayHeading, fmtShortDate, fmtTime } from '../lib/format';
  import {
    applyFilters, filterTasks, isOverdue, taskCounts, taskPageGroups,
    type StatusFilter, type TaskGroup, type TaskView,
  } from '../lib/tasks';
  import HighlightInput from '../quick/HighlightInput.svelte';
  import { changeStatus } from '../lib/actions';
  import TaskRow from './tasks/TaskRow.svelte';
  import TaskBoard from './tasks/TaskBoard.svelte';
  import Doodle from './tasks/Doodle.svelte';
  import LabelChip from './tasks/LabelChip.svelte';

  let {
    tasks,
    labels,
    onopen,
    onerror,
    input = $bindable<HTMLInputElement | undefined>(),
    active = $bindable<number[]>([]),
  }: {
    tasks: Task[];
    labels: TaskLabel[];
    onopen: (t: Task) => void;
    onerror: (msg: string) => void;
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
  let view = $state<TaskView>('all');
  let status = $state<StatusFilter>('any');
  let text = $state('');
  let parsed = $state<ParsedItem | null>(null);

  // Re-evaluated on every data refresh so "overdue" follows the clock.
  let now = $derived.by(() => (void tasks, new Date()));
  let labelled = $derived(applyFilters(tasks, { status: 'any', labels: active }));
  let groups = $derived(taskPageGroups(tasks, { view, status, labels: active }, now));
  // Date-tab counts follow the status filter, except Done (which has no date tabs).
  let counts = $derived(taskCounts(layout === 'list' && status !== 'done' ? applyFilters(tasks, { status, labels: active }) : labelled, now));
  // The board shows open tasks for the date filter plus every finished one.
  let boardTasks = $derived([...filterTasks(labelled, view, now), ...labelled.filter((t) => t.completedAt)]);
  // Four date tabs; finished tasks live under the Done status (or the board's Done column).
  const views: TaskView[] = ['all', 'today', 'upcoming', 'nodate'];

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
      await api.createFromText(text, true);
      text = '';
      parsed = null;
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
      <TaskBoard tasks={boardTasks} {now} activeLabels={active} onstatus={setStatus} {onopen} onlabel={toggleLabel} />
    {:else}
      {#each groups as g (g.key)}
        <section>
          <h2 class:danger={g.kind === 'overdue'}>{heading(g)}<span class="n">{g.tasks.length}</span></h2>
          {#each g.tasks as t (t.id)}
            <TaskRow task={t} due={dueLabel(t, g)} late={isOverdue(t, now)} activeLabels={active}
              onstatus={(s) => setStatus(t, s)} onopen={() => onopen(t)} onlabel={toggleLabel} />
          {/each}
        </section>
      {:else}
        <div class="empty"><Doodle /><p>{emptyText()}</p></div>
      {/each}
    {/if}
  </div>
</div>

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
