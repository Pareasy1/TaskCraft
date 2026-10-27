<script lang="ts">
  import { modLabel } from '../lib/platform';
  import { t } from '../lib/i18n';
  import { ChevronLeft, ChevronRight, Search, Plus } from 'lucide-svelte';

  import type { Area, CalendarMode } from '../lib/navigation';
  import type { Zoom } from '../lib/timeline';
  let {
    area = $bindable<Area>('calendar'),
    mode = $bindable<CalendarMode>('week'),
    zoom = $bindable<Zoom>('weeks'),
    title,
    query = $bindable(''),
    onprev,
    onnext,
    ontoday,
    onnew,
  }: {
    area?: Area;
    mode?: CalendarMode;
    /** The Planner's zoom (shown only in Planner). */
    zoom?: Zoom;
    title: string;
    query?: string;
    onprev: () => void;
    onnext: () => void;
    ontoday: () => void;
    onnew: () => void;
  } = $props();
</script>

<header data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    <h1 data-tauri-drag-region>{title}</h1>
  </div>
  <div class="right" data-tauri-drag-region>
    <button class="icon-btn" title={area !== 'calendar' ? t('toolbar.newTaskKey', { key: modLabel('N') }) : t('toolbar.newEventKey', { key: modLabel('N') })} aria-label={area !== 'calendar' ? t('common.newTask') : t('common.newEvent')} onclick={onnew}><Plus size={17} /></button>
    {#if area === 'calendar'}
      <div class="nav">
        <button class="icon-btn" aria-label={t('common.previous')} onclick={onprev}><ChevronLeft size={17} /></button>
        <button class="btn today" onclick={ontoday}>{t('toolbar.today')}</button>
        <button class="icon-btn" aria-label={t('common.next')} onclick={onnext}><ChevronRight size={17} /></button>
      </div>
    {/if}
    {#if area === 'calendar'}
      <div class="seg" role="group" aria-label={t('toolbar.views')}>
        {#each [['day', t('toolbar.day')], ['week', t('toolbar.week')], ['month', t('toolbar.month')]] as [m, label]}
          <button aria-pressed={mode === m} class:on={mode === m} title="{label} ({m[0].toUpperCase()})" onclick={() => (mode = m as CalendarMode)}>{label}</button>
        {/each}
      </div>
    {/if}
    {#if area === 'planner'}
      <button class="btn today" onclick={ontoday}>{t('planner.today')}</button>
      <div class="seg" role="group" aria-label={t('planner.zoom')}>
        {#each [['days', t('planner.days')], ['weeks', t('planner.weeks')], ['months', t('planner.months')]] as [z, label]}
          <button aria-pressed={zoom === z} class:on={zoom === z} onclick={() => (zoom = z as Zoom)}>{label}</button>
        {/each}
      </div>
    {/if}
    <div class="seg" role="group" aria-label={t('toolbar.areas')}>
      {#each [['calendar', t('area.calendar'), modLabel('1')], ['tasks', t('area.tasks'), modLabel('2')], ['planner', t('area.planner'), modLabel('4')], ['grid', t('area.grid'), modLabel('3')]] as [a, label, key]}
        <button aria-pressed={area === a} class:on={area === a} title="{label} ({key})" onclick={() => (area = a as Area)}>{label}</button>
      {/each}
    </div>
    <label class="search">
      <Search size={13} />
      <input placeholder={t('toolbar.search')} aria-label={t('toolbar.searchLabel')} bind:value={query} />
    </label>
  </div>
</header>

<style>
  header {
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px 0 16px;
    border-bottom: 1px solid var(--border);
    gap: 12px;
  }
  .left { display: flex; align-items: center; min-width: 0; flex: 1; height: 100%; }
  h1 { font-size: 19px; font-weight: 700; margin: 0; white-space: nowrap; letter-spacing: -0.01em; }
  .right { display: flex; align-items: center; gap: 10px; }
  .nav { display: flex; align-items: center; gap: 2px; }
  .today { padding: 3px 10px; }
  .seg {
    display: flex;
    background: var(--bg-sunken);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 2px;
  }
  .seg button { border: none; background: none; padding: 3px 11px; border-radius: 6px; color: var(--muted); }
  .seg button.on { background: var(--surface-strong); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .search {
    display: flex;
    align-items: center;
    gap: 5px;
    border: 1px solid var(--border);
    background: var(--bg-sunken);
    border-radius: 7px;
    padding: 3px 8px;
    color: var(--muted);
    width: 180px;
  }
  .search input { border: none; outline: none; background: transparent; width: 100%; color: var(--text); }
</style>
