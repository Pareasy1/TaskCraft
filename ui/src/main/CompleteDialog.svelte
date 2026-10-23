<script lang="ts">
  // "Complete the 2 open subtasks too?" — asked when a parent with open subtasks is ticked.
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import { answerComplete, type CompleteChoice } from '../lib/complete.svelte';
  import type { Task } from '../lib/types';

  let { task, openChildren }: { task: Task; openChildren: number } = $props();

  // Focus moves to the default button when it opens and back to where it came from when it closes.
  const opener = document.activeElement as HTMLElement | null;
  let dialogEl = $state<HTMLDivElement | undefined>();
  onMount(() => dialogEl?.querySelector<HTMLElement>('.primary')?.focus());
  onDestroy(() => {
    if (opener?.isConnected) opener.focus();
  });

  const answer = (c: CompleteChoice) => answerComplete(c);

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      answer('cancel');
    } else if (e.key === 'Tab' && dialogEl) {
      // Keep Tab inside the dialog.
      const list = [...dialogEl.querySelectorAll<HTMLElement>('button')];
      const i = list.indexOf(document.activeElement as HTMLElement);
      e.preventDefault();
      list[e.shiftKey ? (i <= 0 ? list.length - 1 : i - 1) : i === list.length - 1 ? 0 : i + 1]?.focus();
    }
    e.stopPropagation(); // the main window's shortcuts stay quiet while the dialog is open
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (backdrop click cancels; Esc is the keyboard path) -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && answer('cancel')}>
  <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="cmp-title" aria-describedby="cmp-desc" tabindex="-1" bind:this={dialogEl} onkeydown={keys}>
    <strong id="cmp-title">{task.title}</strong>
    <p id="cmp-desc">{openChildren === 1 ? t('complete.askChildrenOne') : t('complete.askChildren', { n: openChildren })}</p>
    <div class="actions">
      <button class="btn" onclick={() => answer('cancel')}>{t('common.cancel')}</button>
      <button class="btn" onclick={() => answer('only')}>{t('complete.onlyThis')}</button>
      <button class="btn primary" onclick={() => answer('all')}>{t('complete.all')}</button>
    </div>
  </div>
</div>

<style>
  /* Above the item dialog (z-index 25) and the menus (30); same look as the other centred dialogs. */
  .scrim { position: fixed; inset: 0; z-index: 40; display: grid; place-items: center; padding: 24px; background: rgba(0, 0, 0, 0.32); }
  .dialog {
    width: min(420px, 100%);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    padding: 16px 18px;
  }
  .dialog:focus { outline: none; }
  strong { display: block; overflow-wrap: anywhere; }
  p { margin: 8px 0 14px; line-height: 1.4; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; flex-wrap: wrap; }
</style>
