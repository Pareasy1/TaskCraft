<script lang="ts">
  import { t } from '../lib/i18n';
  import { X } from 'lucide-svelte';
  import { dismiss, runAction, toast, undo } from '../lib/toast.svelte';
</script>

<div class="region" role="status" aria-live="polite">
  {#if toast.current}
    <div class="toast">
      <span class="msg">{toast.current.message}</span>
      {#if toast.current.action}<button class="undo" onclick={runAction}>{toast.current.action}</button>{/if}
      {#if toast.current.canUndo}<button class="undo" onclick={undo}>{t('common.undo')}</button>{/if}
      <button class="icon-btn" aria-label={t('common.dismiss')} onclick={dismiss}><X size={14} aria-hidden="true" /></button>
    </div>
  {/if}
</div>

<style>
  .region { position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%); z-index: 40; pointer-events: none; }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 7px 8px 7px 14px;
    max-width: min(520px, calc(100vw - 32px));
  }
  /* Long messages (e.g. files that couldn't be added, with the reason) wrap to three lines. */
  .msg { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; overflow: hidden; overflow-wrap: anywhere; }
  .undo { border: none; background: none; color: var(--accent); font-weight: 600; padding: 4px 8px; border-radius: 6px; }
  .undo:hover { background: var(--hover); }
  @media (prefers-reduced-motion: no-preference) {
    .toast { animation: rise 0.18s ease-out; }
    @keyframes rise { from { opacity: 0; transform: translateY(6px); } }
  }
</style>
