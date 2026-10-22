<script lang="ts">
  import { tick } from 'svelte';
  import { Ellipsis, ListTodo, Plus } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage } from '../../lib/api';
  import { nextPlanColor, PLAN_COLORS, uniqueName } from '../../lib/plans';
  import type { DeleteMode, Plan } from '../../lib/types';
  import DeleteDialog from './DeleteDialog.svelte';

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
  let menu = $state<{ plan: Plan; x: number; y: number } | null>(null);
  let menuEl = $state<HTMLDivElement | undefined>();
  let menuOpener: HTMLElement | null = null;
  const rowButton = (id: number) => listEl?.querySelector<HTMLButtonElement>(`[data-plan="${id}"]`);
  let listEl = $state<HTMLDivElement | undefined>();

  async function openMenu(p: Plan, x: number, y: number, opener: HTMLElement) {
    menuOpener = opener;
    menu = { plan: p, x, y };
    await tick();
    if (!menuEl) return;
    // Keep the menu inside the window; it may extend over the content area.
    const r = menuEl.getBoundingClientRect();
    menu = {
      plan: p,
      x: Math.max(8, Math.min(x, innerWidth - r.width - 8)),
      y: y + r.height > innerHeight - 8 ? Math.max(8, innerHeight - r.height - 8) : y,
    };
    menuItems()[0]?.focus();
  }
  function openFromButton(p: Plan, el: HTMLElement) {
    if (menu?.plan.id === p.id) return closeMenu();
    const r = el.getBoundingClientRect();
    openMenu(p, r.left, r.bottom + 4, el);
  }
  function closeMenu(refocus = true) {
    menu = null;
    if (refocus && menuOpener?.isConnected) menuOpener.focus();
  }
  const menuItems = () => [...(menuEl?.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]:not(:disabled)') ?? [])];
  function menuKeys(e: KeyboardEvent) {
    e.stopPropagation();
    const list = menuItems();
    const i = list.indexOf(document.activeElement as HTMLButtonElement);
    const go = (n: number) => list[(n + list.length) % list.length]?.focus();
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') go(i + 1);
    else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') go(i - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(list.length - 1);
    else if (e.key === 'Escape') closeMenu();
    else if (e.key === 'Tab') return closeMenu(false);
    else return;
    e.preventDefault();
  }
  function rowKeys(e: KeyboardEvent, p: Plan) {
    // Shift+F10 and the menu key open the plan menu, like a right-click.
    if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      openMenu(p, r.left + 16, r.bottom, e.currentTarget as HTMLElement);
    }
  }
  $effect(() => {
    if (!menu) return;
    const away = (e: PointerEvent) => !menuEl?.contains(e.target as Node) && !menuOpener?.contains(e.target as Node) && closeMenu(false);
    window.addEventListener('pointerdown', away, true);
    return () => window.removeEventListener('pointerdown', away, true);
  });
  /** Runs a menu command on the menu's plan once the menu has closed. */
  function act(fn: (p: Plan) => void, refocus = true) {
    const p = menu?.plan;
    closeMenu(refocus);
    if (p) fn(p);
  }

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
  async function confirmDelete(mode: DeleteMode) {
    const p = deleting?.plan;
    deleting = null;
    if (!p) return;
    try {
      await api.deletePlan(p.id, mode);
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
        oncontextmenu={(e) => { e.preventDefault(); openMenu(p, e.clientX, e.clientY, rowButton(p.id) ?? (e.currentTarget as HTMLElement)); }}
      >
        <button class="main" data-plan={p.id} aria-current={scope === p.id ? 'page' : undefined} class:on={scope === p.id}
          onclick={() => onselect(p.id)} ondblclick={() => startRename(p)} onkeydown={(e) => rowKeys(e, p)}>
          <span class="dot" style="background:{p.color}" aria-hidden="true"></span>
          <span class="nm">{p.name}</span>
          <span class="n">{p.openCount}</span>
        </button>
        <button class="icon-btn more" class:open={menu?.plan.id === p.id} aria-label={t('plans.options', { name: p.name })} title={t('plans.options', { name: p.name })}
          aria-haspopup="menu" aria-expanded={menu?.plan.id === p.id}
          onclick={(e) => openFromButton(p, e.currentTarget)}><Ellipsis size={14} aria-hidden="true" /></button>
      </div>
    {/if}
  {/each}
  <div class="item">
    <button class="main" aria-current={scope === 'all' ? 'page' : undefined} class:on={scope === 'all'} onclick={() => onselect('all')}>
      <ListTodo size={13} aria-hidden="true" class="all-icon" />
      <span class="nm">{t('plans.all')}</span>
      <span class="n">{allCount}</span>
    </button>
  </div>
</div>

{#if menu}
  {@const p = menu.plan}
  {@const index = plans.findIndex((x) => x.id === p.id)}
  <div class="menu" role="menu" tabindex="-1" aria-label={p.name} style="left:{menu.x}px; top:{menu.y}px" bind:this={menuEl} onkeydown={menuKeys}>
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
  </div>
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
  /* Fixed so the sidebar's clipping never cuts it off; it may extend over the content. */
  .menu {
    position: fixed;
    z-index: 30;
    min-width: 180px;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 4px;
  }
  .menu:focus { outline: none; }
  .menu > button { display: block; width: 100%; border: none; background: none; padding: 5px 8px; border-radius: 5px; text-align: left; color: var(--text); }
  .menu > button:hover:not(:disabled), .menu > button:focus-visible { background: var(--hover); outline: none; }
  .menu > button:disabled { color: var(--faint); }
  .menu > button.danger { color: var(--danger); }
  .sub { font-size: 12px; color: var(--muted); padding: 6px 8px 2px; }
  .swatches { display: flex; gap: 7px; padding: 4px 8px 6px; }
  .sw { width: 16px; height: 16px; border-radius: 50%; border: none; padding: 0; flex: none; }
  .sw:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .sw[aria-checked='true'] { box-shadow: 0 0 0 2px var(--surface-strong), 0 0 0 3.5px var(--text); }
  .sep { height: 1px; background: var(--border); margin: 4px 2px; }
</style>
