<script lang="ts">
  import { Rows3, Columns3, Tags } from 'lucide-svelte';
  import type { ParsedItem, Task, TaskLabel, TaskStatus } from '../lib/types';
  import { api, errorMessage } from '../lib/api';
  import { describeParsed, fmtDayHeading, fmtShortDate, fmtTime } from '../lib/format';
  import {
    applyFilters, filterTasks, groupTasks, isOverdue, taskCounts, TASK_VIEWS,
    type StatusFilter, type TaskGroup, type TaskView,
  } from '../lib/tasks';
  import HighlightInput from '../quick/HighlightInput.svelte';
  import TaskRow from './tasks/TaskRow.svelte';
  import TaskBoard from './tasks/TaskBoard.svelte';
  import LabelChip from './tasks/LabelChip.svelte';
  import LabelManager from './tasks/LabelManager.svelte';

  let {
    tasks,
    labels,
    onopen,
    onerror,
    input = $bindable<HTMLInputElement | undefined>(),
  }: {
    tasks: Task[];
    labels: TaskLabel[];
    onopen: (t: Task) => void;
    onerror: (msg: string) => void;
    input?: HTMLInputElement;
  } = $props();

  const VIEW_NAMES: Record<TaskView, string> = { all: 'All', today: 'Today', upcoming: 'Upcoming', nodate: 'No date', completed: 'Completed' };
  const STATUS_FILTERS: [StatusFilter, string][] = [['any', 'Any status'], ['todo', 'To do'], ['inProgress', 'In progress']];
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
  let active = $state<number[]>([]);
  let managing = $state(false);
  let text = $state('');
  let parsed = $state<ParsedItem | null>(null);

  // Re-evaluated on every data refresh so "overdue" follows the clock.
  let now = $derived.by(() => (void tasks, new Date()));
  let labelled = $derived(applyFilters(tasks, { status: 'any', labels: active }));
  let filtered = $derived(applyFilters(tasks, { status, labels: active }));
  let groups = $derived(groupTasks(filtered, view, now));
  let counts = $derived(taskCounts(layout === 'list' ? filtered : labelled, now));
  // The board shows open tasks for the date filter plus every finished one.
  let boardTasks = $derived([...filterTasks(labelled, view, now), ...labelled.filter((t) => t.completedAt)]);
  let usage = $derived(new Map(labels.map((l) => [l.id, tasks.filter((t) => !t.completedAt && t.labels.some((x) => x.id === l.id)).length])));
  let views = $derived(layout === 'board' ? TASK_VIEWS.filter((v) => v !== 'completed') : TASK_VIEWS);

  $effect(() => {
    try {
      localStorage.setItem(LAYOUT_KEY, layout);
    } catch {
      // Storage unavailable: the choice just isn't remembered.
    }
    if (layout === 'board' && view === 'completed') view = 'all';
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
      await api.setTaskStatus(t.id, s);
    } catch (err) {
      onerror(errorMessage(err));
    }
  }

  const toggleLabel = (id: number) => (active = active.includes(id) ? active.filter((x) => x !== id) : [...active, id]);
  const labelColor = (name: string) => labels.find((l) => l.name.toLowerCase() === name.toLowerCase())?.color ?? 'var(--accent)';

  const heading = (g: TaskGroup<Task>) =>
    g.kind === 'overdue' ? 'Overdue' : g.kind === 'nodate' ? 'No date' : g.kind === 'completed' ? 'Completed' : fmtDayHeading(g.day!, now);

  const dueLabel = (t: Task, g: TaskGroup<Task>) => {
    if (!t.due) return '';
    const d = new Date(t.due);
    // Inside a day group the date is in the heading; show only the time.
    if (g.kind === 'day') return t.dueHasTime ? fmtTime(d) : '';
    return `${fmtShortDate(d)}${t.dueHasTime ? ` ${fmtTime(d)}` : ''}`;
  };

  const emptyText = () =>
    active.length || status !== 'any'
      ? 'No tasks match these filters.'
      : view === 'completed'
        ? 'Tasks you finish show up here.'
        : 'Nothing here. Type a task above and press Return.';
</script>

<svelte:window onpointerdown={(e) => managing && !(e.target as HTMLElement).closest?.('.mgr') && (managing = false)} />

<div class="page" class:board={layout === 'board'}>
  <div class="head">
    <div class="add">
      <HighlightInput label="New task" bind:value={text} bind:input spans={parsed?.spans ?? []} placeholder="Add a task, like “pay rent friday 9am #home”" onkeydown={add} />
      <div class="hint">
        {#if parsed}
          <strong>{parsed.title || 'Untitled'}</strong>
          <span>{describeParsed(parsed)}</span>
          {#each parsed.tags as tag}<LabelChip name={tag} color={labelColor(tag)} />{/each}
        {:else}
          Press Return to add. Dates and #labels are optional.
        {/if}
      </div>
    </div>
    <div class="layout" role="radiogroup" aria-label="Layout">
      <button role="radio" aria-checked={layout === 'list'} class:on={layout === 'list'} onclick={() => (layout = 'list')} title="List"><Rows3 size={15} /><span>List</span></button>
      <button role="radio" aria-checked={layout === 'board'} class:on={layout === 'board'} onclick={() => (layout = 'board')} title="Board"><Columns3 size={15} /><span>Board</span></button>
    </div>
  </div>

  <div class="filters">
    <div class="views" role="tablist" aria-label="Due">
      {#each views as v}
        <button role="tab" aria-selected={view === v} class:on={view === v} onclick={() => (view = v)}>
          {VIEW_NAMES[v]}<span class="n">{counts[v]}</span>
        </button>
      {/each}
    </div>
    {#if layout === 'list'}
      <select class="status" bind:value={status} aria-label="Status">
        {#each STATUS_FILTERS as [v, name]}<option value={v}>{name}</option>{/each}
      </select>
    {/if}
    <div class="labels">
      {#each labels as l (l.id)}
        <LabelChip name={l.name} color={l.color} active={active.includes(l.id)} count={usage.get(l.id)} onclick={() => toggleLabel(l.id)} />
      {/each}
      {#if active.length}<button class="clear" onclick={() => (active = [])}>Clear</button>{/if}
      <span class="mgr">
        <button class="icon-btn" title="Manage labels" aria-label="Manage labels" aria-expanded={managing} onclick={() => (managing = !managing)}><Tags size={15} /></button>
        {#if managing}<LabelManager {labels} onclose={() => (managing = false)} {onerror} />{/if}
      </span>
    </div>
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
        <p class="empty">{emptyText()}</p>
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
  .n { font-variant-numeric: tabular-nums; font-size: 11px; color: var(--faint); font-weight: 400; }
  .status { border: 1px solid var(--border); background: var(--surface-strong); border-radius: 7px; padding: 3px 6px; color: var(--muted); }
  .labels { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-left: auto; }
  .clear { border: none; background: none; color: var(--accent); font-size: 12px; padding: 2px 4px; }
  .mgr { position: relative; }

  .content { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 0 24px; }
  .board .content { padding-top: 14px; overflow: hidden; }
  h2 { font-size: 13px; font-weight: 600; margin: 20px 0 4px; display: flex; gap: 8px; align-items: baseline; }
  h2.danger { color: var(--danger); }
  .empty { color: var(--muted); text-align: center; margin-top: 48px; }

  @media (max-width: 640px) {
    .page { padding: 12px 16px 0; }
    .layout span { display: none; }
    .labels { margin-left: 0; }
  }
</style>
