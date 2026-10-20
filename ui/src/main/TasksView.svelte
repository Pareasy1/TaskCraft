<script lang="ts">
  import { Repeat } from 'lucide-svelte';
  import type { ParsedItem, Task } from '../lib/types';
  import { api, errorMessage } from '../lib/api';
  import { describeParsed, fmtDayHeading, fmtShortDate, fmtTime } from '../lib/format';
  import { groupTasks, isOverdue, taskCounts, TASK_VIEWS, type TaskGroup, type TaskView } from '../lib/tasks';
  import HighlightInput from '../quick/HighlightInput.svelte';

  let {
    tasks,
    oncomplete,
    onopen,
    onerror,
    input = $bindable<HTMLInputElement | undefined>(),
  }: {
    tasks: Task[];
    oncomplete: (t: Task) => void;
    onopen: (t: Task) => void;
    onerror: (msg: string) => void;
    input?: HTMLInputElement;
  } = $props();

  const LABELS: Record<TaskView, string> = { all: 'All', today: 'Today', upcoming: 'Upcoming', nodate: 'No date', completed: 'Completed' };

  let view = $state<TaskView>('all');
  let text = $state('');
  let parsed = $state<ParsedItem | null>(null);
  // Re-evaluated on every data refresh so "overdue" follows the clock.
  let now = $derived.by(() => (void tasks, new Date()));
  let groups = $derived(groupTasks(tasks, view, now));
  let counts = $derived(taskCounts(tasks, now));

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

  const heading = (g: TaskGroup<Task>) =>
    g.kind === 'overdue' ? 'Overdue' : g.kind === 'nodate' ? 'No date' : g.kind === 'completed' ? 'Completed' : fmtDayHeading(g.day!, now);

  const dueLabel = (t: Task, g: TaskGroup<Task>) => {
    if (!t.due) return '';
    const d = new Date(t.due);
    // Inside a day group the date is in the heading; show only the time.
    if (g.kind === 'day') return t.dueHasTime ? fmtTime(d) : '';
    return `${fmtShortDate(d)}${t.dueHasTime ? ` ${fmtTime(d)}` : ''}`;
  };
</script>

<div class="tasks">
  <div class="add">
    <HighlightInput bind:value={text} bind:input spans={parsed?.spans ?? []} placeholder="Add a task — e.g. “pay rent friday 9am”" onkeydown={add} />
    <div class="hint">{parsed ? `${parsed.title || '…'} · ${describeParsed(parsed)}` : 'Press Return to add · dates are optional'}</div>
  </div>

  <div class="filters" role="tablist">
    {#each TASK_VIEWS as v}
      <button role="tab" class:on={view === v} onclick={() => (view = v)}>
        {LABELS[v]}<span class="n">{counts[v]}</span>
      </button>
    {/each}
  </div>

  <div class="list">
    {#each groups as g (g.key)}
      <section>
        <h2 class:danger={g.kind === 'overdue'}>{heading(g)}<span class="n">{g.tasks.length}</span></h2>
        {#each g.tasks as t (t.id)}
          <div class="row" class:done={!!t.completedAt} role="button" tabindex="0" onclick={() => onopen(t)} onkeydown={(e) => e.key === 'Enter' && onopen(t)}>
            <input type="checkbox" checked={!!t.completedAt} style="accent-color:{t.color}" aria-label="Complete {t.title}"
              onclick={(e) => e.stopPropagation()} onchange={() => oncomplete(t)} />
            <span class="title">{t.title}</span>
            {#if t.recurring}<Repeat size={12} class="rep" />{/if}
            {#if dueLabel(t, g)}<span class="due" class:late={isOverdue(t, now)}>{dueLabel(t, g)}</span>{/if}
            <span class="dot" style="background:{t.color}"></span>
          </div>
        {/each}
      </section>
    {:else}
      <p class="empty">{view === 'completed' ? 'No completed tasks yet.' : 'Nothing here. Add a task above.'}</p>
    {/each}
  </div>
</div>

<style>
  .tasks { height: 100%; display: flex; flex-direction: column; max-width: 760px; margin: 0 auto; width: 100%; padding: 14px 20px 0; box-sizing: border-box; }
  .hint { font-size: 12px; color: var(--muted); padding: 5px 4px 0; min-height: 16px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .filters { display: flex; gap: 4px; margin: 12px 0 4px; flex-wrap: wrap; }
  .filters button { border: 1px solid var(--border); background: none; border-radius: 999px; padding: 3px 11px; color: var(--muted); display: flex; gap: 6px; align-items: center; }
  .filters button.on { background: var(--accent); border-color: var(--accent); color: #fff; }
  .n { font-variant-numeric: tabular-nums; font-size: 11px; opacity: 0.75; }
  .list { flex: 1; min-height: 0; overflow-y: auto; padding-bottom: 20px; }
  h2 { font-size: 12px; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: 0.03em; margin: 16px 6px 4px; display: flex; gap: 8px; }
  h2.danger { color: var(--danger); }
  .row { display: flex; align-items: center; gap: 9px; padding: 7px 8px; border-radius: 7px; border-bottom: 1px solid var(--border); }
  .row:hover { background: var(--hover); }
  .row input { margin: 0; width: 15px; height: 15px; }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .done .title { color: var(--faint); text-decoration: line-through; }
  .row :global(.rep) { color: var(--faint); flex: none; }
  .due { font-size: 12px; color: var(--muted); white-space: nowrap; }
  .due.late { color: var(--danger); }
  .dot { width: 7px; height: 7px; border-radius: 50%; flex: none; }
  .empty { color: var(--muted); text-align: center; margin-top: 40px; }
</style>
