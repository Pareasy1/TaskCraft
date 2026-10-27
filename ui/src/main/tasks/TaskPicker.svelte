<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/api';
  import { pickerMatches } from '../../lib/dependencies';
  import type { Plan, Task } from '../../lib/types';
  import StatusGlyph from './StatusGlyph.svelte';

  let { tasks, exclude, onpick, oncancel }: { tasks: Task[]; exclude: Set<number>; onpick: (t: Task) => void; oncancel: () => void } = $props();

  const uid = Math.random().toString(36).slice(2, 8);
  let query = $state('');
  let active = $state(0);
  let plans = $state<Plan[]>([]);
  let inputEl = $state<HTMLInputElement | undefined>();
  let listEl = $state<HTMLUListElement | undefined>();

  const matches = $derived(tasks.filter((x) => !exclude.has(x.id) && pickerMatches(x.title, query)));
  const planName = (id: number) => plans.find((p) => p.id === id)?.name ?? '';
  $effect(() => {
    void query;
    active = 0;
  });

  onMount(() => {
    inputEl?.focus();
    api.listPlans().then((p) => (plans = p)).catch(() => {});
  });

  function move(by: number) {
    if (!matches.length) return;
    active = (active + by + matches.length) % matches.length;
    tick().then(() => listEl?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }));
  }
  function keys(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') move(1);
    else if (e.key === 'ArrowUp') move(-1);
    else if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
      if (matches[active]) onpick(matches[active]);
    } else if (e.key === 'Escape') {
      e.stopPropagation(); // cancels the picker; doesn't close the dialog
      oncancel();
    } else return;
    e.preventDefault();
  }
</script>

<div class="picker">
  <input bind:this={inputEl} bind:value={query} type="text" role="combobox" aria-expanded="true" aria-autocomplete="list"
    aria-controls="picker-{uid}" aria-activedescendant={matches[active] ? `picker-${uid}-${matches[active].id}` : undefined}
    placeholder={t('dep.search')} aria-label={t('dep.search')} onkeydown={keys} />
  <ul id="picker-{uid}" role="listbox" bind:this={listEl} aria-label={t('dep.search')}>
    {#each matches as x, i (x.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events (keyboard goes through the combobox input) -->
      <li id="picker-{uid}-{x.id}" role="option" aria-selected={i === active} class:active={i === active}
        onpointerenter={() => (active = i)} onmousedown={(e) => e.preventDefault()} onclick={() => onpick(x)}>
        <StatusGlyph status={x.status} size={14} />
        <span class="title">{x.title}</span>
        <span class="plan">{planName(x.planId)}</span>
      </li>
    {:else}
      <li class="empty" role="presentation">{t('dep.noMatches')}</li>
    {/each}
  </ul>
</div>

<style>
  .picker { display: flex; flex-direction: column; gap: 4px; border: 1px solid var(--border); border-radius: 8px; padding: 4px; background: var(--surface-strong); }
  input { border: 1px solid var(--border); background: var(--bg); border-radius: 5px; padding: 4px 6px; font-size: 13px; color: var(--text); outline: none; }
  input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
  ul { list-style: none; margin: 0; padding: 0; max-height: 180px; overflow-y: auto; display: flex; flex-direction: column; }
  li { display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-radius: 6px; cursor: pointer; min-height: 28px; }
  li.active { background: var(--hover); }
  .title { flex: 1; min-width: 0; font-size: 13px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .plan { flex: none; font-size: 11.5px; color: var(--muted); max-width: 40%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { cursor: default; font-size: 12.5px; color: var(--muted); }
</style>
