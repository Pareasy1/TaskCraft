<script lang="ts">
  import { tick } from 'svelte';
  import { Ellipsis, ListTodo, Plus } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage } from '../../lib/api';
  import { nextPlanColor, PLAN_COLORS, uniqueName } from '../../lib/plans';
  import type { DeleteMode, Plan } from '../../lib/types';
  import DeleteDialog from './DeleteDialog.svelte';
  import Menu from '../Menu.svelte';
  import { PopupMenu } from '../../lib/menu.svelte';

  let {
    plans,
    scope,
    allCount,
    onselect,
    onerror,
  }: {
    plans: Plan[];
    scope: number | 'all';
    allCount: number;
    onselect: (s: number | 'all') => void;
    onerror: (m: string) => void;
  } = $props();

  const fail = (e: unknown) => onerror(errorMessage(e));

  // ── Create and inline rename ──
  let editing = $state<number | null>(null);
  let draft = $state('');

  async function create() {
    const name = uniqueName(t('plans.new'), plans.map((p) => p.name));
    try {
      const id = await api.createPlan(name, nextPlanColor(plans));
      draft = name;
      editing = id; // the row shows its name field once the list refreshes
    } catch (e) {
      fail(e);
    }
  }
  function startRename(p: Plan) {
    draft = p.name;
    editing = p.id;
  }
  async function commit(p: Plan) {
    if (editing !== p.id) return;
    editing = null;
    const name = draft.trim();
    if (!name || name === p.name) return; // empty reverts
    try {
      await api.updatePlan(p.id, { name });
    } catch (e) {
      fail(e);
    }
  }
  function renameKeys(e: KeyboardEvent, p: Plan) {
    e.stopPropagation();
    if (e.key === 'Enter') commit(p);
    else if (e.key === 'Escape') {
      e.preventDefault();
      editing = null;
    } else return;
    tick().then(() => rowButton(p.id)?.focus());
  }
  function focusSelect(el: HTMLInputElement) {
    el.focus();
    el.select();
  }

  // ── Order ──
  async function reorder(ids: number[]) {
    try {
      await api.reorderPlans(ids);
    } catch (e) {
      fail(e);
    }
  }
  function shift(p: Plan, by: -1 | 1) {
    const ids = plans.map((x) => x.id);
    const i = ids.indexOf(p.id), j = i + by;
    if (j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    reorder(ids);
  }

  let dragId = $state<number | null>(null);
  /** Insertion index (0…plans.length) where the dragged plan would land. */
  let dropAt = $state<number | null>(null);
  function dragOver(e: DragEvent, index: number) {
    if (dragId == null) return;
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropAt = e.clientY < r.top + r.height / 2 ? index : index + 1;
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    if (dragId == null || dropAt == null) return endDrag();
    const ids = plans.map((x) => x.id);
    const from = ids.indexOf(dragId);
    ids.splice(from, 1);
    ids.splice(dropAt > from ? dropAt - 1 : dropAt, 0, dragId);
    if (ids.some((id, i) => id !== plans[i].id)) reorder(ids);
    endDrag();
  }
  function endDrag() {
    dragId = null;
    dropAt = null;
  }

  // ── Menu ("…" button, right-click, Shift+F10) ──
  const menu = new PopupMenu<Plan>();
  const rowButton = (id: number) => listEl?.querySelector<HTMLButtonElement>(`[data-plan="${id}"]`);
  let listEl = $state<HTMLDivElement | undefined>();
  const closeMenu = (refocus = true) => menu.close(refocus);
  /** Runs a menu command on the menu's plan once the menu has closed. */
  const act = (fn: (p: Plan) => void, refocus = true) => menu.act(fn, refocus);

  async function recolor(p: Plan, color: string) {
    closeMenu();
    if (color === p.color) return;
    try {
      await api.updatePlan(p.id, { color });
    } catch (e) {
      fail(e);
    }
  }

  // ── Delete ──
  let deleting = $state<{ plan: Plan; count: number } | null>(null);
  async function askDelete(p: Plan) {
    closeMenu(); // the dialog hands focus back to the row
    try {
      // Completed tasks move or go too, so count every task in the plan.
      const count = (await api.listTasks('all')).filter((x) => x.planId === p.id).length;
      deleting = { plan: p, count };
    } catch (e) {
      fail(e);
    }
  }
  // The deleted row is gone from the DOM, so focus would fall to <body>; once
  // the refreshed list no longer has it, move focus to the "All tasks" row.
  let afterDelete = $state<number | null>(null);
  let allBtn = $state<HTMLButtonElement>();
  $effect(() => {
    if (afterDelete === null || plans.some((x) => x.id === afterDelete)) return;
    afterDelete = null;
    tick().then(() => allBtn?.focus());
  });
  async function confirmDelete(mode: DeleteMode) {
    const p = deleting?.plan;
    deleting = null;
    if (!p) return;
    try {
      await api.deletePlan(p.id, mode);
      afterDelete = p.id;
      if (scope === p.id) onselect('all');
    } catch (e) {
      fail(e);
    }
  }
</script>

<div class="label row">
  <span id="plans-title">{t('plans.title')}</span>
  <button class="icon-btn" title={t('plans.new')} aria-label={t('plans.new')} onclick={create}><Plus size={14} aria-hidden="true" /></button>
</div>
<div class="list" role="group" aria-labelledby="plans-title" bind:this={listEl}>
  {#each plans as p, i (p.id)}
    {#if editing === p.id}
      <div class="item editing">
        <span class="dot" style="background:{p.color}" aria-hidden="true"></span>
        <input class="name-field" aria-label={t('plans.name')} maxlength="60" bind:value={draft} use:focusSelect
          onkeydown={(e) => renameKeys(e, p)} onblur={() => commit(p)} />
      </div>
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions (drag is a mouse shortcut; Move up/down in the menu is the keyboard path) -->
      <div
        class="item"
        class:drop-before={dropAt === i && dragId !== null}
        class:drop-after={dropAt === i + 1 && i === plans.length - 1 && dragId !== null}
        class:dragging={dragId === p.id}
        draggable="true"
        ondragstart={(e) => { dragId = p.id; e.dataTransfer?.setData('text/plain', p.name); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
        ondragover={(e) => dragOver(e, i)}
        ondrop={drop}
        ondragend={endDrag}
        oncontextmenu={(e) => { e.preventDefault(); menu.open(p, e.clientX, e.clientY, rowButton(p.id) ?? (e.currentTarget as HTMLElement)); }}
      >
        <button class="main" data-plan={p.id} aria-current={scope === p.id ? 'page' : undefined} class:on={scope === p.id}
          onclick={() => onselect(p.id)} ondblclick={() => startRename(p)} onkeydown={(e) => menu.openFromKey(e, p)}>
          <span class="dot" style="background:{p.color}" aria-hidden="true"></span>
          <span class="nm">{p.name}</span>
          <span class="n">{p.openCount}</span>
        </button>
        <button class="icon-btn more" class:open={menu.current?.target.id === p.id} aria-label={t('plans.options', { name: p.name })} title={t('plans.options', { name: p.name })}
          aria-haspopup="menu" aria-expanded={menu.current?.target.id === p.id}
          onclick={(e) => menu.openBelow(p, e.currentTarget, (x) => x.id === p.id)}><Ellipsis size={14} aria-hidden="true" /></button>
      </div>
    {/if}
  {/each}
  <div class="item">
    <button class="main" bind:this={allBtn} aria-current={scope === 'all' ? 'page' : undefined} class:on={scope === 'all'} onclick={() => onselect('all')}>
      <ListTodo size={13} aria-hidden="true" class="all-icon" />
      <span class="nm">{t('plans.all')}</span>
      <span class="n">{allCount}</span>
    </button>
  </div>
</div>

{#if menu.current}
  {@const p = menu.current.target}
  {@const index = plans.findIndex((x) => x.id === p.id)}
  <Menu {menu} label={p.name}>
    <button role="menuitem" onclick={() => act(startRename, false)}>{t('plans.rename')}</button>
    <div class="sub" id="plan-color-label">{t('plans.color')}</div>
    <div class="swatches" role="group" aria-labelledby="plan-color-label">
      {#each PLAN_COLORS as c, n}
        <button role="menuitemradio" class="sw" aria-checked={c.toLowerCase() === p.color.toLowerCase()} style="background:{c}"
          aria-label={t('plans.useColor', { n: n + 1 })} title={t('plans.useColor', { n: n + 1 })} onclick={() => recolor(p, c)}></button>
      {/each}
    </div>
    <div class="sep" role="separator"></div>
    <button role="menuitem" disabled={index <= 0} onclick={() => act((x) => shift(x, -1))}>{t('plans.moveUp')}</button>
    <button role="menuitem" disabled={index >= plans.length - 1} onclick={() => act((x) => shift(x, 1))}>{t('plans.moveDown')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" class="danger" onclick={() => askDelete(p)}>{t('plans.delete')}</button>
  </Menu>
{/if}

{#if deleting}
  <DeleteDialog
    kind="plan"
    name={deleting.plan.name}
    taskCount={deleting.count}
    targets={plans.filter((x) => x.id !== deleting?.plan.id).map((x) => ({ id: x.id, name: x.name }))}
    isLast={plans.length <= 1}
    onconfirm={confirmDelete}
    oncancel={() => (deleting = null)}
  />
{/if}

<style>
  .label { font-size: 12px; font-weight: 600; color: var(--muted); padding: 14px 8px 4px; }
  .row { display: flex; align-items: center; justify-content: space-between; padding-right: 2px; padding-top: 10px; }
  .list { display: flex; flex-direction: column; gap: 1px; flex: 0 1 auto; min-height: 0; overflow-y: auto; }
  .item { position: relative; display: flex; align-items: center; border-radius: 6px; }
  .item:hover { background: var(--hover); }
  .item.dragging { opacity: 0.5; }
  .item.drop-before::before, .item.drop-after::after {
    content: '';
    position: absolute;
    left: 6px;
    right: 6px;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }
  .item.drop-before::before { top: -1px; }
  .item.drop-after::after { bottom: -1px; }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: none;
    text-align: left;
    padding: 4px 8px;
    border-radius: 6px;
    color: var(--text);
  }
  .main.on { background: var(--accent-soft); }
  .main.on .nm { font-weight: 600; }
  .dot { width: 9px; height: 9px; border-radius: 50%; flex: none; }
  .main :global(.all-icon) { flex: none; color: var(--muted); margin: 0 -2px; }
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n { color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  /* The "…" button appears on hover/focus so counts stay aligned. */
  .more { padding: 2px; margin-right: 2px; opacity: 0; }
  .item:hover .more, .more:focus-visible, .more.open { opacity: 1; }
  .editing { padding: 2px 8px; gap: 8px; }
  .name-field {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--accent);
    background: var(--bg);
    border-radius: 5px;
    padding: 2px 5px;
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .sub { font-size: 12px; color: var(--muted); padding: 6px 8px 2px; }
  .swatches { display: flex; gap: 7px; padding: 4px 8px 6px; }
  .sw { width: 16px; height: 16px; border-radius: 50%; border: none; padding: 0; flex: none; }
  .sw:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .sw[aria-checked='true'] { box-shadow: 0 0 0 2px var(--surface-strong), 0 0 0 3.5px var(--text); }
</style>
