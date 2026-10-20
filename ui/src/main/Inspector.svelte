<script lang="ts">
  import { X, Trash2 } from 'lucide-svelte';
  import type { Calendar, EventInput, TaskInput } from '../lib/types';
  import { api, errorMessage } from '../lib/api';
  import { eventToForm, formToEvent, formToTask, recurrenceToRrule, sameInstant, taskToForm, type EventForm, type TaskForm } from '../lib/edit';
  import { ymd } from '../lib/format';

  export type Target =
    | { kind: 'event'; id: number | null; occurrence: string | null; recurring: boolean; input: EventInput }
    | { kind: 'task'; id: number | null; input: TaskInput; completed: boolean };

  let {
    target,
    calendars,
    onclose,
    onsaved,
    askScope,
  }: {
    target: Target;
    calendars: Calendar[];
    onclose: () => void;
    onsaved: () => void;
    askScope: (verb: string, allowThis: boolean) => Promise<'this' | 'all' | null>;
  } = $props();

  let ev = $state<EventForm | null>(null);
  let task = $state<TaskForm | null>(null);
  let error = $state('');
  let confirmDelete = $state(false);

  $effect(() => {
    ev = target.kind === 'event' ? eventToForm(target.input) : null;
    task = target.kind === 'task' ? taskToForm(target.input) : null;
    confirmDelete = false;
    error = '';
  });

  const ALERTS = [0, 5, 10, 15, 30, 60, 120, 1440];
  const alertLabel = (m: number) => (m === 0 ? 'At time of event' : m < 60 ? `${m} min before` : m < 1440 ? `${m / 60} h before` : `${m / 1440} day before`);

  /** Natural-language title: "Lunch tomorrow at 1 at Joe's" fills the other fields. */
  async function reparseTitle() {
    const t = ev ?? task;
    if (!t || !t.title.trim()) return;
    try {
      const p = await api.parsePreview(t.title);
      const meaningful = p.spans.some((s) => s.label !== 'title' && s.label !== 'separator');
      if (!meaningful) return;
      t.title = p.title;
      if (ev) {
        if (p.start) {
          ev.allDay = p.allDay;
          ev.start = p.allDay ? p.start.date : `${p.start.date}T${p.start.time?.slice(0, 5) ?? '09:00'}`;
          ev.end = p.end ? (p.allDay ? p.end.date : `${p.end.date}T${p.end.time?.slice(0, 5)}`) : ev.allDay ? p.start.date : ev.start;
        }
        if (p.location) ev.location = p.location;
        if (p.attendees.length) ev.attendees = p.attendees.join(', ');
        if (p.url) ev.url = p.url;
        if (p.recurrence) {
          ev.repeat = 'custom';
          ev.rrule = recurrenceToRrule(p.recurrence);
        }
        if (p.calendar) ev.calendarId = calendars.find((c) => c.name === p.calendar)?.id ?? ev.calendarId;
        for (const a of p.alerts) if (a.type === 'before' && !ev.alerts.includes(a.value)) ev.alerts = [...ev.alerts, a.value];
      } else if (task && p.start) {
        task.dueDate = p.start.date;
        task.dueTime = p.start.time?.slice(0, 5) ?? '';
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function save() {
    try {
      if (ev && target.kind === 'event') {
        const input = formToEvent(ev);
        if (!input.title) return (error = 'Title is required');
        if (target.id == null) await api.createEvent(input);
        else if (target.recurring && target.occurrence) {
          const scope = await askScope('Change', true);
          if (!scope) return;
          if (scope === 'this') {
            // Detach this occurrence, then apply every edited field to the copy.
            const newId = await api.moveEvent(target.id, target.occurrence, input.start, input.end, 'this');
            await api.updateEvent(newId, { ...input, rrule: null });
          } else {
            // Keep the series anchor; shift it by the edited occurrence delta.
            const delta = new Date(input.start).getTime() - new Date(target.occurrence).getTime();
            const dur = new Date(input.end).getTime() - new Date(input.start).getTime();
            const s = new Date(new Date(target.input.start).getTime() + delta);
            const timesChanged = !sameInstant(input.start, target.input.start) || !sameInstant(input.end, target.input.end);
            const series = await api.getEvent(target.id);
            await api.updateEvent(target.id, timesChanged
              ? { ...input, start: s.toISOString(), end: new Date(s.getTime() + dur).toISOString() }
              : { ...input, start: series.start, end: series.end });
          }
        } else await api.updateEvent(target.id, input);
      } else if (task && target.kind === 'task') {
        const input = formToTask(task);
        if (!input.title) return (error = 'Title is required');
        if (target.id == null) await api.createTask(input);
        else await api.updateTask(target.id, input);
      }
      onsaved();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function remove() {
    if (!confirmDelete) return (confirmDelete = true);
    try {
      if (target.kind === 'event' && target.id != null) {
        const scope = target.recurring ? await askScope('Delete', target.occurrence != null) : 'all';
        if (!scope) return;
        await api.deleteEvent(target.id, target.occurrence, scope);
      } else if (target.kind === 'task' && target.id != null) await api.deleteTask(target.id);
      onsaved();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
    if (e.key === 'Enter' && e.metaKey) save();
  }
</script>

<svelte:window onkeydown={keys} />

<div class="panel" role="dialog" aria-label="Edit item">
  <div class="top">
    <span class="kind">{target.kind === 'event' ? (target.id == null ? 'New event' : 'Event') : target.id == null ? 'New task' : 'Task'}</span>
    <button class="icon-btn" aria-label="Close" onclick={onclose}><X size={16} /></button>
  </div>
  {#if error}<div class="banner"><span>{error}</span></div>{/if}

  {#if ev}
    <input class="field title" bind:value={ev.title} onblur={reparseTitle} placeholder="Title — or type “lunch tomorrow at 1”" />
    <label class="row"><span>Calendar</span>
      <select class="field" bind:value={ev.calendarId}>{#each calendars as c}<option value={c.id}>{c.name}</option>{/each}</select>
    </label>
    <label class="check"><input type="checkbox" bind:checked={ev.allDay} onchange={() => {
      if (!ev) return;
      ev.start = ev.allDay ? ev.start.slice(0, 10) : `${ev.start.slice(0, 10)}T09:00`;
      ev.end = ev.allDay ? ev.end.slice(0, 10) : `${ev.end.slice(0, 10) || ev.start.slice(0, 10)}T10:00`;
    }} /> All-day</label>
    {#if ev.allDay}
      <label class="row"><span>Starts</span><input class="field" type="date" bind:value={ev.start} /></label>
      <label class="row"><span>Ends</span><input class="field" type="date" bind:value={ev.end} min={ev.start} /></label>
    {:else}
      <label class="row"><span>Starts</span><input class="field" type="datetime-local" bind:value={ev.start} /></label>
      <label class="row"><span>Ends</span><input class="field" type="datetime-local" bind:value={ev.end} min={ev.start} /></label>
    {/if}
    <label class="row"><span>Repeat</span>
      <select class="field" bind:value={ev.repeat}>
        <option value="">Never</option><option value="DAILY">Every day</option><option value="WEEKLY">Every week</option>
        <option value="MONTHLY">Every month</option><option value="YEARLY">Every year</option>
        {#if ev.repeat === 'custom'}<option value="custom">Custom ({ev.rrule})</option>{/if}
      </select>
    </label>
    <label class="row"><span>Location</span><input class="field" bind:value={ev.location} /></label>
    <label class="row"><span>People</span><input class="field" bind:value={ev.attendees} placeholder="Anna, Marco" /></label>
    <label class="row"><span>URL</span><input class="field" bind:value={ev.url} /></label>
    <div class="row"><span>Alerts</span>
      <div class="alerts">
        {#each ev.alerts as a, i}
          <span class="pill">{alertLabel(a)}<button class="icon-btn" onclick={() => ev && (ev.alerts = ev.alerts.filter((_, j) => j !== i))}><X size={11} /></button></span>
        {/each}
        <select class="field small" value="" onchange={(e) => {
          const v = Number((e.currentTarget as HTMLSelectElement).value);
          if (ev && !Number.isNaN(v) && !ev.alerts.includes(v)) ev.alerts = [...ev.alerts, v];
          (e.currentTarget as HTMLSelectElement).value = '';
        }}>
          <option value="" disabled>Add alert…</option>
          {#each ALERTS as m}<option value={m}>{alertLabel(m)}</option>{/each}
        </select>
      </div>
    </div>
    <textarea class="field notes" bind:value={ev.notes} placeholder="Notes"></textarea>
  {:else if task}
    <input class="field title" bind:value={task.title} onblur={reparseTitle} placeholder="Task" />
    <label class="row"><span>List</span>
      <select class="field" bind:value={task.calendarId}>{#each calendars as c}<option value={c.id}>{c.name}</option>{/each}</select>
    </label>
    <label class="row"><span>Due</span><input class="field" type="date" bind:value={task.dueDate} /></label>
    <label class="row"><span>Time</span>
      <input class="field" type="time" bind:value={task.dueTime} disabled={!task.dueDate} />
    </label>
    {#if !task.dueDate}<button class="btn link" onclick={() => task && (task.dueDate = ymd(new Date()))}>Due today</button>{/if}
    <textarea class="field notes" bind:value={task.notes} placeholder="Notes"></textarea>
  {/if}

  <div class="actions">
    {#if target.id != null}
      <button class="btn danger" onclick={remove}><Trash2 size={13} /> {confirmDelete ? 'Click again to delete' : 'Delete'}</button>
    {/if}
    <span class="spacer"></span>
    <button class="btn" onclick={onclose}>Cancel</button>
    <button class="btn primary" onclick={save}>Save</button>
  </div>
</div>

<style>
  .panel {
    width: 320px;
    flex: none;
    border-left: 1px solid var(--border);
    background: var(--bg);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }
  .top { display: flex; justify-content: space-between; align-items: center; }
  .kind { font-size: 11px; font-weight: 600; text-transform: uppercase; color: var(--muted); letter-spacing: 0.03em; }
  .title { font-size: 15px; font-weight: 600; }
  .row { display: grid; grid-template-columns: 72px 1fr; align-items: center; gap: 8px; }
  .row > span { color: var(--muted); font-size: 12px; }
  .check { display: flex; align-items: center; gap: 6px; padding-left: 80px; }
  .alerts { display: flex; flex-wrap: wrap; gap: 4px; }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 10px;
    padding: 1px 2px 1px 8px;
    font-size: 11.5px;
  }
  .small { width: auto; }
  .notes { min-height: 90px; resize: vertical; }
  .actions { display: flex; align-items: center; gap: 6px; margin-top: 4px; }
  .actions .danger { display: inline-flex; align-items: center; gap: 4px; }
  .spacer { flex: 1; }
  .link { align-self: flex-start; margin-left: 80px; }
</style>
