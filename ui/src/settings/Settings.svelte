<script lang="ts">
  import { hotkeyLabel, isMac } from '../lib/platform';
  import { t } from '../lib/i18n';
  import { onMount } from 'svelte';
  import { save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { Plus, Trash2, X } from 'lucide-svelte';
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
      flash(t('settings.saved'));
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
  const pretty = (hk: string) => hotkeyLabel(hk);

  // Keys go to the shortcut button while it records, so Esc cancels the
  // recording instead of closing the Settings dialog around it.
  function record(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') return void (recording = false);
    if (['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) return;
    const mods = [e.ctrlKey && 'Control', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
    if (!mods.length) return void (error = t('settings.needModifier', { keys: isMac ? '⌘ ⌥ ⌃ ⇧' : 'Ctrl, Alt, Shift, Win' }));
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
      flash(t('settings.exported'));
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


<div class="settings">
  {#if error}<div class="banner" role="alert"><span>{error}</span><button class="icon-btn" aria-label={t('common.dismiss')} onclick={() => (error = '')}><X size={14} aria-hidden="true" /></button></div>{/if}
  <div role="status" aria-live="polite">{#if notice}<div class="notice">{notice}</div>{/if}</div>

  {#if settings}
    <section>
      <h2>{t('settings.general')}</h2>
      <div class="group">
      <div class="row">
        <span>{t('settings.shortcut')}</span>
        <button class="btn hotkey" class:rec={recording} onclick={() => (recording = !recording)} onkeydown={record} onblur={() => (recording = false)}>
          {recording ? t('settings.recording') : pretty(settings.hotkey)}
        </button>
      </div>
      <label class="row">
        <span>{t('settings.login')}</span>
        <input type="checkbox" checked={settings.launchAtLogin} onchange={(e) => update({ launchAtLogin: e.currentTarget.checked })} />
      </label>
      <label class="row">
        <span>{t('settings.defaultCalendar')}</span>
        <select class="field" value={settings.defaultCalendarId} onchange={(e) => update({ defaultCalendarId: Number(e.currentTarget.value) })}>
          {#each calendars as c}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </label>
      <label class="row">
        <span>{t('settings.eventLength')}</span>
        <select class="field" value={settings.defaultEventMinutes} onchange={(e) => update({ defaultEventMinutes: Number(e.currentTarget.value) })}>
          {#each [15, 30, 45, 60, 90, 120] as m}<option value={m}>{m < 60 ? `${m} min` : `${m / 60} h`}</option>{/each}
        </select>
      </label>
      <label class="row">
        <span>{t('settings.weekStart')}</span>
        <select class="field" value={settings.weekStart} onchange={(e) => update({ weekStart: e.currentTarget.value as Settings['weekStart'] })}>
          <option value="Mon">{t('settings.monday')}</option><option value="Sun">{t('settings.sunday')}</option>
        </select>
      </label>
      <label class="row">
        <span>{t('settings.numericDates')}</span>
        <select class="field" value={settings.dateOrder} onchange={(e) => update({ dateOrder: e.currentTarget.value as Settings['dateOrder'] })}>
          <option value="dayMonth">{t('settings.dayMonth')}</option><option value="monthDay">{t('settings.monthDay')}</option>
        </select>
      </label>
      </div>
    </section>

    <section>
      <h2>{t('settings.calendars')}</h2>
      <div class="group">
      {#each calendars as c (c.id)}
        <div class="cal">
          <input type="color" aria-label={t('settings.calendarColor', { name: c.name })} value={c.color} onchange={(e) => saveCalendar(c, { color: e.currentTarget.value })} />
          <input class="field" aria-label={t('settings.calendarName')} value={c.name} onchange={(e) => saveCalendar(c, { name: e.currentTarget.value })} />
          <button class="icon-btn" title={t('settings.deleteCalendar')} aria-label={t('settings.deleteCalendarNamed', { name: c.name })} disabled={calendars.length < 2}
            onclick={() => (deleting = { cal: c, moveTo: calendars.find((x) => x.id !== c.id)!.id })}><Trash2 size={14} /></button>
        </div>
      {/each}
      <div class="cal">
        <span class="swatch"></span>
        <input class="field" placeholder={t('settings.newCalendar')} aria-label={t('settings.newCalendarName')} bind:value={newName} onkeydown={(e) => e.key === 'Enter' && addCalendar()} />
        <button class="icon-btn" title={t('settings.addCalendar')} aria-label={t('settings.addCalendar')} onclick={addCalendar}><Plus size={15} /></button>
      </div>
      {#if deleting}
        <div class="confirm">
          <span>{t('settings.deleteMove', { name: deleting.cal.name })}</span>
          <select class="field small" bind:value={deleting.moveTo}>
            {#each calendars.filter((c) => c.id !== deleting?.cal.id) as c}<option value={c.id}>{c.name}</option>{/each}
          </select>
          <button class="btn" onclick={() => (deleting = null)}>{t('common.cancel')}</button>
          <button class="btn danger" onclick={confirmDelete}>{t('common.delete')}</button>
        </div>
      {/if}
      </div>
    </section>

    <section>
      <h2>{t('settings.data')}</h2>
      <div class="group">
      <div class="row">
        <span>{t('settings.export')}<br /><span class="hint">{t('settings.exportHint')}</span></span>
        <button class="btn" onclick={exportIcs}>{t('settings.exportButton')}</button>
      </div>
      </div>
    </section>
  {/if}
</div>

<style>
  /* Grouped rows, like macOS settings; shown inside the Settings dialog. */
  .settings { position: relative; display: grid; align-content: start; gap: 18px; }
  h2 { margin: 0 0 6px 4px; font-size: 13px; font-weight: 600; color: var(--muted); }
  section { display: grid; }
  .group { background: var(--bg-sunken); border: 1px solid var(--border); border-radius: 10px; padding: 2px 12px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 40px; padding: 6px 0; }
  .group > :not(:last-child) { border-bottom: 1px solid var(--border); }
  .row > :last-child { flex: 0 1 240px; }
  .row input[type='checkbox'] { flex: none; }
  .hotkey { font-size: 14px; letter-spacing: 0.08em; }
  .hotkey.rec { border-color: var(--accent); color: var(--accent); letter-spacing: normal; font-size: 12px; }
  .cal { display: grid; grid-template-columns: 28px 1fr 28px; gap: 8px; align-items: center; padding: 6px 0; }
  /* Round swatches, like calendar colours everywhere else in the app. */
  .cal input[type='color'] { width: 20px; height: 20px; border: none; padding: 0; background: none; border-radius: 50%; overflow: hidden; justify-self: center; }
  .cal input[type='color']::-webkit-color-swatch-wrapper { padding: 0; }
  .cal input[type='color']::-webkit-color-swatch { border: none; border-radius: 50%; }
  .swatch { width: 18px; height: 18px; border-radius: 50%; border: 1px dashed var(--muted); justify-self: center; }
  .confirm { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 8px 0; }
  .small { width: auto; }
  .hint { color: var(--muted); font-size: 12px; }
  .notice { position: absolute; top: 0; right: 4px; color: var(--label-time); font-size: 12px; }
</style>
