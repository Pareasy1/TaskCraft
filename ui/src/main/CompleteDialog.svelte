<script lang="ts">
  // "Complete the 2 open subtasks too?" — asked when a parent with open subtasks is ticked;
  // "All subtasks of “X” are done. Mark the task as done too?" — once its last subtask is done.
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import { answerComplete, type CompleteAsk, type CompleteChoice } from '../lib/complete.svelte';

  let { ask }: { ask: CompleteAsk } = $props();

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
  <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby={ask.kind === 'children' ? 'cmp-title' : 'cmp-desc'} aria-describedby={ask.kind === 'children' ? 'cmp-desc' : undefined} tabindex="-1" bind:this={dialogEl} onkeydown={keys}>
    {#if ask.kind === 'children'}
      <strong id="cmp-title">{ask.task.title}</strong>
      <p id="cmp-desc">{ask.openChildren === 1 ? t('complete.askChildrenOne') : t('complete.askChildren', { n: ask.openChildren })}</p>
      <div class="actions">
        <button class="btn" onclick={() => answer('cancel')}>{t('common.cancel')}</button>
        <button class="btn" onclick={() => answer('only')}>{t('complete.onlyThis')}</button>
        <button class="btn primary" onclick={() => answer('all')}>{t('complete.all')}</button>
      </div>
    {:else}
      <!-- The question names the task, so it needs no title line of its own. -->
      <p id="cmp-desc" class="only">{t('complete.askParent', { title: ask.task.title })}</p>
      <div class="actions">
        <button class="btn" onclick={() => answer('cancel')}>{t('complete.no')}</button>
        <button class="btn primary" onclick={() => answer('yes')}>{t('complete.yes')}</button>
      </div>
    {/if}
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
  p { margin: 8px 0 14px; line-height: 1.4; overflow-wrap: anywhere; }
  p.only { margin-top: 0; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; flex-wrap: wrap; }
</style>
