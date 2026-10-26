<script lang="ts">
  // "Save as template…": asks for the name; a name already in use asks "Replace “X”?" first.
  import { onDestroy, onMount, tick } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage } from '../../lib/api';
  import { nameConflict } from '../../lib/templates';
  import { notify } from '../../lib/toast.svelte';
  import type { Task } from '../../lib/types';

  let { task, onclose }: { task: Task; onclose: () => void } = $props();

  // svelte-ignore state_referenced_locally (the title is only the suggested name)
  let name = $state([...task.title.trim()].slice(0, 60).join(''));
  let error = $state('');
  let busy = $state(false);
  /** The existing template's name while asking whether to replace it. */
  let replacing = $state<string | null>(null);

  // Focus moves into the dialog when it opens and back to where it came from when it closes.
  const opener = document.activeElement as HTMLElement | null;
  let dialogEl = $state<HTMLDivElement | undefined>();
  let inputEl = $state<HTMLInputElement | undefined>();
  onMount(() => {
    inputEl?.focus();
    inputEl?.select();
  });
  onDestroy(() => {
    if (opener?.isConnected) opener.focus();
  });

  async function save(replace: boolean) {
    if (busy) return;
    const trimmed = name.trim();
    if (!trimmed || [...trimmed].length > 60) return void (error = t('error.template_name'));
    busy = true;
    error = '';
    try {
      await api.saveTemplate(task.id, trimmed, replace);
      notify(t('template.saved', { name: trimmed }));
      onclose();
    } catch (e) {
      if (!replace && e && typeof e === 'object' && (e as { code?: unknown }).code === 'duplicate') {
        // Name the template as stored (its own capitals) in the question.
        const existing = await api.listTemplates().then((list) => nameConflict(trimmed, list)?.name, () => null);
        replacing = existing ?? trimmed;
        await tick();
        dialogEl?.querySelector<HTMLElement>('.primary')?.focus();
      } else error = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  async function back() {
    replacing = null;
    await tick();
    inputEl?.focus();
    inputEl?.select();
  }

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (replacing != null) back();
      else onclose();
    } else if (e.key === 'Tab' && dialogEl) {
      // Keep Tab inside the dialog.
      const list = [...dialogEl.querySelectorAll<HTMLElement>('button, input')].filter((el) => !(el as HTMLButtonElement).disabled);
      if (!list.length) return;
      const i = list.indexOf(document.activeElement as HTMLElement);
      e.preventDefault();
      list[e.shiftKey ? (i <= 0 ? list.length - 1 : i - 1) : i === list.length - 1 ? 0 : i + 1].focus();
    }
    e.stopPropagation(); // the main window's shortcuts (and the details dialog's Esc) stay quiet
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (backdrop click cancels; Esc is the keyboard path) -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && onclose()}>
  {#if replacing == null}
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="tpl-title" tabindex="-1" bind:this={dialogEl} onkeydown={keys}>
      <strong id="tpl-title">{t('template.save')}</strong>
      <form onsubmit={(e) => { e.preventDefault(); save(false); }}>
        <label for="tpl-name">{t('template.name')}</label>
        <input id="tpl-name" class="field" bind:this={inputEl} bind:value={name} maxlength="60" autocomplete="off"
          aria-invalid={!!error} aria-describedby={error ? 'tpl-error' : undefined} oninput={() => (error = '')} />
        {#if error}<p class="error" id="tpl-error" role="alert">{error}</p>{/if}
        <div class="actions">
          <button type="button" class="btn" onclick={onclose}>{t('common.cancel')}</button>
          <button type="submit" class="btn primary" disabled={busy || !name.trim()}>{t('common.save')}</button>
        </div>
      </form>
    </div>
  {:else}
    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="tpl-replace" aria-describedby="tpl-replace-desc" tabindex="-1"
      bind:this={dialogEl} onkeydown={keys}>
      <strong id="tpl-replace">{t('template.replace', { name: replacing })}</strong>
      <p id="tpl-replace-desc">{t('template.replaceHint')}</p>
      <div class="actions">
        <button class="btn" onclick={back}>{t('common.cancel')}</button>
        <button class="btn primary" disabled={busy} onclick={() => save(true)}>{t('template.replaceButton')}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Above the item dialog (z-index 25) and the menus (30); same look as the other centred dialogs. */
  .scrim { position: fixed; inset: 0; z-index: 40; display: grid; place-items: center; padding: 24px; background: rgba(0, 0, 0, 0.32); }
  .dialog {
    width: min(380px, 100%);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    padding: 16px 18px;
  }
  .dialog:focus { outline: none; }
  strong { display: block; overflow-wrap: anywhere; }
  label { display: block; margin: 12px 0 5px; font-size: 12px; color: var(--muted); }
  .error { margin: 6px 0 0; font-size: 12px; color: var(--danger); }
  p { margin: 8px 0 0; line-height: 1.4; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; margin-top: 14px; }
  .btn:disabled { opacity: 0.5; cursor: default; }
</style>
