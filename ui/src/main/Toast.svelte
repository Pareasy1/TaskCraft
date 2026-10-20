<script lang="ts">
  import { X } from 'lucide-svelte';
  import { dismiss, toast, undo } from '../lib/toast.svelte';
</script>

<div class="region" role="status" aria-live="polite">
  {#if toast.current}
    <div class="toast">
      <span class="msg">{toast.current.message}</span>
      {#if toast.current.canUndo}<button class="undo" onclick={undo}>Undo</button>{/if}
      <button class="icon-btn" aria-label="Dismiss message" onclick={dismiss}><X size={14} aria-hidden="true" /></button>
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
  .msg { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .undo { border: none; background: none; color: var(--accent); font-weight: 600; padding: 4px 8px; border-radius: 6px; }
  .undo:hover { background: var(--hover); }
  @media (prefers-reduced-motion: no-preference) {
    .toast { animation: rise 0.18s ease-out; }
    @keyframes rise { from { opacity: 0; transform: translateY(6px); } }
  }
</style>
