<script lang="ts">
  import { t } from '../../lib/i18n';
  import { Trash2 } from 'lucide-svelte';
  import type { TaskLabel } from '../../lib/types';
  import { api, errorMessage } from '../../lib/api';

  let { labels, onclose, onerror }: { labels: TaskLabel[]; onclose: () => void; onerror: (m: string) => void } = $props();

  const PALETTE = ['#3b82f6', '#f97316', '#10b981', '#a855f7', '#ef4444', '#eab308', '#06b6d4', '#ec4899'];
  let picking = $state<number | null>(null);

  async function save(l: TaskLabel, name: string, color: string) {
    if (name.trim() === l.name && color === l.color) return;
    try {
      await api.updateLabel(l.id, name, color);
    } catch (e) {
      onerror(errorMessage(e));
    }
  }

  async function remove(l: TaskLabel) {
    try {
      await api.deleteLabel(l.id);
    } catch (e) {
      onerror(errorMessage(e));
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="pop" role="dialog" aria-label={t('labels.title')}>
  <p class="intro">{t('labels.intro')}</p>
  {#each labels as l (l.id)}
    <div class="item">
      <button class="sw" style="background:{l.color}" aria-label={t('labels.changeColor', { name: l.name })} onclick={() => (picking = picking === l.id ? null : l.id)}></button>
      <input
        value={l.name}
        aria-label={t('labels.name')}
        maxlength="32"
        onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
        onblur={(e) => save(l, (e.currentTarget as HTMLInputElement).value, l.color)}
      />
      <button class="icon-btn" aria-label={t('labels.delete', { name: l.name })} title={t('labels.deleteTitle')} onclick={() => remove(l)}><Trash2 size={13} /></button>
    </div>
    {#if picking === l.id}
      <div class="palette">
        {#each PALETTE as c}
          <button class="sw" class:on={c === l.color} style="background:{c}" aria-label={t('labels.useColor', { color: c })} onclick={() => { picking = null; save(l, l.name, c); }}></button>
        {/each}
      </div>
    {/if}
  {:else}
    <p class="empty">{t('labels.empty')}</p>
  {/each}
</div>

<style>
  .pop {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 25;
    width: 270px;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 10px;
  }
  .intro { margin: 0 0 8px; font-size: 12px; color: var(--muted); line-height: 1.4; }
  .item { display: flex; align-items: center; gap: 8px; padding: 2px 0; }
  .item input { flex: 1; min-width: 0; border: 1px solid transparent; background: none; border-radius: 6px; padding: 4px 6px; }
  .item input:hover { border-color: var(--border); }
  .item input:focus { border-color: var(--accent); outline: none; background: var(--bg); }
  .sw { width: 14px; height: 14px; border-radius: 50%; border: none; flex: none; padding: 0; }
  .sw:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .sw.on { box-shadow: 0 0 0 2px var(--surface-strong), 0 0 0 3.5px var(--text); }
  .palette { display: flex; gap: 8px; padding: 6px 0 8px 22px; }
  .empty { color: var(--faint); margin: 4px 0; }
</style>
