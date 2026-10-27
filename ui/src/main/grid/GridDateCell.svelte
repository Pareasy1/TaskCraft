<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { X } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { ymd } from '../../lib/format';
  import { dateCommitAction } from '../../lib/gridEdit';

  // The Grid's Start / Due editor: a date input. The time of day is kept by the save (gridEdit).
  let {
    iso,
    label,
    onsave,
    ondone,
  }: {
    iso: string | null;
    /** The column's name, for the input's label. */
    label: string;
    /** Saves `YYYY-MM-DD` ('' clears); resolves to an error message, or null when saved. */
    onsave: (value: string) => Promise<string | null>;
    ondone: () => void;
  } = $props();

  // The value the editor opened with (a refresh while editing doesn't change it).
  const initial = untrack(() => (iso ? ymd(new Date(iso)) : ''));
  let value = $state(initial);
  let error = $state('');
  let busy = false;
  /** Set once the edit ended, so a late blur (Esc, the input going away) never saves. */
  let closed = false;
  function finish() {
    closed = true;
    ondone();
  }
  let inputEl = $state<HTMLInputElement | undefined>();
  let editEl = $state<HTMLElement | undefined>();
  onMount(() => inputEl?.focus());

  async function save(v: string) {
    busy = true;
    const err = await onsave(v);
    busy = false;
    if (err) error = err;
    else finish();
  }
  function commit(via: 'enter' | 'blur') {
    if (busy || closed) return;
    // A half-typed date reads as '' — never let that clear the stored date.
    const act = dateCommitAction(value, !!inputEl?.validity.badInput, initial, via);
    if (act === 'error') error = t('table.badDate');
    else if (act === 'cancel') finish();
    else save(act === 'clear' ? '' : value);
  }
  function keys(e: KeyboardEvent) {
    e.stopPropagation(); // the grid's own keys must not act inside the editor
    if (e.key === 'Enter') {
      e.preventDefault();
      commit('enter');
    } else if (e.key === 'Escape') {
      e.preventDefault();
      finish();
    }
  }
  function focusout(e: FocusEvent) {
    // Moving between the input and the Clear button keeps the editor open; leaving it for anywhere
    // else decides like any blur: a valid date saves, a half-typed one cancels.
    const to = e.relatedTarget as Node | null;
    if (to && editEl?.contains(to)) return;
    commit('blur');
  }
  function clearKeys(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Escape') {
      e.preventDefault();
      finish();
    }
  }
</script>

<span class="edit" bind:this={editEl} onfocusout={focusout}>
  <span class="row">
    <input type="date" bind:this={inputEl} bind:value aria-label={label} aria-invalid={!!error} onkeydown={keys} oninput={() => (error = '')}
      ondblclick={(e) => e.stopPropagation()} />
    {#if initial}
      <button class="icon-btn" aria-label={t('table.clearDate', { col: label })} title={t('table.clearDate', { col: label })}
        onpointerdown={(e) => e.preventDefault()} onclick={() => !busy && !closed && save('')} onkeydown={clearKeys}
        ondblclick={(e) => e.stopPropagation()}><X size={12} aria-hidden="true" /></button>
    {/if}
  </span>
  {#if error}<span class="err" role="alert">{error}</span>{/if}
</span>

<style>
  .edit { display: inline-flex; flex-direction: column; gap: 2px; }
  .row { display: inline-flex; align-items: center; gap: 2px; }
  .row .icon-btn { padding: 2px; }
  input { font: inherit; padding: 1px 4px; border: 1px solid var(--accent); border-radius: 5px; background: var(--bg); color: var(--text); }
  input[aria-invalid='true'] { border-color: var(--danger); }
  input:focus { outline: none; box-shadow: 0 0 0 2px var(--accent-soft); }
  .err { font-size: 11.5px; color: var(--danger); white-space: normal; max-width: 220px; }
</style>
