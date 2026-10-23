<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { Ellipsis, Plus } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage, onItemsChanged } from '../../lib/api';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { dropIndex, insertAt } from '../../lib/plans';
  import { shiftId } from '../../lib/subtasks';
  import { notify } from '../../lib/toast.svelte';
  import type { ChecklistItem, Task } from '../../lib/types';
  import Menu from '../Menu.svelte';

  let { task, canConvert, onerror }: { task: Task; canConvert: boolean; onerror: (m: string) => void } = $props();

  let items = $state<ChecklistItem[]>([]);
  let listEl = $state<HTMLUListElement | undefined>();

  async function load() {
    const id = task.id;
    try {
      const list = await api.listChecklist(id);
      if (task.id === id) items = list;
    } catch (e) {
      onerror(errorMessage(e));
    }
  }
  $effect(() => {
    void task.id;
    load();
  });
  const unlisten = onItemsChanged(() => load());
  onDestroy(() => void unlisten.then((f) => f()));

  /** Runs a store command; a failure shows the error and reloads (snapping a reorder back). */
  async function run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      onerror(errorMessage(e));
      load();
    }
  }

  // ── Text: Enter or leaving the field saves, Esc reverts ──
  function commit(el: HTMLInputElement, item: ChecklistItem) {
    const text = el.value.trim();
    if (!text) return void (el.value = item.text); // empty reverts
    if (text !== item.text) run(() => api.updateChecklistItem(item.id, { text }));
  }
  function textKeys(e: KeyboardEvent, item: ChecklistItem) {
    const el = e.currentTarget as HTMLInputElement;
    if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) commit(el, item);
    else if (e.key === 'Escape') {
      e.stopPropagation(); // reverts the text; doesn't close the dialog
      el.value = item.text;
    } else if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      commit(el, item);
      shift(item, e.key === 'ArrowUp' ? -1 : 1);
    } else if (!menu.openFromKey(e, item, listEl?.querySelector<HTMLElement>(`[data-check-menu="${item.id}"]`) ?? undefined)) return;
    e.preventDefault();
  }

  // ── Order ──
  /** Element to focus once the list re-renders after a move. */
  let refocus: string | null = null;
  $effect(() => {
    void items;
    if (refocus == null) return;
    const sel = refocus;
    refocus = null;
    tick().then(() => listEl?.querySelector<HTMLElement>(sel)?.focus());
  });
  function shift(item: ChecklistItem, by: -1 | 1, from: 'text' | 'menu' = 'text') {
    const next = shiftId(items.map((i) => i.id), item.id, by);
    if (!next) return;
    refocus = from === 'text' ? `[data-check-text="${item.id}"]` : `[data-check-menu="${item.id}"]`;
    run(() => api.reorderChecklist(task.id, next));
  }

  let dragId = $state<number | null>(null);
  let dropAt = $state<number | null>(null);
  function dragOver(e: DragEvent) {
    if (dragId == null) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    const rows = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[data-check]')].filter((el) => Number(el.dataset.check) !== dragId);
    dropAt = dropIndex(rows.map((el) => { const r = el.getBoundingClientRect(); return r.top + r.height / 2; }), e.clientY);
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    const id = dragId, at = dropAt;
    endDrag();
    if (id == null || at == null) return;
    const ids = items.map((i) => i.id);
    // `at` counts the rows other than the dragged one; insertAt counts the full list.
    const next = insertAt(ids, id, at > ids.indexOf(id) ? at + 1 : at);
    if (next.some((x, i) => x !== ids[i])) run(() => api.reorderChecklist(task.id, next));
  }
  function endDrag() {
    dragId = dropAt = null;
  }
  const restIndex = (item: ChecklistItem) => items.filter((i) => i.id !== dragId).indexOf(item);

  // ── Row commands ──
  async function remove(item: ChecklistItem) {
    const order = items.map((i) => i.id);
    const taskId = task.id;
    try {
      await api.deleteChecklistItem(item.id);
    } catch (e) {
      return onerror(errorMessage(e));
    }
    notify(t('msg.deleted', { title: item.text }), async () => {
      // Back with its tick and in its old place.
      const id = await api.addChecklistItem(taskId, item.text);
      if (item.done) await api.updateChecklistItem(id, { done: true });
      const now = (await api.listChecklist(taskId)).map((i) => i.id);
      const ids = order.map((x) => (x === item.id ? id : x)).filter((x) => now.includes(x));
      await api.reorderChecklist(taskId, [...ids, ...now.filter((x) => !ids.includes(x))]);
    });
  }

  // ── Adding ──
  let text = $state('');
  async function add() {
    const value = text.trim();
    if (!value) return;
    text = ''; // the field keeps focus for the next one
    await run(() => api.addChecklistItem(task.id, value));
  }

  const menu = new PopupMenu<ChecklistItem>();
</script>

