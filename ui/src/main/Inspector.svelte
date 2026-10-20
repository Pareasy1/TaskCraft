<script lang="ts">
  import { t } from '../lib/i18n';
  import { X, Trash2 } from 'lucide-svelte';
  import type { Calendar, EventInput, TaskInput, TaskLabel } from '../lib/types';
  import { STATUSES, STATUS_LABEL } from '../lib/tasks';
  import StatusGlyph from './tasks/StatusGlyph.svelte';
  import { api, errorMessage } from '../lib/api';
  import { eventToForm, formToEvent, formToTask, mergeTaskSave, recurrenceToRrule, sameInstant, taskInput, taskToForm, type EventForm, type TaskForm } from '../lib/edit';
  import { ymd } from '../lib/format';
  import { notify } from '../lib/toast.svelte';

  export type Target =
    | { kind: 'event'; id: number | null; occurrence: string | null; recurring: boolean; input: EventInput }
    | { kind: 'task'; id: number | null; input: TaskInput; completed: boolean };

  let {
    target,
    calendars,
    labels = [],
    onclose,
    onsaved,
    askScope,
  }: {
    target: Target;
    calendars: Calendar[];
    labels?: TaskLabel[];
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

  let labelText = $state('');
  const labelColor = (name: string) => labels.find((l) => l.name.toLowerCase() === name.toLowerCase())?.color ?? 'var(--faint)';
  function addLabel() {
    const name = labelText.trim().replace(/^#/, '').trim();
    labelText = '';
    if (!task || !name) return;
    if (name.length > 32) return void (error = t('inspector.labelLength'));
    if (!task.labels.some((l) => l.toLowerCase() === name.toLowerCase())) task.labels = [...task.labels, name];
  }

  const ALERTS = [0, 5, 10, 15, 30, 60, 120, 1440];
  const alertLabel = (m: number) =>
    m === 0 ? t('alert.atTime') : m < 60 ? t('alert.minBefore', { n: m }) : m < 1440 ? t('alert.hoursBefore', { n: m / 60 }) : t('alert.daysBefore', { n: m / 1440 });

  /** Natural-language title: "Lunch tomorrow at 1 at Joe's" fills the other fields. */
  async function reparseTitle() {
    const form = ev ?? task;
    if (!form || !form.title.trim()) return;
    try {
      const p = await api.parsePreview(form.title);
      const meaningful = p.spans.some((s) => s.label !== 'title' && s.label !== 'separator');
      if (!meaningful) return;
      form.title = p.title;
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
        if (!input.title) return (error = t('inspector.titleRequired'));
        if (target.id == null) await api.createEvent(input);
        else if (target.recurring && target.occurrence) {
          const scope = await askScope(t('scope.change'), true);
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
        if (!input.title) return (error = t('inspector.titleRequired'));
        if (target.id == null) await api.createTask(input);
        else {
          const latest = taskInput(await api.getTask(target.id));
          await api.updateTask(target.id, mergeTaskSave(target.input, input, latest));
        }
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
        const scope = target.recurring ? await askScope(t('scope.delete'), target.occurrence != null) : 'all';
        if (!scope) return;
        const { id: _id, ...saved } = await api.getEvent(target.id);
        void _id;
        await api.deleteEvent(target.id, target.occurrence, scope);
        // A single event can be recreated; a repeating series change has no undo.
        notify(t('msg.deleted', { title: saved.title }), target.recurring ? undefined : () => api.createEvent(saved).then(() => {}));
      } else if (target.kind === 'task' && target.id != null) {
        const saved = taskInput(await api.getTask(target.id));
        await api.deleteTask(target.id);
        notify(t('msg.deleted', { title: saved.title }), () => api.createTask(saved).then(() => {}));
      }
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

<div class="panel" role="dialog" aria-label={t('inspector.label')}>
  <div class="top">
    <span class="kind">{target.kind === 'event' ? (target.id == null ? t('common.newEvent') : t('common.event')) : target.id == null ? t('common.newTask') : t('common.task')}</span>
    <button class="icon-btn" aria-label={t('common.close')} onclick={onclose}><X size={16} /></button>
  </div>
  {#if error}<div class="banner" role="alert"><span>{error}</span></div>{/if}

  {#if ev}
    <input class="field title" bind:value={ev.title} onblur={reparseTitle} placeholder={t('inspector.titlePlaceholder')} />
    <label class="row"><span>{t('inspector.calendar')}</span>
      <select class="field" bind:value={ev.calendarId}>{#each calendars as c}<option value={c.id}>{c.name}</option>{/each}</select>
    </label>
    <label class="check"><input type="checkbox" bind:checked={ev.allDay} onchange={() => {
      if (!ev) return;
      ev.start = ev.allDay ? ev.start.slice(0, 10) : `${ev.start.slice(0, 10)}T09:00`;
      ev.end = ev.allDay ? ev.end.slice(0, 10) : `${ev.end.slice(0, 10) || ev.start.slice(0, 10)}T10:00`;
    }} /> {t('inspector.allDay')}</label>
    {#if ev.allDay}
      <label class="row"><span>{t('inspector.starts')}</span><input class="field" type="date" bind:value={ev.start} /></label>
      <label class="row"><span>{t('inspector.ends')}</span><input class="field" type="date" bind:value={ev.end} min={ev.start} /></label>
    {:else}
      <label class="row"><span>{t('inspector.starts')}</span><input class="field" type="datetime-local" bind:value={ev.start} /></label>
      <label class="row"><span>{t('inspector.ends')}</span><input class="field" type="datetime-local" bind:value={ev.end} min={ev.start} /></label>
    {/if}
    <label class="row"><span>{t('inspector.repeat')}</span>
      <select class="field" bind:value={ev.repeat}>
        <option value="">{t('inspector.never')}</option><option value="DAILY">{t('inspector.daily')}</option><option value="WEEKLY">{t('inspector.weekly')}</option>
        <option value="MONTHLY">{t('inspector.monthly')}</option><option value="YEARLY">{t('inspector.yearly')}</option>
        {#if ev.repeat === 'custom'}<option value="custom">{t('inspector.custom', { rule: ev.rrule ?? '' })}</option>{/if}
      </select>
    </label>
    <label class="row"><span>{t('inspector.location')}</span><input class="field" bind:value={ev.location} /></label>
    <label class="row"><span>{t('inspector.people')}</span><input class="field" bind:value={ev.attendees} placeholder={t('inspector.peoplePlaceholder')} /></label>
    <label class="row"><span>{t('common.url')}</span><input class="field" bind:value={ev.url} /></label>
    <div class="row"><span>{t('inspector.alerts')}</span>
      <div class="alerts">
        {#each ev.alerts as a, i}
          <span class="pill">{alertLabel(a)}<button class="icon-btn" aria-label={t('inspector.removeAlert', { alert: alertLabel(a) })} onclick={() => ev && (ev.alerts = ev.alerts.filter((_, j) => j !== i))}><X size={11} /></button></span>
        {/each}
        <select class="field small" value="" onchange={(e) => {
          const v = Number((e.currentTarget as HTMLSelectElement).value);
          if (ev && !Number.isNaN(v) && !ev.alerts.includes(v)) ev.alerts = [...ev.alerts, v];
          (e.currentTarget as HTMLSelectElement).value = '';
        }}>
          <option value="" disabled>{t('inspector.addAlert')}</option>
          {#each ALERTS as m}<option value={m}>{alertLabel(m)}</option>{/each}
        </select>
      </div>
    </div>
    <textarea class="field notes" bind:value={ev.notes} placeholder={t('inspector.notes')}></textarea>
  {:else if task}
    <input class="field title" bind:value={task.title} onblur={reparseTitle} placeholder={t('inspector.taskPlaceholder')} />
    <div class="row"><span>{t('inspector.status')}</span>
      <div class="seg" role="radiogroup" aria-label={t('inspector.status')}>
        {#each STATUSES as s}
          <button role="radio" aria-checked={task.status === s} class:on={task.status === s} onclick={() => task && (task.status = s)}>
            <StatusGlyph status={s} size={13} />{STATUS_LABEL[s]}
          </button>
        {/each}
      </div>
    </div>
    <div class="row top-align"><span>{t('inspector.labels')}</span>
      <div class="lbls">
        {#each task.labels as name, i (name)}
          <span class="lbl"><span class="hash" style="color:{labelColor(name)}">#</span>{name}<button class="icon-btn" aria-label={t('inspector.removeLabel', { name })} onclick={() => task && (task.labels = task.labels.filter((_, j) => j !== i))}><X size={11} /></button></span>
        {/each}
        <input class="field small" list="label-names" bind:value={labelText} placeholder={t('inspector.addLabel')} maxlength="33"
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ',') { e.preventDefault(); addLabel(); } }} onblur={addLabel} />
        <datalist id="label-names">{#each labels as l}<option value={l.name}></option>{/each}</datalist>
      </div>
    </div>
    <label class="row"><span>{t('inspector.list')}</span>
      <select class="field" bind:value={task.calendarId}>{#each calendars as c}<option value={c.id}>{c.name}</option>{/each}</select>
    </label>
    <label class="row"><span>{t('inspector.due')}</span><input class="field" type="date" bind:value={task.dueDate} /></label>
    <label class="row"><span>{t('inspector.time')}</span>
      <input class="field" type="time" bind:value={task.dueTime} disabled={!task.dueDate} />
    </label>
    {#if !task.dueDate}<button class="btn link" onclick={() => task && (task.dueDate = ymd(new Date()))}>{t('inspector.dueToday')}</button>{/if}
    <textarea class="field notes" bind:value={task.notes} placeholder={t('inspector.notes')}></textarea>
  {/if}

  <div class="actions">
    {#if target.id != null}
      <button class="btn danger" onclick={remove}><Trash2 size={13} /> {confirmDelete ? t('inspector.confirmDelete') : t('common.delete')}</button>
    {/if}
    <span class="spacer"></span>
    <button class="btn" onclick={onclose}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={save}>{t('common.save')}</button>
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
  .kind { font-size: 12px; font-weight: 600; color: var(--muted); }
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
  .top-align { align-items: start; }
  .top-align > span { padding-top: 4px; }
  .seg { display: flex; background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 8px; padding: 2px; }
  .seg button { flex: 1; display: flex; align-items: center; justify-content: center; gap: 5px; border: none; background: none; padding: 4px 2px; border-radius: 6px; font-size: 11.5px; color: var(--muted); white-space: nowrap; }
  .seg button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .seg button:focus-visible { outline: 2px solid var(--accent); }
  .lbls { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .lbl { display: inline-flex; align-items: center; font-size: 12px; background: var(--hover); border-radius: 6px; padding: 1px 1px 1px 6px; }
  .hash { font-weight: 700; margin-right: 1px; }
  .lbls input { flex: 1; min-width: 90px; }
  .notes { min-height: 90px; resize: vertical; }
  .actions { display: flex; align-items: center; gap: 6px; margin-top: 4px; }
  .actions .danger { display: inline-flex; align-items: center; gap: 4px; }
  .spacer { flex: 1; }
  .link { align-self: flex-start; margin-left: 80px; }
</style>
