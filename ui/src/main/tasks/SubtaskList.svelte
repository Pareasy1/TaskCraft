<script lang="ts">
  import { tick } from 'svelte';
  import { Ellipsis, Plus } from 'lucide-svelte';
  // `t` names a task in this component, so the translator is `tr`.
  import { t as tr } from '../../lib/i18n';
  import { api, errorMessage } from '../../lib/api';
  import { changeStatus } from '../../lib/actions';
  import { fmtShortDate, fmtTime } from '../../lib/format';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { dropIndex, insertAt } from '../../lib/plans';
  import { focusAfterRemoval, shiftId } from '../../lib/subtasks';
  import { notify } from '../../lib/toast.svelte';
  import { restoreSubtask, snapshotSubtask } from '../../lib/restore';
  import { duplicate } from '../../lib/taskCommands.svelte';
  import type { Task } from '../../lib/types';
  import Menu from '../Menu.svelte';
  import LabelChip from './LabelChip.svelte';
  import StatusGlyph from './StatusGlyph.svelte';

  let {
    parent,
    subtasks,
    onopen,
    onerror,
  }: { parent: Task; subtasks: Task[]; onopen: (t: Task) => void; onerror: (m: string) => void } = $props();

  let listEl = $state<HTMLUListElement | undefined>();
  let addEl = $state<HTMLInputElement | undefined>();

  /** Runs a store command; the dialog reloads the list on the change event. */
  async function run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      onerror(errorMessage(e));
    }
  }

  const due = (x: Task) => {
    if (!x.due) return '';
    const d = new Date(x.due);
    return x.dueHasTime ? `${fmtShortDate(d)}, ${fmtTime(d)}` : fmtShortDate(d);
  };

  // ── Order ──
  /** Keyboard focus to restore on this row's "…" button once the list re-renders. */
  let refocus: number | null = null;
  $effect(() => {
    void subtasks;
    if (refocus == null) return;
    const id = refocus;
    refocus = null;
    tick().then(() => listEl?.querySelector<HTMLElement>(`[data-sub-menu="${id}"]`)?.focus());
  });
  function reorder(ids: number[]) {
    run(() => api.reorderSubtasks(parent.id, ids));
  }
  function shift(x: Task, by: -1 | 1) {
    const next = shiftId(subtasks.map((s) => s.id), x.id, by);
    if (!next) return;
    refocus = x.id;
    reorder(next);
  }

  // Drag is the mouse shortcut; the row menu's Move up/down is the keyboard path.
  let dragId = $state<number | null>(null);
  let dropAt = $state<number | null>(null);
  function dragOver(e: DragEvent) {
    if (dragId == null) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    const rows = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[data-sub]')].filter((el) => Number(el.dataset.sub) !== dragId);
    dropAt = dropIndex(rows.map((el) => { const r = el.getBoundingClientRect(); return r.top + r.height / 2; }), e.clientY);
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    const id = dragId, at = dropAt;
    endDrag();
    if (id == null || at == null) return;
    const ids = subtasks.map((s) => s.id);
    // `at` counts the rows other than the dragged one; insertAt counts the full list.
    const from = ids.indexOf(id);
    const next = insertAt(ids, id, at > from ? at + 1 : at);
    if (next.some((x, i) => x !== ids[i])) reorder(next);
  }
  function endDrag() {
    dragId = dropAt = null;
  }
  const restIndex = (x: Task) => subtasks.filter((s) => s.id !== dragId).indexOf(x);

  // ── Row commands ──
  /** The row leaves the list: focus moves to its neighbour (or the add field), never to <body>. */
  async function leave(x: Task, fn: () => Promise<unknown>) {
    const next = focusAfterRemoval(subtasks.map((s) => s.id), x.id);
    try {
      await fn();
    } catch (e) {
      onerror(errorMessage(e));
      listEl?.querySelector<HTMLElement>(`[data-sub-menu="${x.id}"]`)?.focus();
      return;
    }
    // The neighbour's row stays when the list reloads (keyed), so it can take focus now.
    const el = next == null ? null : listEl?.querySelector<HTMLElement>(`[data-sub-menu="${next}"]`);
    (el ?? addEl)?.focus();
  }
  /** The copy goes right after it; focus stays on this row's "…". */
  const duplicateRow = (x: Task) => run(() => duplicate(x, (id) => api.getTask(id).then(onopen)));
  const makeNormal = (x: Task) => leave(x, () => api.setTaskParent(x.id, null));
  const remove = (x: Task) =>
    leave(x, async () => {
      const saved = await snapshotSubtask(x.id, subtasks.map((s) => s.id));
      await api.deleteTask(x.id);
      // Undo brings back its checklist and its place in the list.
      notify(tr('msg.deleted', { title: saved.task.input.title }), () => restoreSubtask(saved).then(() => {}));
    });

  // ── Adding ──
  let text = $state('');
  let pending: Promise<void> = Promise.resolve();
  function add(): Promise<void> {
    const title = text.trim();
    if (!title) return pending;
    text = ''; // the field keeps focus for the next one
    const job = (async () => {
      try {
        // A subtask lives in its parent's calendar and time zone (as checklist-to-subtask does).
        await api.createTask({
          calendarId: parent.calendarId, title, due: null, dueHasTime: false, tz: parent.tz, rrule: null, notes: null,
          alerts: [], status: 'todo', labels: [], parentId: parent.id,
        });
      } catch (e) {
        if (!text) text = title; // give the typed text back
        onerror(errorMessage(e));
        throw e;
      }
    })();
    pending = pending.then(() => job).catch(() => {});
    return job;
  }
  /** Adds any typed but not yet added subtask (the dialog calls this before saving or closing). */
  export async function flush(): Promise<void> {
    await pending;
    await add();
  }
  function addKeys(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      add().catch(() => {});
    } else if (e.key === 'Escape' && text) {
      e.stopPropagation(); // clears the field; Esc on an empty field closes the dialog
      e.preventDefault();
      text = '';
    }
  }

  const menu = new PopupMenu<Task>();
