<script lang="ts">
  import { tick } from 'svelte';
  import { flip } from 'svelte/animate';
  import { Ellipsis, Plus } from 'lucide-svelte';
  // `t` names a task throughout this component, so the translator is `tr`.
  import { t as tr } from '../../lib/i18n';
  import { api, errorMessage } from '../../lib/api';
  import { newTaskInput } from '../../lib/actions';
  import { askSaveTemplate, duplicate } from '../../lib/taskCommands.svelte';
  import { bucketColumns, dropIndex, dropNeighbours, insertAt, uniqueName } from '../../lib/plans';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { PRIORITIES, type Bucket, type DeleteMode, type Plan, type Priority, type Task, type TaskStatus } from '../../lib/types';
  import BoardCard from './BoardCard.svelte';
  import Doodle from './Doodle.svelte';
  import Menu from '../Menu.svelte';
  import DeleteDialog from '../plans/DeleteDialog.svelte';
  import NewFromTemplate from './NewFromTemplate.svelte';

  let {
    plan,
    buckets,
    tasks,
    now,
    activeLabels,
    onstatus,
    onopen,
    onlabel,
    onerror,
    onreload,
    children,
    onpriority,
  }: {
    plan: Plan;
    /** The plan's buckets (Main loads them and reloads on every change). */
    buckets: Bucket[];
    /** The plan's tasks for the current filters. */
    tasks: Task[];
    now: Date;
    activeLabels: number[];
    onstatus: (t: Task, s: TaskStatus) => void;
    onopen: (t: Task) => void;
    onlabel: (id: number) => void;
    onerror: (msg: string) => void;
    /** Re-reads tasks and buckets from the store (after a failed move). */
    onreload: () => void;
    /** Subtasks by parent id, shown on expandable cards. */
    children?: Map<number, Task[]>;
    onpriority: (t: Task, p: Priority) => void;
  } = $props();

  let columns = $derived(bucketColumns(buckets, tasks));
  const reduce = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

  /** Runs a store command; on failure shows the error and snaps the board back to the store. */
  async function run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      onerror(errorMessage(e));
      onreload();
    }
  }

  // ── Moving cards ──
  /** Keyboard focus to restore on this card once the board re-renders. */
  // Plain variable: only a re-render (new `columns`) should act on it, not setting it.
  let refocus: number | null = null;
  $effect(() => {
    void columns;
    if (refocus == null) return;
    const id = refocus;
    tick().then(() => {
      const el = boardEl?.querySelector<HTMLElement>(`[data-card-menu="${id}"]`);
      // Found: done, whether or not focus had to move (a same-column move keeps the node).
      if (el) {
        if (el !== document.activeElement) el.focus();
        refocus = null;
      }
    });
  });

  async function moveTo(t: Task, bucketId: number, before: number | null, after: number | null) {
    try {
      await api.moveTask(t.id, bucketId, before, after);
    } catch (e) {
      if (refocus === t.id) refocus = null; // nothing moved: leave focus alone
      onerror(errorMessage(e));
      onreload();
    }
  }
  /** Last card of another bucket (cards appended at the end). */
  function moveToBucket(t: Task, b: Bucket) {
    const col = columns.find((c) => c.bucket.id === b.id);
    refocus = t.id;
    moveTo(t, b.id, col?.tasks.at(-1)?.id ?? null, null);
  }
  function shiftCard(t: Task, by: -1 | 1) {
    const ids = columnOf(t)?.tasks.map((x) => x.id) ?? [];
    const i = ids.indexOf(t.id);
    if (i + by < 0 || i + by >= ids.length) return;
    const { before, after } = dropNeighbours(ids, t.id, i + by);
    refocus = t.id;
    moveTo(t, t.bucketId, before, after);
  }
  const columnOf = (t: Task) => columns.find((c) => c.bucket.id === t.bucketId);

  // ── Drag and drop (cards, and bucket headers) ──
  let dragCard = $state<number | null>(null);
  let dragBucket = $state<number | null>(null);
  /** Card drop target: bucket and insertion index among its other cards. */
  let cardDrop = $state<{ bucket: number; index: number } | null>(null);
  /** Bucket drop target: insertion index among the columns. */
  let bucketDrop = $state<number | null>(null);
  let boardEl = $state<HTMLDivElement | undefined>();

  function dragOver(e: DragEvent, col: { bucket: Bucket; tasks: Task[] }, i: number) {
    // Both drags allow only "move"; WebKit rejects the drop unless dropEffect matches.
    if ((dragCard != null || dragBucket != null) && e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    if (dragCard != null) {
      e.preventDefault();
      const cards = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[data-slot]')].filter(
        (el) => Number(el.dataset.slot) !== dragCard,
      );
      const mids = cards.map((el) => {
        const r = el.getBoundingClientRect();
        return r.top + r.height / 2;
      });
      cardDrop = { bucket: col.bucket.id, index: dropIndex(mids, e.clientY) };
    } else if (dragBucket != null) {
      e.preventDefault();
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      bucketDrop = e.clientX < r.left + r.width / 2 ? i : i + 1;
    }
  }
  function drop(e: DragEvent, col: { bucket: Bucket; tasks: Task[] }) {
    e.preventDefault();
    const card = dragCard, target = cardDrop, bucket = dragBucket, at = bucketDrop;
    endDrag();
    if (card != null && target) {
      const t = tasks.find((x) => x.id === card);
      if (!t) return;
      const ids = col.tasks.map((x) => x.id);
      const { before, after } = dropNeighbours(ids, card, target.index);
      // Dropped back where it was: nothing to do.
      const i = ids.indexOf(card);
      if (t.bucketId === col.bucket.id && (ids[i - 1] ?? null) === before && (ids[i + 1] ?? null) === after) return;
      moveTo(t, col.bucket.id, before, after);
    } else if (bucket != null && at != null) {
      const ids = columns.map((c) => c.bucket.id);
      const next = insertAt(ids, bucket, at);
      if (next.some((id, n) => id !== ids[n])) run(() => api.reorderBuckets(plan.id, next));
    }
  }
  function endDrag() {
    dragCard = dragBucket = bucketDrop = null;
    cardDrop = null;
  }
  /** Index of `t` among the column's cards other than the one being dragged. */
  const restIndex = (col: { tasks: Task[] }, t: Task) => col.tasks.filter((x) => x.id !== dragCard).indexOf(t);

  // ── Buckets: rename, add, order, delete ──
  let editing = $state<number | null>(null);
  let draft = $state('');
  function startRename(b: Bucket) {
    draft = b.name;
    editing = b.id;
  }
  async function commitRename(b: Bucket) {
    if (editing !== b.id) return;
    editing = null;
    const name = draft.trim();
    if (!name || name === b.name) return; // empty reverts
    await run(() => api.renameBucket(b.id, name));
  }
  function renameKeys(e: KeyboardEvent, b: Bucket) {
    e.stopPropagation();
    if (e.key === 'Enter') commitRename(b);
    else if (e.key === 'Escape') editing = null;
    else return;
    // Without this, Enter would also press the "…" button that gets the focus back.
    e.preventDefault();
    tick().then(() => boardEl?.querySelector<HTMLElement>(`[data-bucket-menu="${b.id}"]`)?.focus());
  }
  function focusSelect(el: HTMLInputElement) {
    el.focus();
    el.select();
  }
  async function addBucket() {
    const name = uniqueName(tr('buckets.new'), buckets.map((b) => b.name));
    try {
      const id = await api.createBucket(plan.id, name);
      draft = name;
      editing = id; // the column shows its name field once the buckets reload
    } catch (e) {
      onerror(errorMessage(e));
    }
  }
  function shiftBucket(b: Bucket, by: -1 | 1) {
    const ids = columns.map((c) => c.bucket.id);
    const i = ids.indexOf(b.id), j = i + by;
    if (j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    run(() => api.reorderBuckets(plan.id, ids));
  }

  let deleting = $state<{ bucket: Bucket; count: number } | null>(null);
  async function askDelete(b: Bucket) {
    bucketMenu.close(); // the dialog hands focus back to the "…" button
    try {
      // Hidden (filtered or finished) tasks move or go too, so count them all.
      const count = (await api.listTasks('all')).filter((x) => x.bucketId === b.id).length;
      deleting = { bucket: b, count };
    } catch (e) {
      onerror(errorMessage(e));
    }
  }
  // The deleted column is gone from the DOM, so focus would fall to <body>;
  // once the reloaded buckets no longer have it, focus the first remaining "…".
  let afterDelete = $state<number | null>(null);
  $effect(() => {
    if (afterDelete === null || buckets.some((x) => x.id === afterDelete)) return;
    afterDelete = null;
    tick().then(() => boardEl?.querySelector<HTMLElement>('[data-bucket-menu]')?.focus());
  });
  async function confirmDelete(mode: DeleteMode) {
    const b = deleting?.bucket;
    deleting = null;
    if (!b) return;
    await run(() => api.deleteBucket(b.id, mode));
    afterDelete = b.id;
  }

  // ── New task at the bottom of a column ──
  let adding = $state<number | null>(null);
  let title = $state('');
  /** A template picked from the footer's "New from template": the field then takes just its title. */
  let fromTemplate = $state<{ id: number; name: string } | null>(null);
  async function createIn(b: Bucket) {
    const name = title.trim();
    if (!name) return;
    title = '';
    const tpl = fromTemplate;
    try {
      // Same defaults as the Tasks page's quick entry: default calendar, local time zone, no date.
      const input = await newTaskInput(name, plan.id, b.id);
      if (!tpl) return void (await api.createTask(input));
      const id = await api.createFromTemplate(tpl.id, input);
      fromTemplate = null;
      adding = null;
      onopen(await api.getTask(id));
    } catch (e) {
      if (!title) title = name; // give the typed text back
      onerror(errorMessage(e));
    }
  }
  function startAdding(b: Bucket, tpl: { id: number; name: string } | null = null) {
    title = '';
    fromTemplate = tpl;
    adding = b.id;
  }
  function stopAdding() {
    adding = null;
    fromTemplate = null;
  }
  function addKeys(e: KeyboardEvent, b: Bucket) {
    e.stopPropagation();
    if (e.key === 'Enter') createIn(b);
    else if (e.key === 'Escape') {
      e.preventDefault();
      stopAdding();
      tick().then(() => boardEl?.querySelector<HTMLElement>(`[data-add="${b.id}"]`)?.focus());
    }
  }

  /** Duplicate from the card menu; the copy lands right after the original. */
  async function duplicateCard(t: Task) {
    try {
      await duplicate(t, (id) => api.getTask(id).then(onopen));
    } catch (e) {
      onerror(errorMessage(e));
    }
  }

  // ── Menus ──
  const cardMenu = new PopupMenu<Task>();
  const bucketMenu = new PopupMenu<Bucket>();
</script>

<div class="board" bind:this={boardEl}>
  {#each columns as col, i (col.bucket.id)}
    {@const b = col.bucket}
    <!-- svelte-ignore a11y_no_static_element_interactions (drag is a mouse shortcut; the card and bucket menus are the keyboard path) -->
    <section
      class="col"
      class:over={cardDrop?.bucket === b.id}
      class:drop-before={dragBucket != null && bucketDrop === i}
      class:drop-after={dragBucket != null && bucketDrop === i + 1 && i === columns.length - 1}
      class:dragging={dragBucket === b.id}
      aria-label={b.name}
      animate:flip={{ duration: reduce ? 0 : 180 }}
      ondragover={(e) => dragOver(e, col, i)}
      ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) cardDrop = null; }}
      ondrop={(e) => drop(e, col)}
    >
      {#if editing === b.id}
        <header>
          <input class="name-field" aria-label={tr('buckets.name')} maxlength="60" bind:value={draft} use:focusSelect
            onkeydown={(e) => renameKeys(e, b)} onblur={() => commitRename(b)} />
        </header>
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions (drag and double-click are mouse shortcuts; the "…" menu has Rename and Move left/right) -->
        <header
          draggable="true"
          ondragstart={(e) => { dragBucket = b.id; e.dataTransfer?.setData('text/plain', b.name); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          ondragend={endDrag}
          ondblclick={() => startRename(b)}
          oncontextmenu={(e) => { e.preventDefault(); bucketMenu.open(b, e.clientX, e.clientY, boardEl?.querySelector<HTMLElement>(`[data-bucket-menu="${b.id}"]`) ?? (e.currentTarget as HTMLElement)); }}
        >
          <h2>{b.name}</h2>
          <span class="n">{col.tasks.length}</span>
          <span class="spacer"></span>
          <button class="icon-btn more" data-bucket-menu={b.id} class:open={bucketMenu.current?.target.id === b.id}
            aria-label={tr('buckets.options', { name: b.name })} title={tr('buckets.options', { name: b.name })}
            aria-haspopup="menu" aria-expanded={bucketMenu.current?.target.id === b.id}
            onclick={(e) => bucketMenu.openBelow(b, e.currentTarget, (x) => x.id === b.id)}
            onkeydown={(e) => bucketMenu.openFromKey(e, b)}><Ellipsis size={14} aria-hidden="true" /></button>
        </header>
      {/if}
      <div class="cards" class:drop-end={cardDrop?.bucket === b.id && cardDrop.index === col.tasks.filter((x) => x.id !== dragCard).length}>
        {#each col.tasks as t (t.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions (drag is a mouse shortcut; the card menu is the keyboard path) -->
          <div
            class="slot"
            data-slot={t.id}
            class:dragging={dragCard === t.id}
            class:drop-before={cardDrop?.bucket === b.id && cardDrop.index === restIndex(col, t) && dragCard !== t.id}
            draggable="true"
            animate:flip={{ duration: reduce ? 0 : 180 }}
            ondragstart={(e) => { e.stopPropagation(); dragCard = t.id; e.dataTransfer?.setData('text/plain', String(t.id)); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
            ondragend={endDrag}
          >
            <BoardCard task={t} {now} {activeLabels} struck
              onstatus={(s) => onstatus(t, s)} onopen={() => onopen(t)} {onlabel}
              children={children?.get(t.id)} onchildstatus={onstatus} onchildopen={onopen}
              ontitlekey={(e) => cardMenu.openFromKey(e, t, boardEl?.querySelector<HTMLElement>(`[data-card-menu="${t.id}"]`) ?? undefined)}
              oncontextmenu={(e) => { e.preventDefault(); cardMenu.open(t, e.clientX, e.clientY, boardEl?.querySelector<HTMLElement>(`[data-card-menu="${t.id}"]`) ?? (e.currentTarget as HTMLElement)); }}>
              {#snippet actions()}
                <button class="icon-btn more" data-card-menu={t.id} class:open={cardMenu.current?.target.id === t.id}
                  aria-label={tr('task.options', { title: t.title })} title={tr('task.options', { title: t.title })}
                  aria-haspopup="menu" aria-expanded={cardMenu.current?.target.id === t.id}
                  onclick={(e) => { e.stopPropagation(); cardMenu.openBelow(t, e.currentTarget, (x) => x.id === t.id); }}
                  onkeydown={(e) => cardMenu.openFromKey(e, t)}><Ellipsis size={14} aria-hidden="true" /></button>
              {/snippet}
            </BoardCard>
          </div>
        {:else}
          <div class="empty"><Doodle size={34} /><p>{tr('buckets.empty')}</p></div>
        {/each}
      </div>
      <footer>
        {#if adding === b.id}
          {@const label = fromTemplate ? tr('template.titleFor', { name: fromTemplate.name }) : tr('common.newTask')}
          <input class="name-field" aria-label={label} placeholder={fromTemplate ? label : tr('inspector.taskPlaceholder')} maxlength="500"
            bind:value={title} use:focusSelect onkeydown={(e) => addKeys(e, b)} onblur={() => { if (!title.trim()) stopAdding(); }} />
        {:else}
          <div class="add-row">
            <button class="add" data-add={b.id} onclick={() => startAdding(b)}><Plus size={13} aria-hidden="true" />{tr('common.newTask')}</button>
            <NewFromTemplate compact context={b.name} onpick={(tpl) => startAdding(b, tpl)} {onerror} />
          </div>
        {/if}
      </footer>
    </section>
  {/each}
  <div class="col ghost">
    <button class="add" onclick={addBucket}><Plus size={13} aria-hidden="true" />{tr('buckets.add')}</button>
  </div>
</div>

{#if cardMenu.current}
  {@const t = cardMenu.current.target}
  {@const ids = columnOf(t)?.tasks.map((x) => x.id) ?? []}
  <Menu menu={cardMenu} label={t.title}>
    {@const others = columns.filter((c) => c.bucket.id !== t.bucketId)}
    {#if others.length}
      <div class="sub" aria-hidden="true">{tr('task.moveTo')} ▸</div>
      {#each others as c (c.bucket.id)}
        <button role="menuitem" class="indent" aria-label={tr('task.moveToNamed', { name: c.bucket.name })}
          onclick={() => cardMenu.act((x) => moveToBucket(x, c.bucket))}>{c.bucket.name}</button>
      {/each}
      <div class="sep" role="separator"></div>
    {/if}
    <button role="menuitem" onclick={() => cardMenu.act(duplicateCard)}>{tr('task.duplicate')}</button>
    <button role="menuitem" onclick={() => cardMenu.act(askSaveTemplate)}>{tr('template.save')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" disabled={ids.indexOf(t.id) <= 0} onclick={() => cardMenu.act((x) => shiftCard(x, -1))}>{tr('task.moveUp')}</button>
    <button role="menuitem" disabled={ids.indexOf(t.id) >= ids.length - 1} onclick={() => cardMenu.act((x) => shiftCard(x, 1))}>{tr('task.moveDown')}</button>
    <div class="sep" role="separator"></div>
    <div class="sub" aria-hidden="true">{tr('priority.title')} ▸</div>
    {#each PRIORITIES as p (p)}
      <button role="menuitemradio" class="indent" aria-checked={t.priority === p}
        aria-label={`${tr('priority.title')}: ${tr(`priority.${p}`)}`}
        onclick={() => cardMenu.act((x) => onpriority(x, p))}><span class="check" aria-hidden="true">{t.priority === p ? '✓' : ''}</span>{tr(`priority.${p}`)}</button>
    {/each}
  </Menu>
{/if}

{#if bucketMenu.current}
  {@const b = bucketMenu.current.target}
  {@const index = columns.findIndex((c) => c.bucket.id === b.id)}
  <Menu menu={bucketMenu} label={b.name}>
    <button role="menuitem" onclick={() => bucketMenu.act(startRename, false)}>{tr('buckets.rename')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" disabled={index <= 0} onclick={() => bucketMenu.act((x) => shiftBucket(x, -1))}>{tr('buckets.moveLeft')}</button>
    <button role="menuitem" disabled={index >= columns.length - 1} onclick={() => bucketMenu.act((x) => shiftBucket(x, 1))}>{tr('buckets.moveRight')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" class="danger" onclick={() => askDelete(b)}>{tr('buckets.delete')}</button>
  </Menu>
{/if}

{#if deleting}
  <DeleteDialog
    kind="bucket"
    name={deleting.bucket.name}
    taskCount={deleting.count}
    targets={buckets.filter((x) => x.id !== deleting?.bucket.id).map((x) => ({ id: x.id, name: x.name }))}
    isLast={buckets.length <= 1}
    onconfirm={confirmDelete}
    oncancel={() => (deleting = null)}
  />
{/if}

<style>
  /* Buckets are fixed-width columns that scroll sideways, Planner style. */
  .board { display: grid; grid-auto-flow: column; grid-auto-columns: 272px; gap: 12px; height: 100%; padding-bottom: 16px; overflow-x: auto; }
  .col {
    position: relative;
    background: var(--bg-sunken);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    box-shadow: inset 0 0 0 1px var(--border);
    transition: box-shadow 0.15s;
  }
  .col.over { box-shadow: inset 0 0 0 2px var(--accent); }
  .col.dragging { opacity: 0.5; }
  /* Where a dragged bucket would land. */
  .col.drop-before::before, .col.drop-after::after {
    content: '';
    position: absolute;
    top: 6px;
    bottom: 6px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }
  .col.drop-before::before { left: -8px; }
  .col.drop-after::after { right: -8px; }
  .col.ghost { background: none; box-shadow: inset 0 0 0 1px var(--border); align-self: start; padding: 6px; }
  header { display: flex; align-items: center; gap: 7px; padding: 8px 8px 4px 12px; min-height: 38px; cursor: grab; }
  header:active { cursor: grabbing; }
  h2 { margin: 0; font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n { color: var(--faint); font-size: 12px; font-variant-numeric: tabular-nums; }
  .spacer { flex: 1; }
  .more { padding: 2px; opacity: 0; flex: none; }
  header:hover .more, .slot:hover .more, .more:focus-visible, .more.open { opacity: 1; }
  .cards { flex: 1; overflow-y: auto; padding: 4px 8px 6px; display: flex; flex-direction: column; gap: 6px; }
  .slot { position: relative; }
  .slot.dragging { opacity: 0.45; }
  /* Where a dragged card would land. */
  .slot.drop-before::before, .cards.drop-end::after {
    content: '';
    display: block;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }
  .slot.drop-before::before { position: absolute; left: 2px; right: 2px; top: -4px; }
  .cards.drop-end::after { margin: -2px 2px 0; flex: none; }
  .empty { color: var(--muted); font-size: 12px; text-align: center; margin: 18px 8px; }
  .empty p { margin: 0; }
  footer { padding: 0 8px 8px; }
  .add-row { display: flex; align-items: center; gap: 2px; }
  .add-row :global(.tpl) { opacity: 0; }
  .col:hover .add-row :global(.tpl), .add-row :global(.tpl:focus-visible), .add-row :global(.tpl.open) { opacity: 1; }
  .add { display: flex; align-items: center; gap: 6px; width: 100%; border: none; background: none; color: var(--muted); font-size: 12.5px; padding: 6px 8px; border-radius: 7px; text-align: left; }
  .add:hover { background: var(--hover); color: var(--text); }
  .add:focus-visible { outline: 2px solid var(--accent); }
  .name-field {
    flex: 1;
    width: 100%;
    min-width: 0;
    border: 1px solid var(--accent);
    background: var(--bg);
    border-radius: 6px;
    padding: 4px 7px;
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .sub { font-size: 12px; color: var(--muted); padding: 6px 8px 2px; }
  /* Doubled class: outranks the shared menu's button padding. */
  .indent.indent { padding-left: 18px; }
  .check { display: inline-block; width: 14px; margin-left: -12px; color: var(--accent); }
  @media (max-width: 720px) { .board { grid-auto-columns: minmax(240px, 85vw); } }
</style>