<section class="checklist" aria-label={t('checklist.title')}>
  <h3>{t('checklist.title')}{#if items.length}<span class="n">{items.filter((i) => i.done).length}/{items.length}</span>{/if}</h3>
  {#if items.length}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions (drag is a mouse shortcut; Alt+↑/↓ and the row menu are the keyboard path) -->
    <ul bind:this={listEl} ondragover={dragOver} ondrop={drop}
      ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) dropAt = null; }}
      class:drop-end={dragId != null && dropAt === items.length - 1}>
      {#each items as item (item.id)}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions (drag is a mouse shortcut; Alt+↑/↓ and the row menu are the keyboard path) -->
        <li
          data-check={item.id}
          class:done={item.done}
          class:dragging={dragId === item.id}
          class:drop-before={dragId != null && dragId !== item.id && dropAt === restIndex(item)}
          draggable="true"
          ondragstart={(e) => { dragId = item.id; e.dataTransfer?.setData('text/plain', item.text); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          ondragend={endDrag}
          oncontextmenu={(e) => { e.preventDefault(); menu.open(item, e.clientX, e.clientY, listEl?.querySelector<HTMLElement>(`[data-check-menu="${item.id}"]`) ?? (e.currentTarget as HTMLElement)); }}
        >
          <input type="checkbox" checked={item.done} aria-label={item.text}
            onchange={(e) => run(() => api.updateChecklistItem(item.id, { done: e.currentTarget.checked }))} />
          <input class="text" data-check-text={item.id} value={item.text} maxlength="200" aria-label={t('checklist.edit', { text: item.text })}
            onkeydown={(e) => textKeys(e, item)} onblur={(e) => commit(e.currentTarget, item)} />
          <button class="icon-btn more" data-check-menu={item.id} class:open={menu.current?.target.id === item.id}
            aria-label={t('checklist.options', { text: item.text })} title={t('checklist.options', { text: item.text })}
            aria-haspopup="menu" aria-expanded={menu.current?.target.id === item.id}
            onclick={(e) => menu.openBelow(item, e.currentTarget, (y) => y.id === item.id)}
            onkeydown={(e) => menu.openFromKey(e, item)}><Ellipsis size={14} aria-hidden="true" /></button>
        </li>
      {/each}
    </ul>
  {/if}
  <label class="add">
    <Plus size={13} aria-hidden="true" />
    <input bind:value={text} placeholder={t('checklist.add')} aria-label={t('checklist.add')} maxlength="200"
      onkeydown={(e) => { if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) { e.preventDefault(); add(); } }} />
  </label>
</section>

{#if menu.current}
  {@const item = menu.current.target}
  {@const i = items.findIndex((x) => x.id === item.id)}
  <Menu {menu} label={item.text}>
    <button role="menuitem" disabled={i <= 0} onclick={() => menu.act((y) => shift(y, -1, 'menu'))}>{t('task.moveUp')}</button>
    <button role="menuitem" disabled={i < 0 || i >= items.length - 1} onclick={() => menu.act((y) => shift(y, 1, 'menu'))}>{t('task.moveDown')}</button>
    <div class="sep" role="separator"></div>
    {#if canConvert}
      <button role="menuitem" onclick={() => menu.act((y) => run(() => api.checklistItemToSubtask(y.id)), false)}>{t('checklist.toSubtask')}</button>
    {/if}
    <button role="menuitem" class="danger" onclick={() => menu.act(remove, false)}>{t('common.delete')}</button>
  </Menu>
{/if}

<style>
  .checklist { display: flex; flex-direction: column; gap: 4px; }
  h3 { margin: 4px 0 2px; font-size: 12px; font-weight: 600; color: var(--muted); display: flex; gap: 8px; align-items: baseline; }
  .n { font-weight: 400; color: var(--faint); font-variant-numeric: tabular-nums; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  li { position: relative; display: flex; align-items: center; gap: 8px; padding: 2px 6px; border-radius: 7px; min-height: 30px; cursor: grab; }
  li:hover { background: var(--hover); }
  li.dragging { opacity: 0.45; }
  li.drop-before::before, ul.drop-end::after {
    content: '';
    display: block;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }
  li.drop-before::before { position: absolute; left: 4px; right: 4px; top: -1px; }
  ul.drop-end::after { margin: 0 4px; }
  input[type='checkbox'] { margin: 0; flex: none; accent-color: var(--accent); }
  .text { flex: 1; min-width: 0; border: 1px solid transparent; background: none; border-radius: 5px; padding: 3px 5px; margin-left: -5px; font-size: 13px; color: var(--text); }
  .text:hover { border-color: var(--border); }
  .text:focus { outline: none; border-color: var(--accent); background: var(--bg); box-shadow: 0 0 0 3px var(--accent-soft); }
  li.done .text { color: var(--muted); text-decoration: line-through; }
  .more { padding: 2px; opacity: 0; flex: none; }
  li:hover .more, .more:focus-visible, .more.open { opacity: 1; }
  .add { display: flex; align-items: center; gap: 8px; padding: 2px 6px; color: var(--muted); }
  .add input { flex: 1; border: none; background: none; padding: 4px 0; font-size: 13px; color: var(--text); outline: none; }
  .add:focus-within { color: var(--accent); }
  .add input::placeholder { color: var(--muted); }
</style>
