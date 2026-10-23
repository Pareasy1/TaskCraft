<script lang="ts" module>
  import type { Task, TaskStatus } from '../../lib/types';

  /** One board column: Status columns pass `status` (header glyph); others may pass a colour dot. */
  export interface BoardColumn {
    key: string;
    title: string;
    tasks: Task[];
    status?: TaskStatus;
    color?: string;
    danger?: boolean;
    empty: string;
    /** Header count when it differs from the cards shown (Done hides older ones). */
    count?: number;
    /** Footer link, e.g. Done's "Show 12 older". */
    more?: { label: string; onclick: () => void };
  }
</script>

<script lang="ts">
  // `t` names a task throughout this component, so the translator is `tr`.
  import { t as tr } from '../../lib/i18n';
  import { flip } from 'svelte/animate';
  import { tick } from 'svelte';
  import type { Plan } from '../../lib/types';
  import { STATUS_LABEL } from '../../lib/tasks';
  import StatusGlyph from './StatusGlyph.svelte';
  import BoardCard from './BoardCard.svelte';
  import Doodle from './Doodle.svelte';

  let {
    columns,
    now,
    activeLabels,
    plans = [],
    onstatus,
    onopen,
    onlabel,
    ondrop,
    onkeymove,
    children,
    stateOf = (t) => STATUS_LABEL[t.status],
  }: {
    columns: BoardColumn[];
    now: Date;
    activeLabels: number[];
    /** Given in All tasks: each card shows its plan's dot. */
    plans?: Plan[];
    onstatus: (t: Task, s: TaskStatus) => void;
    onopen: (t: Task) => void;
    onlabel: (id: number) => void;
    /** A card dropped on a column; without it the board has no drag and drop. */
    ondrop?: (t: Task, key: string) => void;
    /** Left/Right arrow on a card's title; return true if it moved the card. */
    onkeymove?: (t: Task, by: -1 | 1) => boolean;
    /** Subtasks by parent id, shown on expandable cards. */
    children?: Map<number, Task[]>;
    /** What the arrow keys change, read out on the card ("To do", "Urgent"). */
    stateOf?: (t: Task) => string;
  } = $props();

  let dragging = $state<number | null>(null);
  let over = $state<string | null>(null);
  const reduce = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  const planOf = (t: Task) => (plans.length ? plans.find((p) => p.id === t.planId) : undefined);

  function drop(e: DragEvent, key: string) {
    e.preventDefault();
    over = null;
    const id = Number(e.dataTransfer?.getData('text/plain'));
    const t = columns.flatMap((c) => c.tasks).find((x) => x.id === id);
    if (t && ondrop) ondrop(t, key);
  }

  let refocus: number | null = null;
  function keys(e: KeyboardEvent, t: Task) {
    if (!onkeymove || (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft')) return;
    if (!onkeymove(t, e.key === 'ArrowRight' ? 1 : -1)) return;
    e.preventDefault();
    refocus = t.id;
  }
  // The card is re-created in its new column; keep keyboard focus on it.
  $effect(() => {
    void columns;
    if (refocus == null) return;
    const id = refocus;
    tick().then(() => {
      const el = document.querySelector<HTMLElement>(`[data-card="${id}"]`);
      if (el && el !== document.activeElement) {
        el.focus();
        refocus = null;
      }
    });
  });
</script>

<div class="board" class:wide={columns.length > 3}>
  {#each columns as col (col.key)}
    <section
      class="col"
      class:over={over === col.key}
      aria-label={col.title}
      ondragover={(e) => { if (!ondrop) return; e.preventDefault(); over = col.key; }}
      ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) over = null; }}
      ondrop={(e) => drop(e, col.key)}
    >
      <header>
        {#if col.status}<StatusGlyph status={col.status} size={15} />{:else if col.color}<span class="dot" style="background:{col.color}" aria-hidden="true"></span>{/if}
        <h2 class:danger={col.danger}>{col.title}</h2>
        <span class="n">{col.count ?? col.tasks.length}</span>
      </header>
      <div class="cards">
        {#each col.tasks as t (t.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions (drag is a mouse shortcut; the status menu and arrow keys are the keyboard path) -->
          <div
            class="slot"
            class:dragging={dragging === t.id}
            draggable={ondrop ? 'true' : undefined}
            animate:flip={{ duration: reduce ? 0 : 180 }}
            ondragstart={(e) => { dragging = t.id; e.dataTransfer?.setData('text/plain', String(t.id)); }}
            ondragend={() => { dragging = null; over = null; }}
          >
            <BoardCard task={t} {now} {activeLabels} plan={planOf(t)}
              titleLabel={onkeymove ? tr('board.cardAria', { title: t.title, status: stateOf(t) }) : undefined}
              onstatus={(s) => onstatus(t, s)} onopen={() => onopen(t)} {onlabel} ontitlekey={(e) => keys(e, t)}
              children={children?.get(t.id)} onchildstatus={onstatus} onchildopen={onopen} />
          </div>
        {:else}
          <div class="empty"><Doodle size={34} /><p>{col.empty}</p></div>
        {/each}
        {#if col.more}
          <button class="more" onclick={col.more.onclick}>{col.more.label}</button>
        {/if}
      </div>
    </section>
  {/each}
</div>

<style>
  .board { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; height: 100%; padding-bottom: 16px; }
  /* More than three columns (due dates, labels): fixed width, scroll sideways. */
  .board.wide { grid-template-columns: none; grid-auto-flow: column; grid-auto-columns: minmax(240px, 1fr); overflow-x: auto; }
  .col {
    background: var(--bg-sunken);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    box-shadow: inset 0 0 0 1px var(--border);
    transition: box-shadow 0.15s;
  }
  .col.over { box-shadow: inset 0 0 0 2px var(--accent); }
  header { display: flex; align-items: center; gap: 7px; padding: 11px 12px 6px; }
  h2 { margin: 0; font-size: 13px; font-weight: 600; }
  h2.danger { color: var(--danger); }
  .dot { width: 9px; height: 9px; border-radius: 50%; flex: none; }
  .n { color: var(--faint); font-size: 12px; font-variant-numeric: tabular-nums; }
  .cards { flex: 1; overflow-y: auto; padding: 4px 8px 10px; display: flex; flex-direction: column; gap: 6px; }
  .slot.dragging { opacity: 0.45; }
  .empty { color: var(--muted); font-size: 12px; text-align: center; margin: 18px 8px; }
  .empty p { margin: 0; }
  .more { border: none; background: none; color: var(--accent); font-size: 12px; padding: 6px; border-radius: 6px; }
  .more:hover { background: var(--hover); }
  @media (max-width: 720px) { .board { grid-template-columns: 1fr; height: auto; } }
</style>
