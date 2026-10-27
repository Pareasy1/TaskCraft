<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { ymd } from '../../lib/format';

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
  let inputEl: HTMLInputElement | undefined;
  onMount(() => inputEl?.focus());

  async function commit() {
    if (busy || closed) return;
    if (value === initial) return finish();
    busy = true;
    const err = await onsave(value);
    busy = false;
    if (err) error = err;
    else finish();
  }
  function keys(e: KeyboardEvent) {
    e.stopPropagation(); // the grid's own keys must not act inside the editor
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      finish();
    }
  }
</script>

<span class="edit">
  <input type="date" bind:this={inputEl} bind:value aria-label={label} aria-invalid={!!error} onkeydown={keys} onblur={() => (error ? finish() : commit())}
    ondblclick={(e) => e.stopPropagation()} />
  {#if error}<span class="err" role="alert">{error}</span>{/if}
</span>

<style>
  .edit { display: inline-flex; flex-direction: column; gap: 2px; }
  input { font: inherit; padding: 1px 4px; border: 1px solid var(--accent); border-radius: 5px; background: var(--bg); color: var(--text); }
  input[aria-invalid='true'] { border-color: var(--danger); }
  input:focus { outline: none; box-shadow: 0 0 0 2px var(--accent-soft); }
  .err { font-size: 11.5px; color: var(--danger); white-space: normal; max-width: 220px; }
</style>