</script>

<section class="subtasks" aria-label={tr('subtasks.title')}>
  <h3>{tr('subtasks.title')}{#if subtasks.length}<span class="n">{tr('subtasks.progress', { done: subtasks.filter((s) => s.completedAt).length, total: subtasks.length })}</span>{/if}</h3>
  {#if subtasks.length}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions (drag is a mouse shortcut; the row menu is the keyboard path) -->
    <ul bind:this={listEl} ondragover={dragOver} ondrop={drop}
      ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) dropAt = null; }}
      class:drop-end={dragId != null && dropAt === subtasks.length - 1}>
      {#each subtasks as x (x.id)}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions (drag is a mouse shortcut; the row menu is the keyboard path) -->
        <li
          data-sub={x.id}
          class:done={x.status === 'done'}
          class:dragging={dragId === x.id}
          class:drop-before={dragId != null && dragId !== x.id && dropAt === restIndex(x)}
          draggable="true"
          ondragstart={(e) => { dragId = x.id; e.dataTransfer?.setData('text/plain', x.title); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          ondragend={endDrag}
          oncontextmenu={(e) => { e.preventDefault(); menu.open(x, e.clientX, e.clientY, listEl?.querySelector<HTMLElement>(`[data-sub-menu="${x.id}"]`) ?? (e.currentTarget as HTMLElement)); }}
        >
          <StatusGlyph status={x.status} size={15} onchange={(s) => run(() => changeStatus(x, s))} />
          <button class="title" onclick={() => onopen(x)} onkeydown={(e) => menu.openFromKey(e, x, listEl?.querySelector<HTMLElement>(`[data-sub-menu="${x.id}"]`) ?? undefined)}>{x.title}</button>
          {#each x.labels as l (l.id)}<LabelChip name={l.name} color={l.color} />{/each}
          {#if x.due}<span class="due">{due(x)}</span>{/if}
          <button class="icon-btn more" data-sub-menu={x.id} class:open={menu.current?.target.id === x.id}
            aria-label={tr('task.options', { title: x.title })} title={tr('task.options', { title: x.title })}
            aria-haspopup="menu" aria-expanded={menu.current?.target.id === x.id}
            onclick={(e) => menu.openBelow(x, e.currentTarget, (y) => y.id === x.id)}
            onkeydown={(e) => menu.openFromKey(e, x)}><Ellipsis size={14} aria-hidden="true" /></button>
        </li>
      {/each}
    </ul>
  {/if}
  <label class="add">
    <Plus size={13} aria-hidden="true" />
    <input bind:this={addEl} bind:value={text} placeholder={tr('subtasks.add')} aria-label={tr('subtasks.add')} maxlength="500"
      onkeydown={addKeys} onblur={() => add().catch(() => {})} />
  </label>
</section>

{#if menu.current}
  {@const x = menu.current.target}
  {@const i = subtasks.findIndex((s) => s.id === x.id)}
  <Menu {menu} label={x.title}>
    <button role="menuitem" onclick={() => menu.act(onopen, false)}>{tr('subtasks.open')}</button>
    <button role="menuitem" onclick={() => menu.act(duplicateRow)}>{tr('task.duplicate')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" disabled={i <= 0} onclick={() => menu.act((y) => shift(y, -1))}>{tr('task.moveUp')}</button>
    <button role="menuitem" disabled={i < 0 || i >= subtasks.length - 1} onclick={() => menu.act((y) => shift(y, 1))}>{tr('task.moveDown')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" onclick={() => menu.act(makeNormal, false)}>{tr('subtasks.makeTask')}</button>
    <button role="menuitem" class="danger" onclick={() => menu.act(remove, false)}>{tr('common.delete')}</button>
  </Menu>
{/if}

<style>
  .subtasks { display: flex; flex-direction: column; gap: 4px; }
  h3 { margin: 4px 0 2px; font-size: 12px; font-weight: 600; color: var(--muted); display: flex; gap: 8px; align-items: baseline; }
  .n { font-weight: 400; color: var(--faint); font-variant-numeric: tabular-nums; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  li { position: relative; display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-radius: 7px; min-height: 30px; cursor: grab; }
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
  .title { flex: 1; min-width: 0; border: none; background: none; padding: 0; text-align: left; font-size: 13px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; }
  .title:hover { text-decoration: underline; }
  .title:focus-visible { outline: 2px solid var(--accent); border-radius: 3px; }
  li.done .title { color: var(--muted); text-decoration: line-through; }
  .due { font-size: 11.5px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .more { padding: 2px; opacity: 0; flex: none; }
  li:hover .more, .more:focus-visible, .more.open { opacity: 1; }
  .add { display: flex; align-items: center; gap: 8px; padding: 2px 6px; color: var(--muted); }
  .add input { flex: 1; border: none; background: none; padding: 4px 0; font-size: 13px; color: var(--text); outline: none; }
  .add:focus-within { color: var(--accent); }
  .add input::placeholder { color: var(--muted); }
</style>
