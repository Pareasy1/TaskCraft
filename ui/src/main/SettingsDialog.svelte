<script lang="ts">
  import { X } from 'lucide-svelte';
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import Settings from '../settings/Settings.svelte';

  let { onclose }: { onclose: () => void } = $props();

  // Focus moves into the dialog when it opens and back to where it came from when it closes.
  const opener = document.activeElement as HTMLElement | null;
  let dialogEl = $state<HTMLDivElement | undefined>();
  onMount(() => dialogEl?.querySelector<HTMLElement>('.close')?.focus());
  onDestroy(() => {
    if (opener?.isConnected) opener.focus();
  });

  const focusables = () =>
    [...(dialogEl?.querySelectorAll<HTMLElement>('button, input, select, textarea, [tabindex]:not([tabindex="-1"])') ?? [])].filter(
      (el) => !(el as HTMLButtonElement).disabled && el.offsetParent !== null,
    );

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    } else if (e.key === 'Tab') {
      // Keep Tab inside the dialog.
      const list = focusables();
      if (!list.length) return;
      const i = list.indexOf(document.activeElement as HTMLElement);
      const next = e.shiftKey ? (i <= 0 ? list.length - 1 : i - 1) : i === list.length - 1 ? 0 : i + 1;
      e.preventDefault();
      list[next].focus();
    }
    e.stopPropagation(); // the main window's shortcuts stay quiet while Settings is open
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (backdrop click closes; Esc is the keyboard path) -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1" bind:this={dialogEl} onkeydown={keys}>
    <header>
      <h2 id="settings-title">{t('settings.title')}</h2>
      <button class="icon-btn close" aria-label={t('common.close')} title={t('common.close')} onclick={onclose}><X size={16} aria-hidden="true" /></button>
    </header>
    <div class="body">
      <Settings />
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.32);
  }
  .dialog {
    width: min(560px, 100%);
    max-height: min(680px, 100%);
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .dialog:focus { outline: none; }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 10px 20px;
    border-bottom: 1px solid var(--border);
  }
  h2 { margin: 0; font-size: 15px; font-weight: 600; }
  .body { overflow-y: auto; padding: 16px 20px 22px; }
  @media (prefers-reduced-motion: no-preference) {
    .scrim { animation: fade 0.14s ease-out; }
    .dialog { animation: rise 0.18s cubic-bezier(0.2, 0.8, 0.2, 1); }
    @keyframes fade { from { opacity: 0; } }
    @keyframes rise { from { opacity: 0; transform: translateY(6px) scale(0.99); } }
  }
</style>
