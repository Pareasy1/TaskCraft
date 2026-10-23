<script lang="ts">
  import { isMod } from '../lib/platform';
  import { t } from '../lib/i18n';
  import { onDestroy, tick } from 'svelte';
  import { X, Trash2, CornerLeftUp } from 'lucide-svelte';
  import { PRIORITIES, type Bucket, type Calendar, type EventInput, type Plan, type Task, type TaskInput, type TaskLabel } from '../lib/types';
  import { STATUSES, STATUS_LABEL } from '../lib/tasks';
  import StatusGlyph from './tasks/StatusGlyph.svelte';
  import SubtaskList from './tasks/SubtaskList.svelte';
  import Checklist from './tasks/Checklist.svelte';
  import { api, errorMessage, onItemsChanged } from '../lib/api';
  import { eventToForm, formToEvent, formToTask, mergeTaskSave, recurrenceToRrule, sameInstant, startAfterDue, taskFormDirty, taskInput, taskToForm, type EventForm, type TaskForm } from '../lib/edit';
  import { parentCandidates } from '../lib/subtasks';
  import { ymd } from '../lib/format';
  import { notify } from '../lib/toast.svelte';

  export type Target =
    | { kind: 'event'; id: number | null; occurrence: string | null; recurring: boolean; input: EventInput }
    | { kind: 'task'; id: number | null; input: TaskInput; completed: boolean };

  let {
    target,
    calendars,
    labels = [],
    plans = [],
    onclose,
    onsaved,
    askScope,
    onopentask,
  }: {
    target: Target;
    calendars: Calendar[];
    labels?: TaskLabel[];
    plans?: Plan[];
    onclose: () => void;
    onsaved: () => void;
    askScope: (verb: string, allowThis: boolean) => Promise<'this' | 'all' | null>;
    /** Opens another task's details in place of this one (a subtask, or the parent). */
    onopentask?: (id: number) => void;
  } = $props();

  let ev = $state<EventForm | null>(null);
  let task = $state<TaskForm | null>(null);
  let error = $state('');
  let confirmDelete = $state(false);

  // Focus moves into the panel when it opens and back to where it came from when it closes.
  const opener = document.activeElement as HTMLElement | null;
  let panelEl = $state<HTMLDivElement | undefined>();
  $effect(() => {
    void target;
    tick().then(() => panelEl?.querySelector<HTMLInputElement>('.field.title')?.focus());
  });
  onDestroy(() => {
    if (opener?.isConnected) opener.focus();
  });

  $effect(() => {
    ev = target.kind === 'event' ? eventToForm(target.input) : null;
    task = target.kind === 'task' ? taskToForm(target.input) : null;
    parentPick = target.kind === 'task' ? target.input.parentId ?? null : null;
    confirmDelete = false;
    error = '';
  });

  // ── A stored task's subtasks, parent and parent choices (live: reloaded on every change) ──
  let stored = $state<Task | null>(null);
  let subtasks = $state<Task[]>([]);
  let allTasks = $state<Task[]>([]);
  let parentTask = $state<Task | null>(null);
  /** The parent picked in the dialog; applied with `setTaskParent` on Save. */
  let parentPick = $state<number | null>(null);
  async function loadRelated() {
    const id = target.kind === 'task' ? target.id : null;
    if (id == null) return void ((stored = parentTask = null), (subtasks = allTasks = []));
    try {
      const [me, subs, all] = await Promise.all([api.getTask(id), api.listSubtasks(id), api.listTasks('all')]);
      if (target.kind !== 'task' || target.id !== id) return;
      stored = me;
      subtasks = subs;
      allTasks = all;
      parentTask = me.parentId == null ? null : all.find((x) => x.id === me.parentId) ?? (await api.getTask(me.parentId));
    } catch (e) {
      error = errorMessage(e);
    }
  }
  $effect(() => {
    void target;
    loadRelated();
  });
  const unlisten = onItemsChanged(() => loadRelated());
  onDestroy(() => void unlisten.then((f) => f()));

  let candidates = $derived(stored && task?.planId != null ? parentCandidates(allTasks, stored.id, task.planId, parentPick) : []);
  /** A task with subtasks or a repeat can't become a subtask, so it gets no parent picker. */
  let canHaveParent = $derived(!!stored && !subtasks.length && !stored.rrule);
  let isSubtask = $derived(stored?.parentId != null);

  // A start time goes with a due time: clearing the due time clears it.
  $effect(() => {
    if (task && task.startTime && !(task.dueDate && task.dueTime)) task.startTime = '';
  });
  let startInvalid = $derived(!!task && startAfterDue(formToTask(task)));

  // Plan and bucket: the bucket list follows the chosen plan.
  let planBuckets = $state<Bucket[]>([]);
  $effect(() => {
    const id = task?.planId;
    if (id == null) return void (planBuckets = []);
    api.listBuckets(id).then(
      (list) => {
        if (task?.planId !== id) return;
        planBuckets = list;
        // A new task (or one whose bucket vanished) takes the plan's first bucket.
        if (!list.some((b) => b.id === task?.bucketId) && task) task.bucketId = list[0]?.id ?? null;
      },
      (e) => (error = errorMessage(e)),
    );
  });
  function pickPlan(id: number) {
    if (!task || id === task.planId) return;
    task.planId = id;
    task.bucketId = null; // set to the plan's first bucket once its buckets load
    planBuckets = []; // never offer the old plan's buckets meanwhile
  }

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

  // ── Saving, closing and moving to another task never drop what was typed ──
  let subtaskList = $state<SubtaskList | undefined>();
  let checklist = $state<Checklist | undefined>();
  /** Adds the text still sitting in the "Add subtask" / "Add item" fields; false if that failed (the error shows). */
  async function flushAdds(): Promise<boolean> {
    try {
      await subtaskList?.flush();
      await checklist?.flush();
      return true;
    } catch {
      return false;
    }
  }
  async function save() {
    if ((await flushAdds()) && (await persist())) onsaved();
  }
  async function close() {
    if (await flushAdds()) onclose();
  }
  /** Opens a subtask or the parent; unsaved edits to this task are saved first (and on failure it stays). */
  async function openOther(id: number) {
    if (!(await flushAdds())) return;
    if (target.kind === 'task' && target.id != null && task && taskFormDirty(target.input, task, parentPick) && !(await persist())) return;
    onopentask?.(id);
  }

  /** Writes the form to the store; false (with the error shown) when it didn't. */
  async function persist(): Promise<boolean> {
    try {
      if (ev && target.kind === 'event') {
        const input = formToEvent(ev);
        if (!input.title) { error = t('inspector.titleRequired'); return false; }
        if (target.id == null) await api.createEvent(input);
        else if (target.recurring && target.occurrence) {
          const scope = await askScope(t('scope.change'), true);
          if (!scope) return false;
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
        if (!input.title) { error = t('inspector.titleRequired'); return false; }
        if (startAfterDue(input)) { error = t('error.start_after_due'); return false; }
        if (target.id == null) await api.createTask(input);
        else {
          // The parent goes first: its rules (no nesting, no repeat) may refuse it.
          if (parentPick !== (target.input.parentId ?? null)) await api.setTaskParent(target.id, parentPick);
          const latest = taskInput(await api.getTask(target.id));
          const out = mergeTaskSave(target.input, input, latest);
          // A subtask lives in its parent's plan and bucket.
          if (latest.parentId != null) Object.assign(out, { planId: latest.planId, bucketId: latest.bucketId });
          await api.updateTask(target.id, out);
        }
      }
      return true;
    } catch (e) {
      error = errorMessage(e);
      return false;
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
        const id = target.id;
        const [me, subs, list] = await Promise.all([api.getTask(id), api.listSubtasks(id), api.listChecklist(id)]);
        const saved = taskInput(me);
        await api.deleteTask(id);
        // Undo brings back its subtasks and checklist too (they went with it).
        notify(t('msg.deleted', { title: saved.title }), async () => {
          const newId = await api.createTask(saved);
          for (const s of subs) await api.createTask({ ...taskInput(s), parentId: newId });
          for (const item of list) {
            const itemId = await api.addChecklistItem(newId, item.text);
            if (item.done) await api.updateChecklistItem(itemId, { done: true });
          }
        });
      }
      onsaved();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  // Keep Tab inside the dialog.
  function trapTab(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !panelEl) return;
    const list = [...panelEl.querySelectorAll<HTMLElement>('button, input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter(
      (el) => !(el as HTMLButtonElement).disabled && el.offsetParent !== null,
    );
    if (!list.length) return;
    const i = list.indexOf(document.activeElement as HTMLElement);
    e.preventDefault();
    list[e.shiftKey ? (i <= 0 ? list.length - 1 : i - 1) : i === list.length - 1 ? 0 : i + 1].focus();
  }

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
    if (e.key === 'Enter' && isMod(e)) save();
  }
</script>

<svelte:window onkeydown={keys} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (backdrop click closes; Esc is the keyboard path) -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && close()}>
<div class="panel" role="dialog" aria-modal="true" aria-label={t('inspector.label')} tabindex="-1" bind:this={panelEl} onkeydown={trapTab}>
  <div class="top">
    <span class="kind">{target.kind === 'event' ? (target.id == null ? t('common.newEvent') : t('common.event')) : target.id == null ? t('common.newTask') : t('common.task')}</span>
    <button class="icon-btn" aria-label={t('common.close')} onclick={close}><X size={16} /></button>
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
    {#if isSubtask && parentTask}
      <button class="btn link part-of" onclick={() => parentTask && openOther(parentTask.id)}>
        <CornerLeftUp size={13} aria-hidden="true" />{t('subtasks.partOf', { title: parentTask.title })}
      </button>
    {/if}
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
    <label class="row"><span>{t('priority.title')}</span>
      <select class="field" bind:value={task.priority}>
        {#each PRIORITIES as p}<option value={p}>{t(`priority.${p}`)}</option>{/each}
      </select>
    </label>
    {#if plans.length && parentPick == null}
      <label class="row"><span>{t('details.plan')}</span>
        <select class="field" value={task.planId ?? ''} onchange={(e) => pickPlan(Number(e.currentTarget.value))}>
          {#if task.planId == null}<option value="" disabled></option>{/if}
          {#each plans as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
        </select>
      </label>
      <label class="row"><span>{t('details.bucket')}</span>
        <select class="field" bind:value={task.bucketId} disabled={!planBuckets.length}>
          {#each planBuckets as b (b.id)}<option value={b.id}>{b.name}</option>{/each}
        </select>
      </label>
    {/if}
    <label class="row"><span>{t('inspector.calendar')}</span>
      <select class="field" bind:value={task.calendarId}>{#each calendars as c}<option value={c.id}>{c.name}</option>{/each}</select>
    </label>
    <label class="row"><span>{t('inspector.due')}</span><input class="field" type="date" bind:value={task.dueDate} /></label>
    <label class="row"><span>{t('inspector.time')}</span>
      <input class="field" type="time" bind:value={task.dueTime} disabled={!task.dueDate} />
    </label>
    {#if !task.dueDate}<button class="btn link" onclick={() => task && (task.dueDate = ymd(new Date()))}>{t('inspector.dueToday')}</button>{/if}
    <div class="row"><label for="start-date">{t('start.title')}</label>
      <div class="pair">
        <input id="start-date" class="field" type="date" bind:value={task.startDate} max={task.dueDate || undefined}
          aria-invalid={startInvalid} aria-describedby={startInvalid ? 'start-error' : undefined} />
        {#if task.dueDate && task.dueTime}
          <input class="field" type="time" bind:value={task.startTime} disabled={!task.startDate} aria-label={t('start.time')} />
        {/if}
      </div>
    </div>
    {#if startInvalid}<p class="inline-error" id="start-error">{t('error.start_after_due')}</p>{/if}
    {#if canHaveParent && stored && (candidates.length || parentPick != null)}
      <label class="row"><span>{t('subtasks.parent')}</span>
        <select class="field" value={parentPick ?? ''} onchange={(e) => (parentPick = e.currentTarget.value ? Number(e.currentTarget.value) : null)}>
          <option value="">{t('subtasks.none')}</option>
          {#each candidates as c (c.id)}<option value={c.id}>{c.title}</option>{/each}
        </select>
      </label>
    {/if}
    <textarea class="field notes" bind:value={task.notes} placeholder={t('inspector.notes')}></textarea>
    {#if stored}
      {#if !isSubtask}
        <SubtaskList bind:this={subtaskList} parent={stored} {subtasks} onopen={(x) => openOther(x.id)} onerror={(m) => (error = m)} />
      {/if}
      <Checklist bind:this={checklist} task={stored} canConvert={!isSubtask} onerror={(m) => (error = m)} />
    {/if}
  {/if}

  <div class="actions">
    {#if target.id != null}
      <button class="btn danger" onclick={remove}><Trash2 size={13} /> {confirmDelete ? t('inspector.confirmDelete') : t('common.delete')}</button>
      {#if confirmDelete && subtasks.length}<span class="warn">{t('delete.withSubtasks', { n: subtasks.length })}</span>{/if}
    {/if}
    <span class="spacer"></span>
    <button class="btn" onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={save} disabled={startInvalid}>{t('common.save')}</button>
  </div>
</div>
</div>

<style>
  /* Item details open as a centred dialog over the main window. */
  .scrim { position: fixed; inset: 0; z-index: 25; display: grid; place-items: center; padding: 24px; background: rgba(0, 0, 0, 0.32); }
  .panel {
    width: min(480px, 100%);
    max-height: min(760px, 100%);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    padding: 14px 18px 16px;
    display: flex;
    flex-direction: column;
    gap: 9px;
    overflow-y: auto;
  }
  .panel:focus { outline: none; }
  @media (prefers-reduced-motion: no-preference) {
    .scrim { animation: fade 0.14s ease-out; }
    .panel { animation: rise 0.18s cubic-bezier(0.2, 0.8, 0.2, 1); }
    @keyframes fade { from { opacity: 0; } }
    @keyframes rise { from { opacity: 0; transform: translateY(6px) scale(0.99); } }
  }
  .top { display: flex; justify-content: space-between; align-items: center; }
  .kind { font-size: 12px; font-weight: 600; color: var(--muted); }
  .title { font-size: 15px; font-weight: 600; }
  .row { display: grid; grid-template-columns: 88px 1fr; align-items: center; gap: 10px; }
  .row > span, .row > label { color: var(--muted); font-size: 12px; }
  .pair { display: flex; gap: 6px; }
  .pair .field { flex: 1; min-width: 0; }
  .inline-error { margin: -4px 0 0 98px; font-size: 12px; color: var(--danger); }
  .warn { font-size: 12px; color: var(--danger); }
  .link.part-of { display: inline-flex; align-items: center; gap: 5px; margin-left: 0; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .check { display: flex; align-items: center; gap: 6px; padding-left: 98px; }
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
  .link { align-self: flex-start; margin-left: 98px; }
</style>
