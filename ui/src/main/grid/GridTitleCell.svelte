<script lang="ts">
  import { untrack } from 'svelte';
  import { ChevronRight } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import type { GridRow } from '../../lib/grid';
  import type { Task } from '../../lib/types';

  // The Grid's Title cell: indent, collapse caret, and the inline title editor.
  let {
    row,
    editing,
    ontoggle,
    onsave,
    ondone,
  }: {
    row: GridRow<Task>;
    editing: boolean;
    ontoggle: () => void;
    /** Resolves to an error message (shown inline), or null when saved. */
    onsave: (value: string) => Promise<string | null>;
    /** Leaves edit mode (saved or cancelled). */
    ondone: () => void;
  } = $props();

  let value = $state('');
  let error = $state('');
  let busy = false;
  /** Set once the edit ended, so a late blur (Esc, the input going away) never saves. */
  let closed = false;
  let inputEl = $state<HTMLInputElement | undefined>();
  $effect(() => {
    if (!editing || !inputEl) return;
    // Only entering edit mode starts over; a data refresh while typing keeps the text.
    untrack(() => {
      value = row.task.title;
      error = '';
      closed = false;
      inputEl!.focus();
      inputEl!.select();
    });
  });

  function finish() {
    closed = true;
    ondone();
  }
  async function commit() {
    if (busy || closed) return;
    if (value.trim() === row.task.title) return finish();
    if (!value.trim()) {
      error = t('inspector.titleRequired');
      return;
    }
    busy = true;
    const err = await onsave(value);
    busy = false;
    if (err) error = err;
    else finish();
  }
  function keys(e: KeyboardEvent) {
    e.stopPropagation(); // the grid's own keys must not act while typing
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      finish();
    }
  }
  // Leaving the field saves a changed title; an empty one is dropped quietly.
  function blur() {
    if (!value.trim()) finish();
    else commit();
  }
  const canToggle = $derived(row.hasChildren && !row.dimmed);
</script>

<span class="cell" class:sub={row.depth === 1} title={row.dimmed ? t('table.parentRow') : undefined}>
  {#if canToggle}
    <!-- Not a button: keyboard users fold with Left/Right on the cell, which announces aria-expanded. -->
    <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
    <span class="caret" class:down={!row.collapsed} aria-hidden="true"
      title={row.collapsed ? t('subtasks.expand') : t('subtasks.collapse')}
      onclick={(e) => { e.stopPropagation(); ontoggle(); }} ondblclick={(e) => e.stopPropagation()}><ChevronRight size={14} /></span>
  {:else}
    <span class="caret-space" aria-hidden="true"></span>
  {/if}
  {#if editing}
    <span class="edit">
      <input bind:this={inputEl} bind:value aria-label={t('table.col.title')} aria-invalid={!!error} aria-describedby={error ? `title-err-${row.task.id}` : undefined}
        onkeydown={keys} onblur={blur} ondblclick={(e) => e.stopPropagation()} />
      {#if error}<span class="err" id="title-err-{row.task.id}" role="alert">{error}</span>{/if}
    </span>
  {:else}
    <span class="text">{row.task.title}{#if row.dimmed}<span class="sr"> {t('table.parentRowSr')}</span>{/if}</span>
  {/if}
</span>

<style>
  .cell { display: flex; align-items: center; gap: 4px; min-width: 0; }
  .cell.sub { padding-left: 20px; }
  .caret { cursor: pointer; padding: 0; width: 16px; height: 16px; flex: none; color: var(--muted); border-radius: 4px; display: inline-grid; place-items: center; }
  .caret:hover { background: var(--hover); color: var(--text); }
  .caret.down { transform: rotate(90deg); }
  @media (prefers-reduced-motion: no-preference) { .caret { transition: transform 0.15s; } }
  .caret-space { width: 16px; flex: none; }
  .text { overflow-wrap: anywhere; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .edit { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  input { font: inherit; width: 100%; padding: 2px 6px; border: 1px solid var(--accent); border-radius: 5px; background: var(--bg); color: var(--text); }
  input:focus { outline: none; box-shadow: 0 0 0 2px var(--accent-soft); }
  .err { font-size: 11.5px; color: var(--danger); }
</style>
