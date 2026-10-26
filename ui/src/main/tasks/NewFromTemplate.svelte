<script lang="ts">
  // "New from template ▸": a button whose menu lists the templates; picking one hands it to `onpick`.
  import { ChevronDown, LayoutTemplate } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage } from '../../lib/api';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { templateMenuItems, type TemplateMenuItem } from '../../lib/templates';
  import Menu from '../Menu.svelte';

  let {
    onpick,
    onerror,
    compact = false,
    context = '',
  }: {
    onpick: (template: { id: number; name: string }) => void;
    onerror: (msg: string) => void;
    /** Icon only (bucket footers). */
    compact?: boolean;
    /** Where the task goes (a bucket's name), added to the button's spoken label. */
    context?: string;
  } = $props();

  const menu = new PopupMenu<null>();
  let items = $state<TemplateMenuItem[]>([]);
  const label = $derived(context ? `${t('template.newFrom')} — ${context}` : t('template.newFrom'));

  /** True while the templates load for the menu; a click meanwhile is ignored (it would close it again). */
  let opening = $state(false);
  async function toggle(button: HTMLElement) {
    if (opening) return;
    if (menu.current) return menu.close();
    opening = true;
    try {
      // Fresh each time: templates change from other places (Settings, other windows).
      items = templateMenuItems(await api.listTemplates());
      menu.openBelow(null, button, () => true);
    } catch (e) {
      onerror(errorMessage(e));
    } finally {
      opening = false;
    }
  }
</script>

<button class={compact ? 'icon-btn tpl compact' : 'tpl'} aria-label={label} title={t('template.newFrom')}
  aria-haspopup="menu" aria-expanded={!!menu.current || opening} class:open={!!menu.current || opening}
  onclick={(e) => toggle(e.currentTarget)}>
  <LayoutTemplate size={compact ? 13 : 14} aria-hidden="true" />
  {#if !compact}<span>{t('template.newFrom')}</span><ChevronDown size={13} aria-hidden="true" />{/if}
</button>

{#if menu.current}
  <Menu {menu} label={t('template.newFrom')}>
    {#each items as item (item.kind === 'template' ? item.id : 'empty')}
      {#if item.kind === 'template'}
        <button role="menuitem" class="item" onclick={() => menu.act(() => onpick({ id: item.id, name: item.name }), false)}>
          <span class="name">{item.name}</span>{#if item.detail}<span class="detail">{item.detail}</span>{/if}
        </button>
      {:else}
        <!-- Focusable so the keyboard lands in the menu (and Esc closes it); it does nothing. -->
        <button role="menuitem" class="none" aria-disabled="true" onclick={() => menu.close()}>{item.label}</button>
      {/if}
    {/each}
  </Menu>
{/if}

<style>
  .tpl:not(.compact) {
    display: flex;
    align-items: center;
    gap: 5px;
    border: 1px solid var(--border);
    background: var(--bg-sunken);
    border-radius: 8px;
    padding: 4px 8px 4px 9px;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .tpl:not(.compact):hover, .tpl.open { background: var(--hover); color: var(--text); }
  .tpl:focus-visible { outline: 2px solid var(--accent); }
  .compact { padding: 4px; flex: none; }
  /* Doubled class: outranks the shared menu's button styles. */
  .item.item { display: flex; align-items: baseline; gap: 12px; max-width: 320px; }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .detail { flex: none; font-size: 11.5px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .none.none { max-width: 260px; color: var(--muted); white-space: normal; line-height: 1.4; cursor: default; }
</style>
