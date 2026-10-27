<script lang="ts" module>
  import { SvelteSet } from 'svelte/reactivity';
  /** Collapsed groups (`b:<id>` / `p:<id>`) and parents (task ids), remembered for the session. */
  const collapsed = new SvelteSet<string | number>();
  /** The sticky name column's width. */
  export const NAME_W = 240;
  const HEADER_H = 40;
</script>

<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage, onItemsChanged } from '../../lib/api';
  import type { Bucket, Dependency, Plan, Span, Task } from '../../lib/types';
  import { DAY_WIDTH, ROW_H, dayNumber, saveZoom, timelineRange, timelineRows, type Zoom } from '../../lib/timeline';
  import { quickAddTask } from '../../lib/actions';
  import type { StatusFilter } from '../../lib/tasks';
  import HighlightInput from '../../quick/HighlightInput.svelte';
  import Doodle from '../tasks/Doodle.svelte';
  import PlannerHeader from './PlannerHeader.svelte';
  import PlannerRows from './PlannerRows.svelte';

  let {
    tasks,
    plans,
    buckets,
    weekStart,
    scope,
    onopen,
    onerror,
    zoom = $bindable<Zoom>('weeks'),
    input = $bindable<HTMLInputElement | undefined>(),
    active = $bindable<number[]>([]),
  }: {
    /** Every task; the planner picks the scope's. */
    tasks: Task[];
    plans: Plan[];
    /** The scoped plan's buckets (empty in All tasks). */
    buckets: Bucket[];
    weekStart: 0 | 1;
    scope: number | 'all';
    onopen: (t: Task) => void;
    onerror: (msg: string) => void;
    /** Days / Weeks / Months, switched from the toolbar; saved here. */
    zoom?: Zoom;
    /** The "New task" input, focused by Mod+N. */
    input?: HTMLInputElement;
    /** Label filter (ids), shared with the sidebar, Tasks and Grid. */
    active?: number[];
  } = $props();

  $effect(() => saveZoom(zoom));

  const STATUS_FILTERS: [StatusFilter, string][] = [
    ['any', t('status.any')], ['todo', t('status.todo')], ['inProgress', t('status.inProgress')], ['done', t('status.done')],
  ];
  let plan = $derived(scope === 'all' ? null : plans.find((p) => p.id === scope) ?? null);
  let status = $state<StatusFilter>('any');

  // In All tasks every plan's buckets are needed (for order); inside a plan Main has them.
  let allBuckets = $state<Bucket[]>([]);
  let bucketReq = 0;
  $effect(() => {
    if (scope !== 'all') return;
    void tasks; // a refresh may have renamed or added buckets
    const ids = plans.map((p) => p.id);
    const req = ++bucketReq;
    Promise.all(ids.map((id) => api.listBuckets(id)))
      .then((lists) => req === bucketReq && (allBuckets = lists.flat()))
      .catch((e) => req === bucketReq && onerror(errorMessage(e)));
  });
  let shownBuckets = $derived(scope === 'all' ? allBuckets : buckets);

  // ── Dependencies: every row, refreshed when anything changes (arrows build on these) ──
  let deps = $state<Dependency[]>([]);
  let depReq = 0;
  function loadDeps() {
    const req = ++depReq;
    api.listDependencies().then((d) => req === depReq && (deps = d), (e) => req === depReq && onerror(errorMessage(e)));
  }
  onMount(() => {
    loadDeps();
    const un = onItemsChanged(loadDeps);
    return () => void un.then((f) => f());
  });
  /** Titles of each task's open blockers. */
  let blockers = $derived.by(() => {
    const byId = new Map(tasks.map((x) => [x.id, x]));
    const m = new Map<number, string[]>();
    for (const d of deps) {
      const b = byId.get(d.dependsOn);
      if (!b || b.completedAt) continue;
      m.set(d.taskId, [...(m.get(d.taskId) ?? []), b.title]);
    }
    return m;
  });

  // ── Rows and range ──
  let now = $derived.by(() => (void tasks, new Date()));
  let today = $derived(dayNumber(now));
  let inScope = $derived(scope === 'all' ? tasks : tasks.filter((x) => x.planId === scope));
  let range = $derived(timelineRange(inScope, now));
  let rows = $derived(timelineRows(tasks, { scope, buckets: shownBuckets, plans, labels: active, status, collapsed }));
  let filtered = $derived(status !== 'any' || active.length > 0);
  let laneW = $derived((range.to - range.from + 1) * DAY_WIDTH[zoom]);

  // ── Horizontal position: the day at the timeline's left third stays put across zoom and range changes ──
  let scroller = $state<HTMLDivElement | undefined>();
  let anchorDay = dayNumber(new Date()) + 0.5; // the middle of today's column
  const visibleW = () => Math.max(0, (scroller?.clientWidth ?? 0) - NAME_W);
  function applyAnchor() {
    if (!scroller) return;
    scroller.scrollLeft = (anchorDay - range.from) * DAY_WIDTH[zoom] - visibleW() / 3;
  }
  $effect(() => {
    void range.from; void zoom; void scroller;
    untrack(applyAnchor);
  });
  function onscroll() {
    if (scroller) anchorDay = range.from + (scroller.scrollLeft + visibleW() / 3) / DAY_WIDTH[zoom];
  }
  /** Brings today back to the left third ("Today" and Mod+T). */
  export function scrollToToday() {
    anchorDay = dayNumber(new Date()) + 0.5;
    applyAnchor();
  }

  function reveal(i: number) {
    if (!scroller) return;
    const top = i * ROW_H; // rows start right under the sticky header
    const view = scroller.clientHeight - HEADER_H;
    if (top < scroller.scrollTop) scroller.scrollTop = top;
    else if (top + ROW_H > scroller.scrollTop + view) scroller.scrollTop = top + ROW_H - view;
  }

  // ── New task ──
  let text = $state('');
  let spans = $state<Span[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(timer));
  $effect(() => {
    const v = text;
    clearTimeout(timer);
    if (!v.trim()) return void (spans = []);
    timer = setTimeout(() => api.parsePreview(v, true).then((p) => (spans = p.spans), () => (spans = [])), 80);
  });
  let adding = false;
  async function add(e: KeyboardEvent) {
    if (e.key === 'Escape') return void (text = '');
    if (e.key !== 'Enter' || !text.trim()) return;
    if (adding) return;
    adding = true;
    try {
      await quickAddTask(text, plan, buckets[0], tasks, () => (text = ''));
    } catch (err) {
      onerror(errorMessage(err));
    } finally {
      adding = false;
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
  </div>

  {#if rows.length}
    <div class="scroller" bind:this={scroller} {onscroll}>
      <div class="canvas" style="width:{NAME_W + laneW}px">
        <PlannerHeader {range} {zoom} {weekStart} {today} nameW={NAME_W} height={HEADER_H} />
        <PlannerRows {rows} {range} {zoom} {now} nameW={NAME_W} {blockers} {onopen} onreveal={reveal}
          ontoggle={(k) => (collapsed.has(k) ? collapsed.delete(k) : collapsed.add(k))} />
      </div>
    </div>
  {:else}
    <div class="empty"><Doodle /><p>{filtered ? t('table.emptyFiltered') : t('planner.empty')}</p></div>
  {/if}
</div>

<style>
  .page { height: 100%; display: flex; flex-direction: column; padding: 16px 0 0; min-width: 0; }
  .head { display: flex; gap: 12px; align-items: flex-start; padding: 0 24px 12px; }
  .add { flex: 1; min-width: 0; max-width: 560px; }
  .status { display: flex; margin-left: auto; margin-top: 3px; background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 8px; padding: 2px; flex: none; }
  .status button { border: none; background: none; padding: 3px 9px; border-radius: 6px; color: var(--muted); font-size: 12px; }
  .status button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .status button:focus-visible { outline: 2px solid var(--accent); }
  .scroller { flex: 1; min-height: 0; overflow: auto; border-top: 1px solid var(--border); overscroll-behavior: contain; }
  .canvas { position: relative; min-height: 100%; }
  .empty { color: var(--muted); text-align: center; border-top: 1px solid var(--border); padding-top: 48px; }
  .empty p { margin: 0; }
  @media (max-width: 720px) {
    .page { padding-top: 12px; }
    .head { flex-wrap: wrap; padding: 0 16px 12px; }
    .add { flex-basis: 100%; max-width: none; }
    .status { margin-left: 0; }
  }
</style>
