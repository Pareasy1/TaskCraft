<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, errorMessage, onItemsChanged } from '../lib/api';
  import type { Calendar, Hit, ItemRef, Occurrence, Settings, Task, TaskLabel } from '../lib/types';
  import { addDays, monthGrid, startOfDay, weekDays } from '../lib/layout';
  import { fmtShortDate, fmtTime, monthTitle, ymd } from '../lib/format';
  import Toolbar from './Toolbar.svelte';
  import Sidebar from './Sidebar.svelte';
  import DayWeekGrid from './DayWeekGrid.svelte';
  import MonthGrid from './MonthGrid.svelte';
  import Inspector, { type Target } from './Inspector.svelte';
  import TasksView from './TasksView.svelte';

  type View = 'day' | 'week' | 'month' | 'tasks';
  let view = $state<View>('week');
  let anchor = $state(startOfDay(new Date()));
  let sideMonth = $state(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
  let calendars = $state<Calendar[]>([]);
  let settings = $state<Settings | null>(null);
  let occurrences = $state<Occurrence[]>([]);
  let sideOcc = $state<Occurrence[]>([]);
  let taskSections = $state<{ name: string; tasks: Task[] }[]>([]);
  let allTasks = $state<Task[]>([]);
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
    if (view === 'tasks') return 'Tasks';
    if (view === 'month') return monthTitle(anchor);
    if (view === 'day') return anchor.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
    const a = days[0], b = days[6];
    return a.getMonth() === b.getMonth() ? monthTitle(a) : `${fmtShortDate(a)} – ${fmtShortDate(b)} ${b.getFullYear()}`;
  });
  let marked = $derived(new Set(sideOcc.map((o) => ymd(new Date(o.start)))));

  async function refresh() {
    try {
      const sg = monthGrid(sideMonth.getFullYear(), sideMonth.getMonth(), weekStart);
      const [occ, side, cals, s, overdue, today, upcoming, completed, all, lbls] = await Promise.all([
        api.listRange(range[0], range[1]),
        api.listRange(sg[0], addDays(sg[41], 1)),
        api.listCalendars(),
        api.getSettings(),
        api.listTasks('overdue'),
        api.listTasks('today'),
        api.listTasks('upcoming'),
        api.listTasks('completed'),
        api.listTasks('all'),
        api.listLabels(),
      ]);
      allTasks = all;
      labels = lbls;
      occurrences = occ;
      sideOcc = side;
      calendars = cals;
      settings = s;
      taskSections = [
        { name: 'Overdue', tasks: overdue },
        { name: 'Today', tasks: today },
        { name: 'Upcoming', tasks: upcoming },
        { name: 'Completed', tasks: completed.slice(0, 30) },
      ];
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

  function askScope(verb: string, allowThis = true): Promise<'this' | 'all' | null> {
    return new Promise((resolve) => (scopePrompt = { verb, allowThis, resolve: (s) => { scopePrompt = null; resolve(s); } }));
  }

  async function openEvent(id: number, occurrence: string | null) {
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
    try {
      const t = await api.getTask(id);
      target = {
        kind: 'task',
        id,
        completed: !!t.completedAt,
        input: { calendarId: t.calendarId, title: t.title, due: t.due, dueHasTime: t.dueHasTime, tz: t.tz, rrule: t.rrule, notes: t.notes, alerts: t.alerts, status: t.status, labels: t.labels.map((l) => l.name) },
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
      const scope = o.recurring ? await askScope('Move', true) : 'all';
      if (!scope) return;
      await api.moveEvent(o.eventId, o.recurring ? o.start : null, start.toISOString(), end.toISOString(), scope);
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

  async function completeTask(t: Task) {
    try {
      await api.setTaskCompleted(t.id, !t.completedAt);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function keys(e: KeyboardEvent) {
    if (!e.metaKey) return;
    const tag = (e.target as HTMLElement)?.tagName;
    if (e.key === '1') view = 'day';
    else if (e.key === '2') view = 'week';
    else if (e.key === '3') view = 'month';
    else if (e.key === '4') view = 'tasks';
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
    const un = [onItemsChanged(refresh), listen<ItemRef>('focus-item', (e) => {
      invoke('take_pending_focus');
      reveal(e.payload);
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
    onselectday={(d) => (anchor = startOfDay(d))}
    ontogglecalendar={toggleCalendar}
    oncomplete={completeTask}
    onopentask={(t) => openTask(t.id)}
  />
  <section class="content">
    <Toolbar bind:view {title} bind:query onprev={() => shift(-1)} onnext={() => shift(1)} ontoday={goToday}
      onnew={() => { if (view === 'tasks') return taskInput?.focus(); const s = new Date(); s.setMinutes(0, 0, 0); s.setHours(s.getHours() + 1); newEvent(s); }} />
    {#if error}<div class="banner err"><span>{error}</span><button class="icon-btn" onclick={() => (error = '')}>✕</button></div>{/if}
    <div class="body">
      {#if hits.length || query.trim()}
        <div class="results">
          <div class="rhead">{hits.length} result{hits.length === 1 ? '' : 's'} for “{query}”</div>
          {#each hits as h (h.kind + h.id)}
            <button class="hit" onclick={() => (h.kind === 'event' ? reveal({ kind: 'event', id: h.id }) : openTask(h.id))}>
              <span class="hk">{h.kind}</span>
              <span class="ht">{h.title}</span>
              <span class="hw">{h.when ? `${fmtShortDate(new Date(h.when))} ${fmtTime(new Date(h.when))}` : ''}</span>
            </button>
          {/each}
        </div>
      {:else if view === 'tasks'}
        <TasksView tasks={allTasks} {labels} bind:input={taskInput} onopen={(t) => openTask(t.id)} onerror={(m) => (error = m)} />
      {:else if view === 'month'}
        <MonthGrid month={anchor} {weekStart} {occurrences}
          onselect={(o) => openEvent(o.eventId, o.start)}
          onday={(d) => { anchor = startOfDay(d); view = 'day'; }} />
      {:else}
        <DayWeekGrid {days} {occurrences}
          onselect={(o) => openEvent(o.eventId, o.start)}
          oncreate={newEvent}
          onmove={move} />
      {/if}
    </div>
  </section>
  {#if target}
    <Inspector {target} {calendars} {labels} onclose={() => (target = null)} onsaved={() => (target = null)} {askScope} />
  {/if}
</div>

{#if scopePrompt}
  <div class="scrim">
    <div class="dialog" role="dialog" aria-label="Repeating event">
      <strong>{scopePrompt.verb} a repeating event</strong>
      <p class="muted">
        {scopePrompt.allowThis
          ? 'Apply the change to this occurrence only, or to every event in the series?'
          : 'This applies to every event in the series. Open a single occurrence from the calendar to change just that one.'}
      </p>
      <div class="dactions">
        <button class="btn" onclick={() => scopePrompt?.resolve(null)}>Cancel</button>
        <button class="btn" onclick={() => scopePrompt?.resolve('all')}>All events</button>
        {#if scopePrompt.allowThis}
          <button class="btn primary" onclick={() => scopePrompt?.resolve('this')}>This event</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .app { display: flex; height: 100%; }
  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .body { flex: 1; min-height: 0; }
  .err { margin: 8px 12px 0; }
  .results { padding: 10px 16px; overflow-y: auto; height: 100%; }
  .rhead { color: var(--muted); margin-bottom: 6px; }
  .hit {
    width: 100%;
    display: grid;
    grid-template-columns: 50px 1fr auto;
    gap: 10px;
    border: none;
    background: none;
    text-align: left;
    padding: 6px 8px;
    border-radius: 6px;
  }
  .hit:hover { background: var(--hover); }
  .hk { color: var(--faint); text-transform: uppercase; font-size: 10px; padding-top: 2px; }
  .hw { color: var(--muted); }
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.25); display: grid; place-items: center; z-index: 20; }
  .dialog { background: var(--bg); border-radius: 12px; padding: 16px 18px; width: 340px; box-shadow: var(--shadow); }
  .dialog p { margin: 6px 0 14px; }
  .dactions { display: flex; justify-content: flex-end; gap: 6px; }
</style>
