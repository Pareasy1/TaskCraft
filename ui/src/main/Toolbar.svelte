<script lang="ts">
  import { t } from '../lib/i18n';
  import { ChevronLeft, ChevronRight, Search, Plus } from 'lucide-svelte';

  type View = 'day' | 'week' | 'month' | 'tasks';
  let {
    view = $bindable<View>('week'),
    title,
    query = $bindable(''),
    onprev,
    onnext,
    ontoday,
    onnew,
  }: {
    view?: View;
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
  <div class="right">
    <button class="icon-btn" title={view === 'tasks' ? t('toolbar.newTaskKey') : t('toolbar.newEventKey')} aria-label={view === 'tasks' ? t('common.newTask') : t('common.newEvent')} onclick={onnew}><Plus size={17} /></button>
    {#if view !== 'tasks'}
      <div class="nav">
        <button class="icon-btn" aria-label={t('common.previous')} onclick={onprev}><ChevronLeft size={17} /></button>
        <button class="btn today" onclick={ontoday}>{t('toolbar.today')}</button>
        <button class="icon-btn" aria-label={t('common.next')} onclick={onnext}><ChevronRight size={17} /></button>
      </div>
    {/if}
    <div class="seg" role="tablist">
      {#each [['day', t('toolbar.day'), '⌘1'], ['week', t('toolbar.week'), '⌘2'], ['month', t('toolbar.month'), '⌘3'], ['tasks', t('toolbar.tasks'), '⌘4']] as [v, label, key]}
        <button role="tab" class:on={view === v} title="{label} ({key})" onclick={() => (view = v as View)}>{label}</button>
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
