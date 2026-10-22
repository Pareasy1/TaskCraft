<script lang="ts">
  import { isMod } from '../lib/platform';
  import { t } from '../lib/i18n';
  import { onMount, tick } from 'svelte';
  import { X } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, errorMessage, onItemsChanged } from '../lib/api';
  import type { Calendar, Hit, ItemRef, Occurrence, Plan, Settings, Task, TaskLabel, TaskStatus } from '../lib/types';
  import { addDays, monthGrid, startOfDay, weekDays } from '../lib/layout';
  import { fmtLongDate, fmtShortDate, fmtTime, monthTitle, ymd } from '../lib/format';
  import Toolbar from './Toolbar.svelte';
  import Sidebar from './Sidebar.svelte';
  import DayWeekGrid from './DayWeekGrid.svelte';
  import MonthGrid from './MonthGrid.svelte';
  import Inspector, { type Target } from './Inspector.svelte';
  import TasksView from './TasksView.svelte';
  import Toast from './Toast.svelte';
  import SettingsDialog from './SettingsDialog.svelte';
  import { notify } from '../lib/toast.svelte';
  import { changeStatus } from '../lib/actions';
  import { activate, clearSelection, isSelected, select } from '../lib/selection.svelte';
  import { sidebarSections } from '../lib/tasks';
  import { taskInput as toTaskInput } from '../lib/edit';

  type View = 'day' | 'week' | 'month' | 'tasks';
  let view = $state<View>('week');
  let anchor = $state(startOfDay(new Date()));
  let sideMonth = $state(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
  let calendars = $state<Calendar[]>([]);
  let settings = $state<Settings | null>(null);
  let occurrences = $state<Occurrence[]>([]);
  let sideOcc = $state<Occurrence[]>([]);
  let allTasks = $state<Task[]>([]);
  let plans = $state<Plan[]>([]);
  /** Which plan the Tasks view shows; 'all' is every plan. */
  let scope = $state<number | 'all'>('all');
  let scopePlan = $derived(scope === 'all' ? null : plans.find((p) => p.id === scope) ?? null);
  let scoped = $derived(scopePlan ? allTasks.filter((x) => x.planId === scopePlan.id) : allTasks);
  // Sidebar groups use the same rules as the Tasks page (see sidebarSections).
  const SECTION_NAMES = {
    overdue: t('group.overdue'), today: t('view.today'), upcoming: t('view.upcoming'), nodate: t('view.nodate'), completed: t('view.completed'),
  };
  let taskSections = $derived(sidebarSections(allTasks, new Date()).map((sec) => ({ ...sec, name: SECTION_NAMES[sec.key] })));
  /** Label filter shared by the Tasks page and the sidebar's label list. */
  let activeLabels = $state<number[]>([]);
  let labels = $state<TaskLabel[]>([]);
  let taskInput = $state<HTMLInputElement | undefined>();
  let query = $state('');
  let hits = $state<Hit[]>([]);
  let target = $state<Target | null>(null);
  let error = $state('');
  let tz = $state('UTC');
  let scopePrompt = $state<{ verb: string; allowThis: boolean; resolve: (s: 'this' | 'all' | null) => void } | null>(null);

  let weekStart = $derived<0 | 1>(settings?.weekStart === 'Sun' ? 0 : 1);
  let days = $derived(view === 'day' ? [anchor] : weekDays(anchor, weekStart));
  let range = $derived.by(() => {
    if (view === 'month') {
      const g = monthGrid(anchor.getFullYear(), anchor.getMonth(), weekStart);
      return [g[0], addDays(g[41], 1)] as const;
    }
    return [days[0], addDays(days[days.length - 1], 1)] as const;
  });
  let title = $derived.by(() => {
    if (view === 'tasks') {
      // The view switcher already says "Tasks"; the title says how many are open.
      if (scopePlan) return scopePlan.name;
      const n = allTasks.filter((x) => !x.completedAt).length;
      return n === 0 ? t('tasks.openNone') : n === 1 ? t('tasks.openOne') : t('tasks.openMany', { n });
    }
    if (view === 'month') return monthTitle(anchor);
    if (view === 'day') return fmtLongDate(anchor);
    const a = days[0], b = days[6];
    return a.getMonth() === b.getMonth() ? monthTitle(a) : `${fmtShortDate(a)} – ${fmtShortDate(b)} ${b.getFullYear()}`;
  });
  let marked = $derived(new Set(sideOcc.map((o) => ymd(new Date(o.start)))));

  async function refresh() {
    try {
      const sg = monthGrid(sideMonth.getFullYear(), sideMonth.getMonth(), weekStart);
      const [occ, side, cals, s, all, lbls, pls] = await Promise.all([
        api.listRange(range[0], range[1]),
        api.listRange(sg[0], addDays(sg[41], 1)),
        api.listCalendars(),
        api.getSettings(),
        api.listTasks('all'),
        api.listLabels(),
        api.listPlans(),
      ]);
      plans = pls;
      // A deleted plan drops the view back to every plan.
      if (scope !== 'all' && !pls.some((p) => p.id === scope)) scope = 'all';
      allTasks = all;
      labels = lbls;
      occurrences = occ;
      sideOcc = side;
      calendars = cals;
      settings = s;
    } catch (e) {
      error = errorMessage(e);
    }
  }

  $effect(() => {
    void range;
    void sideMonth;
    refresh();
  });

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const q = query;
    clearTimeout(searchTimer);
    if (!q.trim()) return void (hits = []);
    searchTimer = setTimeout(async () => {
      try {
        hits = await api.search(q);
      } catch (e) {
        error = errorMessage(e);
      }
    }, 150);
  });

  function shift(n: number) {
    anchor = view === 'month'
      ? new Date(anchor.getFullYear(), anchor.getMonth() + n, 1)
      : addDays(anchor, n * (view === 'day' ? 1 : 7));
  }
  function goToday() {
    anchor = startOfDay(new Date());
  }

  let dialogEl = $state<HTMLDivElement | undefined>();
  let bodyEl = $state<HTMLDivElement | undefined>();
  let settingsOpen = $state(false);
  function askScope(verb: string, allowThis = true): Promise<'this' | 'all' | null> {
    const back = document.activeElement as HTMLElement | null;
    return new Promise((resolve) => (scopePrompt = { verb, allowThis, resolve: (s) => { scopePrompt = null; back?.focus(); resolve(s); } }));
  }
  // Move keyboard focus into the dialog's default button when it opens.
  $effect(() => {
    if (scopePrompt && dialogEl) dialogEl.querySelector<HTMLButtonElement>('.primary, .btn:last-child')?.focus();
  });
  function dialogKeys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      scopePrompt?.resolve(null);
    } else if (e.key === 'Tab' && dialogEl) {
      // Keep Tab inside the dialog.
      const b = [...dialogEl.querySelectorAll<HTMLButtonElement>('button')];
      const i = b.indexOf(document.activeElement as HTMLButtonElement);
      e.preventDefault();
      b[(i + (e.shiftKey ? b.length - 1 : 1)) % b.length]?.focus();
    }
    e.stopPropagation();
  }

  async function openEvent(id: number, occurrence: string | null) {
    select({ kind: 'event', id, occurrence });
    try {
      const e = await api.getEvent(id);
      const { id: _id, ...input } = e;
      void _id;
      const occ = occurrence && e.rrule ? occurrence : null;
      // Show the clicked occurrence's times for recurring events.
      const shown = occ
        ? { ...input, start: occ, end: new Date(new Date(occ).getTime() + (new Date(input.end).getTime() - new Date(input.start).getTime())).toISOString() }
        : input;
      target = { kind: 'event', id, occurrence: occ, recurring: !!e.rrule, input: shown };
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function openTask(id: number) {
    select({ kind: 'task', id });
    try {
      const t = await api.getTask(id);
      target = {
        kind: 'task',
        id,
        completed: !!t.completedAt,
        input: toTaskInput(t),
      };
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function reveal(ref: ItemRef) {
    if (ref.kind === 'event') {
      const e = await api.getEvent(ref.id);
      anchor = startOfDay(new Date(e.start));
      await openEvent(ref.id, null);
    } else await openTask(ref.id);
  }

  function newEvent(start: Date) {
    const cal = settings?.defaultCalendarId ?? calendars[0]?.id ?? 1;
    const end = new Date(start.getTime() + (settings?.defaultEventMinutes ?? 60) * 60_000);
    target = {
      kind: 'event', id: null, occurrence: null, recurring: false,
      input: { calendarId: cal, title: '', start: start.toISOString(), end: end.toISOString(), tz, allDay: false, location: null, attendees: [], notes: null, url: null, rrule: null, alerts: [] },
    };
  }

  async function move(o: Occurrence, start: Date, end: Date) {
    try {
      const scope = o.recurring ? await askScope(t('scope.move'), true) : 'all';
      if (!scope) return;
      await api.moveEvent(o.eventId, o.recurring ? o.start : null, start.toISOString(), end.toISOString(), scope);
      const msg = t('msg.movedTo', { title: o.title, where: `${fmtShortDate(start)}${o.allDay ? '' : `, ${fmtTime(start)}`}` });
      // Undo restores a single event; a change to a repeating series has no undo.
      notify(msg, o.recurring ? undefined : () => api.moveEvent(o.eventId, null, o.start, o.end, 'all').then(() => {}));
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function toggleCalendar(c: Calendar) {
    try {
      await api.upsertCalendar({ id: c.id, name: c.name, color: c.color, visible: !c.visible });
    } catch (e) {
      error = errorMessage(e);
    }
  }

  /** Moves a task to a new due date from the calendar, with Undo. */
  async function moveTask(task: Task, due: string) {
    const before = toTaskInput(task);
    try {
      await api.updateTask(task.id, { ...before, due });
      const d = new Date(due);
      notify(t('msg.movedTo', { title: task.title, where: `${fmtShortDate(d)}${task.dueHasTime ? `, ${fmtTime(d)}` : ''}` }), () =>
        api.updateTask(task.id, before),
      );
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function setStatus(t: Task, s: TaskStatus) {
    try {
      await changeStatus(t, s);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape' && !target && !settingsOpen && !scopePrompt) return clearSelection();
    if (!isMod(e)) return;
    const tag = (e.target as HTMLElement)?.tagName;
    const views: Record<string, View> = { '1': 'day', '2': 'week', '3': 'month', '4': 'tasks' };
    if (views[e.key]) {
      view = views[e.key];
      // Land keyboard focus in the view so the next Tab reaches its content.
      tick().then(() => bodyEl?.focus());
    }
    else if (e.key === ',') settingsOpen = true;
    else if (e.key === 't' && tag !== 'INPUT') goToday();
    else if (e.key === 'n' && view === 'tasks') taskInput?.focus();
    else if (e.key === 'n') {
      const s = new Date();
      s.setMinutes(0, 0, 0);
      s.setHours(s.getHours() + 1);
      newEvent(s);
    } else return;
    e.preventDefault();
  }

  onMount(() => {
    api.localTimeZone().then((z) => (tz = z));
    invoke<ItemRef | null>('take_pending_focus').then((r) => r && reveal(r));
    // Settings opened from the menu bar or the popover (possibly before this window loaded).
    invoke<boolean>('take_pending_settings').then((open) => open && (settingsOpen = true));
    const un = [onItemsChanged(refresh), listen<ItemRef>('focus-item', (e) => {
      invoke('take_pending_focus');
      reveal(e.payload);
    }), listen('open-settings', () => {
      invoke('take_pending_settings');
      settingsOpen = true;
    })];
    return () => un.forEach((p) => p.then((f) => f()));
  });
</script>

<svelte:window onkeydown={keys} />

<div class="app">
  <Sidebar
    {calendars}
    bind:month={sideMonth}
    selected={anchor}
    {marked}
    {weekStart}
    {taskSections}
    tasksMode={view === 'tasks'}
    {labels}
    tasks={allTasks}
    bind:activeLabels
    onerror={(m) => (error = m)}
    onopensettings={() => (settingsOpen = true)}
    {plans}
    {scope}
    onselectscope={(s) => { scope = s; view = 'tasks'; }}
    onselectday={(d) => (anchor = startOfDay(d))}
    ontogglecalendar={toggleCalendar}
    onstatus={setStatus}
    onopentask={(t) => openTask(t.id)}
  />
  <main class="content">
    <Toolbar bind:view {title} bind:query onprev={() => shift(-1)} onnext={() => shift(1)} ontoday={goToday}
      onnew={() => { if (view === 'tasks') return taskInput?.focus(); const s = new Date(); s.setMinutes(0, 0, 0); s.setHours(s.getHours() + 1); newEvent(s); }} />
    {#if error}<div class="banner err" role="alert"><span>{error}</span><button class="icon-btn" aria-label={t('common.dismiss')} onclick={() => (error = '')}><X size={14} aria-hidden="true" /></button></div>{/if}
    <div class="body" tabindex="-1" aria-label={title} bind:this={bodyEl}>
      {#if hits.length || query.trim()}
        <div class="results">
          <div class="rhead">{hits.length === 1 ? t('main.result', { q: query }) : t('main.results', { n: hits.length, q: query })}</div>
          {#each hits as h (h.kind + h.id)}
            {@const openHit = () => (h.kind === 'event' ? reveal({ kind: 'event', id: h.id }) : openTask(h.id))}
            <button class="hit" class:is-selected={isSelected(h.kind, h.id, null)}
              onclick={(e) => activate(e, openHit, () => select({ kind: h.kind, id: h.id }))} ondblclick={openHit}>
              <span class="hk">{h.kind === 'event' ? t('common.event') : t('common.task')}</span>
              <span class="ht">{h.title}</span>
              <span class="hw">{h.when ? `${fmtShortDate(new Date(h.when))} ${fmtTime(new Date(h.when))}` : ''}</span>
            </button>
          {/each}
        </div>
      {:else if view === 'tasks'}
        <TasksView tasks={scoped} {labels} bind:input={taskInput} bind:active={activeLabels} onopen={(t) => openTask(t.id)} onerror={(m) => (error = m)} />
      {:else if view === 'month'}
        <MonthGrid month={anchor} {weekStart} {occurrences} tasks={allTasks}
          ontaskopen={(task) => openTask(task.id)} ontaskstatus={setStatus}
          onselect={(o) => openEvent(o.eventId, o.start)}
          onday={(d) => { anchor = startOfDay(d); view = 'day'; }} />
      {:else}
        <DayWeekGrid {days} {occurrences} tasks={allTasks}
          ontaskopen={(task) => openTask(task.id)} ontaskstatus={setStatus} ontaskmove={moveTask}
          onselect={(o) => openEvent(o.eventId, o.start)}
          oncreate={newEvent}
          onmove={move} />
      {/if}
    </div>
  </main>
  {#if target}
    <Inspector {target} {calendars} {labels} onclose={() => (target = null)} onsaved={() => (target = null)} {askScope} />
  {/if}
</div>

<Toast />

{#if settingsOpen}<SettingsDialog onclose={() => (settingsOpen = false)} />{/if}

{#if scopePrompt}
  <div class="scrim">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="scope-title" tabindex="-1" bind:this={dialogEl} onkeydown={dialogKeys}>
      <strong id="scope-title">{t('scope.title', { verb: scopePrompt.verb })}</strong>
      <p class="muted">
        {scopePrompt.allowThis
          ? t('scope.question')
          : t('scope.allOnly')}
      </p>
      <div class="dactions">
        <button class="btn" onclick={() => scopePrompt?.resolve(null)}>{t('common.cancel')}</button>
        <button class="btn" onclick={() => scopePrompt?.resolve('all')}>{t('scope.all')}</button>
        {#if scopePrompt.allowThis}
          <button class="btn primary" onclick={() => scopePrompt?.resolve('this')}>{t('scope.this')}</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .app { display: flex; height: 100%; }
  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .body { flex: 1; min-height: 0; }
  /* Focus lands here from ⌘1–4 (Ctrl+1–4); the region itself needs no ring. */
  .body:focus { outline: none; }
  .err { margin: 8px 12px 0; }
  .results { padding: 10px 16px; overflow-y: auto; height: 100%; }
  .rhead { color: var(--muted); margin-bottom: 6px; }
  .hit {
    width: 100%;
    display: grid;
    grid-template-columns: max-content 1fr auto;
    gap: 10px;
    border: none;
    background: none;
    text-align: left;
    padding: 6px 8px;
    border-radius: 6px;
  }
  .hit:hover { background: var(--hover); }
  .hk { color: var(--muted); font-size: 11px; padding-top: 2px; }
  .hw { color: var(--muted); }
  /* Above the item dialog, which asks this question (z-index 25). */
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.25); display: grid; place-items: center; z-index: 40; }
  .dialog { background: var(--bg); border-radius: 12px; padding: 16px 18px; width: 340px; box-shadow: var(--shadow); }
  .dialog p { margin: 6px 0 14px; }
  .dactions { display: flex; justify-content: flex-end; gap: 6px; }
</style>
