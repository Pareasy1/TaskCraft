<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import type { DeleteMode } from '../../lib/types';

  let {
    kind,
    name,
    taskCount,
    targets,
    isLast,
    onconfirm,
    oncancel,
  }: {
    kind: 'plan' | 'bucket';
    name: string;
    taskCount: number;
    targets: { id: number; name: string }[];
    isLast: boolean;
    onconfirm: (mode: DeleteMode) => void;
    oncancel: () => void;
  } = $props();

  let action = $state<'move' | 'delete'>('move');
  // svelte-ignore state_referenced_locally (the first target is only the initial choice)
  let moveTo = $state(targets[0]?.id ?? 0);
  // With nowhere to move them, deleting is the only choice.
  let canMove = $derived(targets.length > 0);
  $effect(() => {
    if (!canMove) action = 'delete';
  });

  let summary = $derived(
    kind === 'plan'
      ? taskCount === 1 ? t('del.planHasTasksOne') : t('del.planHasTasks', { n: taskCount })
      : taskCount === 1 ? t('del.bucketHasTasksOne') : t('del.bucketHasTasks', { n: taskCount }),
  );
  let lastText = $derived(kind === 'plan' ? t('del.lastPlan') : t('del.lastBucket'));

  function confirm() {
    if (isLast) return;
    // An empty plan/bucket has nothing to move; `moveTo` is still the safe mode.
    if (taskCount > 0 && action === 'delete') onconfirm('deleteTasks');
    else onconfirm(canMove ? { moveTo } : 'deleteTasks');
  }

  // Focus moves into the dialog when it opens and back to where it came from when it closes.
  const opener = document.activeElement as HTMLElement | null;
  let dialogEl = $state<HTMLDivElement | undefined>();
  onMount(() => dialogEl?.querySelector<HTMLElement>('.cancel')?.focus());
  onDestroy(() => {
    if (opener?.isConnected) opener.focus();
  });

  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      oncancel();
    } else if (e.key === 'Tab' && dialogEl) {
      // Keep Tab inside the dialog.
      const list = [...dialogEl.querySelectorAll<HTMLElement>('button, input, select')].filter(
        (el) => !(el as HTMLButtonElement).disabled && el.offsetParent !== null,
      );
      if (!list.length) return;
      const i = list.indexOf(document.activeElement as HTMLElement);
      e.preventDefault();
      list[e.shiftKey ? (i <= 0 ? list.length - 1 : i - 1) : i === list.length - 1 ? 0 : i + 1].focus();
    }
    e.stopPropagation(); // the main window's shortcuts stay quiet while the dialog is open
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (backdrop click cancels; Esc is the keyboard path) -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="del-title" aria-describedby="del-desc" tabindex="-1" bind:this={dialogEl} onkeydown={keys}>
    <strong id="del-title">{kind === 'plan' ? t('del.planTitle') : t('del.bucketTitle')}</strong>
    <div id="del-desc">
      {#if isLast}
        <p class="muted">{lastText}</p>
      {:else if taskCount === 0}
        <p>{t('del.confirmEmpty', { name })}</p>
      {:else}
        <p>{summary}</p>
        <div class="choices" role="radiogroup" aria-label={summary}>
          {#if canMove}
            <label class="choice">
              <input type="radio" name="del-mode" value="move" bind:group={action} />
              <span>{t('del.moveTo')}</span>
              <select class="field" aria-label={t('del.moveTo')} bind:value={moveTo} disabled={action !== 'move'}>
                {#each targets as x (x.id)}<option value={x.id}>{x.name}</option>{/each}
              </select>
            </label>
          {/if}
          <label class="choice">
            <input type="radio" name="del-mode" value="delete" bind:group={action} />
            <span>{t('del.deleteToo')}</span>
          </label>
        </div>
      {/if}
    </div>
    <div class="actions">
      <button class="btn cancel" onclick={oncancel}>{t('common.cancel')}</button>
      <button class="btn danger" disabled={isLast} title={isLast ? lastText : undefined} onclick={confirm}>{t('common.delete')}</button>
    </div>
  </div>
</div>

<style>
  /* Above the sidebar's menus (z-index 30); same look as the other centred dialogs. */
  .scrim { position: fixed; inset: 0; z-index: 40; display: grid; place-items: center; padding: 24px; background: rgba(0, 0, 0, 0.32); }
  .dialog {
    width: min(360px, 100%);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    padding: 16px 18px;
  }
  .dialog:focus { outline: none; }
  p { margin: 8px 0 12px; line-height: 1.4; }
  .muted { color: var(--muted); }
  .choices { display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; }
  .choice { display: flex; align-items: center; gap: 7px; }
  .choice input { margin: 0; }
  .choice .field { flex: 1; min-width: 0; width: auto; padding: 3px 6px; }
  .choice .field:disabled { opacity: 0.55; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; }
  .btn:disabled { opacity: 0.5; cursor: default; }
</style>
