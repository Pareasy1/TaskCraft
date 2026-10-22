<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';
  import type { PopupMenu } from '../lib/menu.svelte';

  // Markup and look for a PopupMenu; items are buttons with role="menuitem…",
  // separators are <div class="sep" role="separator">.
  let { menu, label, children }: { menu: PopupMenu<T>; label: string; children: Snippet } = $props();
</script>

{#if menu.current}
  <div class="menu" role="menu" tabindex="-1" aria-label={label} style="left:{menu.current.x}px; top:{menu.current.y}px" bind:this={menu.el} onkeydown={menu.keys}>
    {@render children()}
  </div>
{/if}

<style>
  /* Fixed so a scrolling or clipping parent never cuts it off. */
  .menu {
    position: fixed;
    z-index: 30;
    min-width: 180px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 4px;
  }
  .menu:focus { outline: none; }
  .menu > :global(button) { display: block; width: 100%; border: none; background: none; padding: 5px 8px; border-radius: 5px; text-align: left; color: var(--text); }
  .menu > :global(button:hover:not(:disabled)), .menu > :global(button:focus-visible) { background: var(--hover); outline: none; }
  .menu > :global(button:disabled) { color: var(--faint); }
  .menu > :global(button.danger) { color: var(--danger); }
  .menu > :global(.sep) { height: 1px; background: var(--border); margin: 4px 2px; }
</style>
