<script lang="ts">
  import { onMount } from 'svelte';
  import { save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { Plus, Trash2 } from 'lucide-svelte';
  import { api, errorMessage, onItemsChanged } from '../lib/api';
  import type { Calendar, Settings } from '../lib/types';

  let settings = $state<Settings | null>(null);
  let calendars = $state<Calendar[]>([]);
  let error = $state('');
  let notice = $state('');
  let recording = $state(false);
  let deleting = $state<{ cal: Calendar; moveTo: number } | null>(null);
  let newName = $state('');

  async function load() {
    try {
      [settings, calendars] = await Promise.all([api.getSettings(), api.listCalendars()]);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function update(patch: Partial<Settings>) {
    if (!settings) return;
    try {
      settings = await api.setSettings({ ...settings, ...patch });
      error = '';
      flash('Saved');
    } catch (e) {
      error = errorMessage(e);
      await load();
    }
  }

  function flash(msg: string) {
    notice = msg;
    setTimeout(() => (notice = ''), 1500);
  }

  // ---- hotkey recorder ----
  const SYMBOL: Record<string, string> = { Control: '⌃', Alt: '⌥', Shift: '⇧', Super: '⌘' };
  const pretty = (hk: string) =>
    hk.split('+').map((p) => SYMBOL[p] ?? (p.startsWith('Key') ? p.slice(3) : p.startsWith('Digit') ? p.slice(5) : p)).join('');

  function record(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    if (e.key === 'Escape') return void (recording = false);
    if (['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) return;
    const mods = [e.ctrlKey && 'Control', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
    if (!mods.length) return void (error = 'Use at least one modifier key (⌘, ⌥, ⌃ or ⇧)');
    const key = e.code.startsWith('Key') ? e.code.slice(3) : e.code.startsWith('Digit') ? e.code.slice(5) : e.code;
    recording = false;
    update({ hotkey: [...mods, key].join('+') });
  }

  // ---- calendars ----
  async function saveCalendar(c: Calendar, patch: Partial<Calendar>) {
    try {
      await api.upsertCalendar({ id: c.id, name: patch.name ?? c.name, color: patch.color ?? c.color, visible: c.visible });
    } catch (e) {
      error = errorMessage(e);
      await load();
    }
  }

  async function addCalendar() {
    const name = newName.trim();
    if (!name) return;
    const palette = ['#22c55e', '#a855f7', '#ec4899', '#eab308', '#14b8a6', '#ef4444'];
    try {
      await api.upsertCalendar({ id: null, name, color: palette[calendars.length % palette.length], visible: true });
      newName = '';
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      if (settings?.defaultCalendarId === deleting.cal.id) await update({ defaultCalendarId: deleting.moveTo });
      await api.deleteCalendar(deleting.cal.id, deleting.moveTo);
      deleting = null;
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function exportIcs() {
    try {
      const path = await saveDialog({ defaultPath: 'TaskCraft.ics', filters: [{ name: 'iCalendar', extensions: ['ics'] }] });
      if (!path) return;
      await api.exportIcs(path);
      flash('Exported');
    } catch (e) {
      error = errorMessage(e);
    }
  }

  onMount(() => {
    load();
    const un = onItemsChanged(load);
    return () => un.then((f) => f());
  });
</script>

<svelte:window onkeydown={record} />

<main>
  <h1>Settings</h1>
  {#if error}<div class="banner"><span>{error}</span><button class="icon-btn" onclick={() => (error = '')}>✕</button></div>{/if}
  {#if notice}<div class="notice">{notice}</div>{/if}

  {#if settings}
    <section>
      <h2>General</h2>
      <div class="row">
        <span>Quick entry shortcut</span>
        <button class="btn hotkey" class:rec={recording} onclick={() => (recording = !recording)}>
          {recording ? 'Press keys… (esc to cancel)' : pretty(settings.hotkey)}
        </button>
      </div>
      <label class="row">
        <span>Launch at login</span>
        <input type="checkbox" checked={settings.launchAtLogin} onchange={(e) => update({ launchAtLogin: e.currentTarget.checked })} />
      </label>
      <label class="row">
        <span>Default calendar</span>
        <select class="field" value={settings.defaultCalendarId} onchange={(e) => update({ defaultCalendarId: Number(e.currentTarget.value) })}>
          {#each calendars as c}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </label>
      <label class="row">
        <span>Default event length</span>
        <select class="field" value={settings.defaultEventMinutes} onchange={(e) => update({ defaultEventMinutes: Number(e.currentTarget.value) })}>
          {#each [15, 30, 45, 60, 90, 120] as m}<option value={m}>{m < 60 ? `${m} min` : `${m / 60} h`}</option>{/each}
        </select>
      </label>
      <label class="row">
        <span>Week starts on</span>
        <select class="field" value={settings.weekStart} onchange={(e) => update({ weekStart: e.currentTarget.value as Settings['weekStart'] })}>
          <option value="Mon">Monday</option><option value="Sun">Sunday</option>
        </select>
      </label>
      <label class="row">
        <span>Numeric dates</span>
        <select class="field" value={settings.dateOrder} onchange={(e) => update({ dateOrder: e.currentTarget.value as Settings['dateOrder'] })}>
          <option value="dayMonth">Day/Month (7/11 = 7 Nov)</option><option value="monthDay">Month/Day (7/11 = Jul 11)</option>
        </select>
      </label>
    </section>

    <section>
      <h2>Calendars</h2>
      {#each calendars as c (c.id)}
        <div class="cal">
          <input type="color" value={c.color} onchange={(e) => saveCalendar(c, { color: e.currentTarget.value })} />
          <input class="field" value={c.name} onchange={(e) => saveCalendar(c, { name: e.currentTarget.value })} />
          <button class="icon-btn" title="Delete calendar" disabled={calendars.length < 2}
            onclick={() => (deleting = { cal: c, moveTo: calendars.find((x) => x.id !== c.id)!.id })}><Trash2 size={14} /></button>
        </div>
      {/each}
      <div class="cal">
        <span class="swatch"></span>
        <input class="field" placeholder="New calendar" bind:value={newName} onkeydown={(e) => e.key === 'Enter' && addCalendar()} />
        <button class="icon-btn" title="Add calendar" onclick={addCalendar}><Plus size={15} /></button>
      </div>
      {#if deleting}
        <div class="confirm">
          <span>Delete “{deleting.cal.name}” and move its items to</span>
          <select class="field small" bind:value={deleting.moveTo}>
            {#each calendars.filter((c) => c.id !== deleting?.cal.id) as c}<option value={c.id}>{c.name}</option>{/each}
          </select>
          <button class="btn" onclick={() => (deleting = null)}>Cancel</button>
          <button class="btn danger" onclick={confirmDelete}>Delete</button>
        </div>
      {/if}
    </section>

    <section>
      <h2>Data</h2>
      <div class="row">
        <span>Export everything as an .ics file</span>
        <button class="btn" onclick={exportIcs}>Export .ics…</button>
      </div>
    </section>
  {/if}
</main>

<style>
  main { padding: 18px 22px; display: grid; gap: 14px; height: 100%; overflow-y: auto; }
  h1 { margin: 0; font-size: 18px; }
  h2 { margin: 0 0 6px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.03em; color: var(--muted); }
  section { display: grid; gap: 8px; }
  .row { display: grid; grid-template-columns: 1fr 230px; align-items: center; gap: 10px; }
  .row input[type='checkbox'] { justify-self: start; }
  .hotkey { font-size: 14px; letter-spacing: 0.08em; }
  .hotkey.rec { border-color: var(--accent); color: var(--accent); letter-spacing: normal; font-size: 12px; }
  .cal { display: grid; grid-template-columns: 28px 1fr 28px; gap: 8px; align-items: center; }
  .cal input[type='color'] { width: 24px; height: 24px; border: none; padding: 0; background: none; }
  .swatch { width: 18px; height: 18px; border-radius: 50%; border: 1px dashed var(--faint); justify-self: center; }
  .confirm { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; background: var(--bg-sunken); border-radius: 8px; padding: 8px; }
  .small { width: auto; }
  .notice { position: fixed; top: 14px; right: 18px; color: var(--label-time); font-size: 12px; }
</style>
