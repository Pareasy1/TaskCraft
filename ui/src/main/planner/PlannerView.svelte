<script lang="ts" module>
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  /** Collapsed groups (`b:<id>` / `p:<id>`) and parents (task ids), remembered for the session. */
  const collapsed = new SvelteSet<string | number>();
  /** The sticky name column's width. */
  export const NAME_W = 240;
  const HEADER_H = 40;
</script>

<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage, onItemsChanged } from '../../lib/api';
  import type { Bucket, Dependency, Plan, Span, Task } from '../../lib/types';
  import {
    DAY_WIDTH, ROW_H, dayNumber, dayToX, loadUndatedOpen, placeUndated, saveUndatedOpen, saveZoom, timelineRange, timelineRows, xToDay, type Zoom,
  } from '../../lib/timeline';
  import { quickAddTask, saveDates, type TaskDates } from '../../lib/actions';
  import { applyFilters, type StatusFilter } from '../../lib/tasks';
  import { pickerExclude } from '../../lib/dependencies';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { notify } from '../../lib/toast.svelte';
  import Menu from '../Menu.svelte';
  import TaskPicker from '../tasks/TaskPicker.svelte';
  import ArrowLayer from './ArrowLayer.svelte';
  import HighlightInput from '../../quick/HighlightInput.svelte';
  import Doodle from '../tasks/Doodle.svelte';
  import PlannerHeader from './PlannerHeader.svelte';
  import PlannerRows from './PlannerRows.svelte';
  import UndatedPanel from './UndatedPanel.svelte';

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

  // ── Drawing and removing dependencies ──
  const byId = $derived(new Map(tasks.map((x) => [x.id, x])));
  /** The selected arrow (`taskId:dependsOn`): the focused one. */
  let selectedArrow = $state<string | null>(null);

  /** `targetId` now depends on `source` (a connector drop or "Depends on…"), with Undo. */
  async function link(source: Task, targetId: number) {
    const target = byId.get(targetId);
    if (!target || deps.some((d) => d.taskId === targetId && d.dependsOn === source.id)) return;
    try {
      await api.addDependency(targetId, source.id);
    } catch (e) {
      return onerror(errorMessage(e)); // self / loop refusals, translated
    }
    deps = [...deps, { taskId: targetId, dependsOn: source.id }];
    loadDeps();
    notify(t('dep.linked', { a: source.title, b: target.title }), () => api.removeDependency(targetId, source.id));
  }

  /** Removes "taskId depends on dependsOn", with Undo; focus goes to the dependent task's row. */
  async function unlink(d: Dependency, refocus = false) {
    const a = byId.get(d.dependsOn), b = byId.get(d.taskId);
    try {
      await api.removeDependency(d.taskId, d.dependsOn);
    } catch (e) {
      return onerror(errorMessage(e));
    }
    deps = deps.filter((x) => !(x.taskId === d.taskId && x.dependsOn === d.dependsOn));
    if (selectedArrow === `${d.taskId}:${d.dependsOn}`) selectedArrow = null;
    loadDeps();
    notify(t('dep.unlinked', { a: a?.title ?? '', b: b?.title ?? '' }), () => api.addDependency(d.taskId, d.dependsOn));
    if (refocus) {
      await tick();
      scroller?.querySelector<HTMLElement>(`[data-row="t:${d.taskId}"]`)?.focus({ preventScroll: true });
    }
  }

  // ── The bar menu (right-click, menu key, Shift+F10 on a row) ──
  const barMenu = new PopupMenu<Task>();
  function onmenu(task: Task, e: MouseEvent | KeyboardEvent, row: HTMLElement) {
    if (e instanceof MouseEvent) return void barMenu.open(task, e.clientX, e.clientY, row);
    // From the keyboard: under the row's (sticky) name, since the row itself starts off-screen when scrolled.
    e.preventDefault();
    const r = (row.querySelector('.name') ?? row).getBoundingClientRect();
    barMenu.open(task, r.left + 16, r.bottom, row);
  }
  /** Every dependency of the menu's task (open or done), with the task it depends on. */
  let menuBlockers = $derived.by(() => {
    const id = barMenu.current?.target.id;
    if (id == null) return [];
    return deps.filter((d) => d.taskId === id).map((d) => byId.get(d.dependsOn)).filter((x): x is Task => !!x);
  });
  const NO_DATES = { start: null, startHasTime: false, due: null, dueHasTime: false } satisfies TaskDates;

  // "Depends on…": the searchable task list, floating where the menu was.
  let picking = $state<{ task: Task; x: number; y: number; opener: HTMLElement | null } | null>(null);
  let pickerEl = $state<HTMLDivElement | undefined>();
  function startPicking(task: Task) {
    const at = barMenu.current!;
    const row = scroller?.querySelector<HTMLElement>(`[data-row="t:${task.id}"]`) ?? null;
    barMenu.close(false);
    picking = { task, x: Math.max(8, Math.min(at.x, innerWidth - 328)), y: Math.max(8, Math.min(at.y, innerHeight - 280)), opener: row };
  }
  function endPicking() {
    const opener = picking?.opener;
    picking = null;
    if (opener?.isConnected) opener.focus({ preventScroll: true });
  }
  function pick(other: Task) {
    const task = picking!.task;
    endPicking();
    link(other, task.id);
  }
  $effect(() => {
    if (!picking) return;
    const away = (e: PointerEvent) => !pickerEl?.contains(e.target as Node) && (picking = null);
    window.addEventListener('pointerdown', away, true);
    return () => window.removeEventListener('pointerdown', away, true);
  });

  // ── Dates being dragged or saved: shown at once, dropped when fresh tasks arrive or a save fails ──
  const preview = new SvelteMap<number, TaskDates>();
  $effect.pre(() => {
    void tasks;
    untrack(() => preview.clear());
  });
  let shown = $derived(preview.size ? tasks.map((x) => (preview.has(x.id) ? { ...x, ...preview.get(x.id)! } : x)) : tasks);

  function onpreview(id: number, next: TaskDates | null) {
    if (next) preview.set(id, next);
    else preview.delete(id);
  }
  /** Saves new dates for `task` (as it was before the change), keeping them on screen meanwhile. */
  function onsave(task: Task, next: TaskDates) {
    preview.set(task.id, next);
    saveDates(task, next).catch((e) => {
      preview.delete(task.id);
      onerror(errorMessage(e));
    });
  }

  // ── Rows and range ──
  let now = $derived.by(() => (void tasks, new Date()));
  let today = $derived(dayNumber(now));
  // The range follows the saved tasks, so a drag in progress doesn't shift the grid under the pointer.
  let inScope = $derived(scope === 'all' ? tasks : tasks.filter((x) => x.planId === scope));
  let range = $derived(timelineRange(inScope, now));
  let rows = $derived(timelineRows(shown, { scope, buckets: shownBuckets, plans, labels: active, status, collapsed }));
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

  // ── No dates panel ──
  let undatedOpen = $state(loadUndatedOpen());
  $effect(() => saveUndatedOpen(undatedOpen));
  let undated = $derived(
    applyFilters(scope === 'all' ? shown : shown.filter((x) => x.planId === scope), { status, labels: active }).filter((x) => !x.start && !x.due),
  );
  /** The day a panel drag points at, highlighted on the grid. */
  let dropDay = $state<number | null>(null);
  /** The timeline day under a viewport point (bar lanes only, not the names or the header). */
  function dayAt(cx: number, cy: number): number | null {
    if (!scroller) return null;
    const r = scroller.getBoundingClientRect();
    const x = cx - r.left - NAME_W, y = cy - r.top;
    if (x < 0 || cx > r.left + scroller.clientWidth || y < HEADER_H || cy > r.bottom) return null;
    const day = xToDay(scroller.scrollLeft + x, range, zoom);
    return day >= range.from && day <= range.to ? day : null;
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
    <div class="body">
      <div class="scroller" bind:this={scroller} {onscroll}>
        <div class="canvas" style="width:{NAME_W + laneW}px">
          <PlannerHeader {range} {zoom} {weekStart} {today} nameW={NAME_W} height={HEADER_H} />
          <PlannerRows {rows} {range} {zoom} {now} nameW={NAME_W} {blockers} {onopen} onreveal={reveal} {onpreview} {onsave} onlink={link} {onmenu}
            ontoggle={(k) => (collapsed.has(k) ? collapsed.delete(k) : collapsed.add(k))} />
          <ArrowLayer {rows} {deps} {range} {zoom} nameW={NAME_W} top={HEADER_H} selected={selectedArrow}
            onselect={(d) => (selectedArrow = d ? `${d.taskId}:${d.dependsOn}` : null)} onremove={(d) => unlink(d, true)} />
          {#if dropDay !== null}
            <div class="drop" style="left:{NAME_W + dayToX(dropDay, range, zoom)}px; width:{DAY_WIDTH[zoom]}px; top:{HEADER_H}px" aria-hidden="true"></div>
          {/if}
        </div>
      </div>
      <UndatedPanel tasks={undated} bind:open={undatedOpen} {today} {dayAt} {onopen}
        onhover={(d) => (dropDay = d)} onplace={(task, day) => onsave(task, placeUndated(day))} />
    </div>
  {:else}
    <div class="empty"><Doodle /><p>{filtered ? t('table.emptyFiltered') : t('planner.empty')}</p></div>
  {/if}
</div>

{#if barMenu.current}
  {@const task = barMenu.current.target}
  <Menu menu={barMenu} label={task.title}>
    <button role="menuitem" onclick={() => barMenu.act(onopen, false)}>{t('planner.openDetails')}</button>
    <button role="menuitem" onclick={() => startPicking(task)}>{t('dep.dependsOnMenu')}</button>
    {#if menuBlockers.length}
      <div class="sub" aria-hidden="true">{t('dep.remove')} ▸</div>
      {#each menuBlockers as b (b.id)}
        <button role="menuitem" class="indent" aria-label={t('dep.removeNamed', { title: b.title })}
          onclick={() => barMenu.act((x) => unlink({ taskId: x.id, dependsOn: b.id }))}>{b.title}</button>
      {/each}
    {:else}
      <button role="menuitem" disabled>{t('dep.remove')}</button>
    {/if}
    <div class="sep" role="separator"></div>
    <button role="menuitem" disabled={!task.start && !task.due} onclick={() => barMenu.act((x) => onsave(x, NO_DATES))}>{t('planner.removeDates')}</button>
  </Menu>
{/if}

{#if picking}
  <div class="picker-pop" role="dialog" aria-label={t('dep.dependsOnMenu')} style="left:{picking.x}px; top:{picking.y}px" bind:this={pickerEl}>
    <TaskPicker {tasks} exclude={pickerExclude(picking.task.id, deps)} onpick={pick} oncancel={endPicking} />
  </div>
{/if}

<style>
  .sub { font-size: 12px; color: var(--muted); padding: 6px 8px 2px; }
  .indent.indent { padding-left: 18px; max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .picker-pop { position: fixed; z-index: 30; width: 320px; box-shadow: var(--shadow); border-radius: 8px; }
  .page { height: 100%; display: flex; flex-direction: column; padding: 16px 0 0; min-width: 0; }
  .head { display: flex; gap: 12px; align-items: flex-start; padding: 0 24px 12px; }
  .add { flex: 1; min-width: 0; max-width: 560px; }
  .status { display: flex; margin-left: auto; margin-top: 3px; background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 8px; padding: 2px; flex: none; }
  .status button { border: none; background: none; padding: 3px 9px; border-radius: 6px; color: var(--muted); font-size: 12px; }
  .status button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .status button:focus-visible { outline: 2px solid var(--accent); }
  .body { flex: 1; min-height: 0; display: flex; }
  .scroller { flex: 1; min-width: 0; overflow: auto; border-top: 1px solid var(--border); overscroll-behavior: contain; }
  .drop { position: absolute; bottom: 0; z-index: 1; pointer-events: none; background: var(--accent-soft); box-shadow: inset 0 0 0 1px var(--accent); }
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
