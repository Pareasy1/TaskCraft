<script lang="ts">
  import { t } from '../../lib/i18n';
  import { tick } from 'svelte';
  import type { TaskStatus } from '../../lib/types';
  import { nextStatus, STATUSES, STATUS_LABEL } from '../../lib/tasks';

  let {
    status,
    size = 18,
    onchange,
  }: { status: TaskStatus; size?: number; onchange?: (s: TaskStatus) => void } = $props();

  let menu = $state(false);
  let trigger: HTMLButtonElement | undefined = $state();
  let menuEl: HTMLDivElement | undefined = $state();

  const items = () => [...(menuEl?.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]') ?? [])];
  async function openMenu() {
    menu = true;
    await tick();
    (items().find((b) => b.getAttribute('aria-checked') === 'true') ?? items()[0])?.focus();
  }
  function closeMenu(refocus = true) {
    menu = false;
    if (refocus) trigger?.focus();
  }
  const pick = (s: TaskStatus) => {
    closeMenu();
    if (s !== status) onchange?.(s);
  };
  function triggerKeys(e: KeyboardEvent) {
    // Shift+F10 and the menu key open the chooser, like a right-click.
    if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      e.stopPropagation();
      openMenu();
    }
  }
  function menuKeys(e: KeyboardEvent) {
    e.stopPropagation(); // the row's own arrow-key moves must not fire inside the menu
    const list = items();
    const i = list.indexOf(document.activeElement as HTMLButtonElement);
    const go = (n: number) => list[(n + list.length) % list.length]?.focus();
    if (e.key === 'ArrowDown') go(i + 1);
    else if (e.key === 'ArrowUp') go(i - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(list.length - 1);
    else if (e.key === 'Escape') closeMenu();
    else if (e.key === 'Tab') return closeMenu(false);
    else return;
    e.preventDefault();
  }
  // Close when clicking anywhere else.
  $effect(() => {
    if (!menu) return;
    const away = (e: PointerEvent) => !menuEl?.contains(e.target as Node) && !trigger?.contains(e.target as Node) && closeMenu(false);
    window.addEventListener('pointerdown', away, true);
    return () => window.removeEventListener('pointerdown', away, true);
  });
</script>

{#snippet glyph(s: TaskStatus, px: number)}
  <svg class="g {s}" width={px} height={px} viewBox="0 0 20 20" aria-hidden="true">
    <circle class="ring" cx="10" cy="10" r="8.25" />
    <path class="half" d="M10 1.75 A8.25 8.25 0 0 1 10 18.25 Z" />
    <circle class="fill" cx="10" cy="10" r="8.25" />
    <path class="tick" d="M5.7 10.5 C6.6 11.2 7.5 12.2 8.5 13.4 C9.9 10.9 11.8 8.7 14.4 6.6" />
  </svg>
{/snippet}

{#if onchange}
  <span class="wrap">
    <button
      bind:this={trigger}
      aria-haspopup="menu"
      aria-expanded={menu}
      onkeydown={triggerKeys}
      class="btn-glyph"
      title={t('status.title', { status: STATUS_LABEL[status], next: STATUS_LABEL[nextStatus(status)] })}
      aria-label={t('status.aria', { status: STATUS_LABEL[status], next: STATUS_LABEL[nextStatus(status)] })}
      onclick={(e) => { e.stopPropagation(); onchange(nextStatus(status)); }}
      oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); if (menu) closeMenu(); else openMenu(); }}
    >{@render glyph(status, size)}</button>
    {#if menu}
      <div class="menu" role="menu" tabindex="-1" aria-label={t('inspector.status')} bind:this={menuEl} onkeydown={menuKeys}>
        {#each STATUSES as s}
          <button role="menuitemradio" aria-checked={s === status} onclick={(e) => { e.stopPropagation(); pick(s); }}>
            {@render glyph(s, 14)}<span>{STATUS_LABEL[s]}</span>
          </button>
        {/each}
      </div>
    {/if}
  </span>
{:else}
  {@render glyph(status, size)}
{/if}

<style>
  .wrap { position: relative; display: inline-flex; }
  .btn-glyph { border: none; background: none; padding: 2px; margin: -2px; border-radius: 50%; display: inline-flex; }
  .btn-glyph:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .g { flex: none; display: block; }
  .ring { fill: none; stroke: var(--faint); stroke-width: 1.5; }
  .half, .fill { fill: var(--st, transparent); }
  .half { opacity: 0; }
  .fill { opacity: 0; transform-origin: 10px 10px; transform: scale(0.4); }
  .tick { fill: none; stroke: var(--bg); stroke-width: 2.1; stroke-linecap: round; stroke-linejoin: round; stroke-dasharray: 15; stroke-dashoffset: 15; }
  .inProgress { --st: var(--status-progress); }
  .inProgress .ring { stroke: var(--status-progress); }
  .inProgress .half { opacity: 1; }
  .done { --st: var(--status-done); }
  .done .ring { stroke: var(--status-done); }
  .done .fill { opacity: 1; transform: scale(1); }
  .done .tick { stroke-dashoffset: 0; }
  @media (prefers-reduced-motion: no-preference) {
    .half, .fill, .ring { transition: opacity 0.18s ease, transform 0.22s cubic-bezier(0.3, 1.4, 0.5, 1), stroke 0.18s; }
    .tick { transition: stroke-dashoffset 0.25s ease 0.12s; }
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: -4px;
    z-index: 30;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 4px;
    min-width: 140px;
  }
  .menu button { display: flex; align-items: center; gap: 8px; width: 100%; border: none; background: none; padding: 5px 8px; border-radius: 5px; text-align: left; }
  .menu button:hover, .menu button:focus-visible { background: var(--hover); outline: none; }
  .menu button[aria-checked='true'] span { font-weight: 600; }
</style>
