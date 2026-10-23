<script lang="ts">
  import { t } from '../lib/i18n';
  import { onMount, tick } from 'svelte';
  import type { Occurrence, Task, TaskStatus } from '../lib/types';
  import { addDays, layoutDay, nudgeEvent, sameDay, snap15 } from '../lib/layout';
  import { fmtHour, fmtShortDate, fmtTime, fmtWeekdayShort } from '../lib/format';
  import { calTaskTitle, movedSpan, shiftStart, tasksForDay, TASK_SLOT_MIN, type DayTask } from '../lib/calendarTasks';
  import { nextStatus, STATUS_LABEL } from '../lib/tasks';
  import StatusGlyph from './tasks/StatusGlyph.svelte';
  import { activate, isSelected, select } from '../lib/selection.svelte';

  const HOUR = 48; // px per hour
  const PX_MIN = HOUR / 60;

  let {
    days,
    occurrences,
    onselect,
    oncreate,
    onmove,
    tasks = [],
    ontaskopen = () => {},
    ontaskstatus = () => {},
    ontaskmove = () => {},
    parentTitles = new Map(),
  }: {
    days: Date[];
    occurrences: Occurrence[];
    onselect: (o: Occurrence) => void;
    oncreate: (start: Date) => void;
    onmove: (o: Occurrence, start: Date, end: Date) => void;
    tasks?: Task[];
    ontaskopen?: (t: Task) => void;
    ontaskstatus?: (t: Task, s: TaskStatus) => void;
    /** `due` is the new ISO due date; `start` moves with it for a span (null otherwise). */
    ontaskmove?: (t: Task, due: string, start: string | null) => void;
    /** Task id to title, to label subtasks "Subtask · Parent". */
    parentTitles?: Map<number, string>;
  } = $props();

  let now = $state(new Date());
  let scroller: HTMLDivElement | undefined = $state();
  let colsEl: HTMLDivElement | undefined = $state();
  let gridEl: HTMLDivElement | undefined = $state();

  const key = (o: Occurrence) => `${o.eventId}@${o.start}`;
  const isAllDay = (o: Occurrence) => o.allDay || new Date(o.end).getTime() - new Date(o.start).getTime() >= 86_400_000;

  let allDay = $derived(
    days.map((d) => occurrences.filter((o) => isAllDay(o) && new Date(o.start) < addDays(d, 1) && new Date(o.end) > d)),
  );
  // Tasks share each day with events: timed ones in the overlap layout, the rest in the all-day strip.
  let dayTasks = $derived(days.map((d) => tasksForDay(tasks, d, now)));
  const tkey = (task: Task) => `task:${task.id}`;
  type Placed =
    | { kind: 'event'; p: ReturnType<typeof layoutDay>[number]; o: Occurrence }
    | { kind: 'task'; p: ReturnType<typeof layoutDay>[number]; x: DayTask<Task> };
  let timed = $derived(
    days.map((d, i): Placed[] => {
      const items = occurrences.filter((o) => !isAllDay(o) && new Date(o.start) < addDays(d, 1) && new Date(o.end) > d);
      const ts = dayTasks[i].timed;
      const pos = layoutDay(
        [
          ...items.map((o) => ({ id: key(o), start: new Date(o.start), end: new Date(o.end) })),
          ...ts.map((x) => {
            const due = new Date(x.task.due!);
            return { id: tkey(x.task), start: due, end: new Date(due.getTime() + TASK_SLOT_MIN * 60_000) };
          }),
        ],
        d,
      );
      return pos.map((p) =>
        p.id.startsWith('task:')
          ? { kind: 'task' as const, p, x: ts.find((x) => tkey(x.task) === p.id)! }
          : { kind: 'event' as const, p, o: items.find((o) => key(o) === p.id)! },
      );
    }),
  );

  // ---- drag to move / resize ----
  type Drag = { o: Occurrence; mode: 'move' | 'resize'; x0: number; y0: number; dMin: number; dDay: number; moved: boolean };
  let drag = $state<Drag | null>(null);

  function begin(e: PointerEvent, o: Occurrence, mode: 'move' | 'resize') {
    if (e.button !== 0) return;
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { o, mode, x0: e.clientX, y0: e.clientY, dMin: 0, dDay: 0, moved: false };
  }
  function moveDrag(e: PointerEvent) {
    if (!drag || !colsEl) return;
    const colW = colsEl.clientWidth / days.length;
    drag.dMin = snap15(Math.abs((e.clientY - drag.y0) / PX_MIN)) * Math.sign(e.clientY - drag.y0);
    drag.dDay = drag.mode === 'move' ? Math.round((e.clientX - drag.x0) / colW) : 0;
    drag.moved ||= Math.abs(e.clientY - drag.y0) > 3 || Math.abs(e.clientX - drag.x0) > 3;
  }
  function endDrag() {
    if (!drag) return;
    const d = drag;
    drag = null;
    if (!d.moved) return select({ kind: 'event', id: d.o.eventId, occurrence: d.o.start });
    const s = new Date(d.o.start);
    const en = new Date(d.o.end);
    if (d.mode === 'move') {
      const ns = addDays(new Date(s.getTime() + d.dMin * 60_000), d.dDay);
      onmove(d.o, ns, new Date(ns.getTime() + (en.getTime() - s.getTime())));
    } else {
      const ne = new Date(Math.max(s.getTime() + 15 * 60_000, en.getTime() + d.dMin * 60_000));
      onmove(d.o, s, ne);
    }
  }
  const preview = (o: Occurrence, top: number, height: number) => {
    if (!drag || key(drag.o) !== key(o)) return { top, height, shift: 0 };
    return drag.mode === 'move'
      ? { top: top + drag.dMin, height, shift: drag.dDay }
      : { top, height: Math.max(15, height + drag.dMin), shift: 0 };
  };

  // ---- keyboard: Enter opens, arrows move/resize (see nudgeEvent) ----
  let refocus: number | null = null;
  function keydown(e: KeyboardEvent, o: Occurrence) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      return onselect(o);
    }
    const n = nudgeEvent(new Date(o.start), new Date(o.end), e.key, e.shiftKey);
    if (!n) return;
    e.preventDefault();
    refocus = o.eventId;
    onmove(o, n.start, n.end);
  }
  // Keep focus on the event after it re-renders at its new time.
  $effect(() => {
    void timed;
    if (refocus == null) return;
    const id = refocus;
    tick().then(() => {
      const el = colsEl?.querySelector<HTMLElement>(`[data-event="${id}"]`);
      if (el) {
        el.focus();
        refocus = null;
      }
    });
  });

  // ---- tasks: drag to move (time and day on the grid, day in the all-day strip) ----
  type TaskDrag = { task: Task; timed: boolean; x0: number; y0: number; colW: number; dMin: number; dDay: number; moved: boolean };
  let tdrag = $state<TaskDrag | null>(null);
  function beginTask(e: PointerEvent, task: Task, isTimed: boolean) {
    // The status mark handles its own clicks.
    if (e.button !== 0 || (e.target as HTMLElement).closest('.wrap')) return;
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const colW = (colsEl?.clientWidth ?? 0) / days.length;
    tdrag = { task, timed: isTimed, x0: e.clientX, y0: e.clientY, colW, dMin: 0, dDay: 0, moved: false };
  }
  function moveTask(e: PointerEvent) {
    if (!tdrag) return;
    const dy = e.clientY - tdrag.y0;
    tdrag.dMin = tdrag.timed ? snap15(Math.abs(dy / PX_MIN)) * Math.sign(dy) : 0;
    tdrag.dDay = tdrag.colW ? Math.round((e.clientX - tdrag.x0) / tdrag.colW) : 0;
    tdrag.moved ||= Math.abs(dy) > 3 || Math.abs(e.clientX - tdrag.x0) > 3;
  }
  function endTask() {
    const d = tdrag;
    tdrag = null;
    if (!d) return;
    if (!d.moved) return select({ kind: 'task', id: d.task.id });
    if (!d.dMin && !d.dDay) return;
    const due = new Date(d.task.due!);
    const newDue = addDays(new Date(due.getTime() + d.dMin * 60_000), d.dDay).toISOString();
    ontaskmove(d.task, newDue, shiftStart(d.task, newDue));
  }
  /** The system interrupted the gesture (app switch, trackpad gesture): drop the preview, move nothing. */
  const cancelTask = () => (tdrag = null);
  const dragging = (task: Task) => (tdrag && tdrag.moved && tdrag.task.id === task.id ? tdrag : null);

  // ---- tasks: Enter opens, Space changes status, arrows move (see movedDue) ----
  let refocusTask: number | null = null;
  function taskKey(e: KeyboardEvent, task: Task) {
    if (e.target !== e.currentTarget) return; // keys on the status mark are its own
    if (e.key === 'Enter') {
      e.preventDefault();
      return ontaskopen(task);
    }
    if (e.key === ' ') {
      e.preventDefault();
      return ontaskstatus(task, nextStatus(task.status));
    }
    const mv = movedSpan(task, e.key);
    if (!mv) return;
    e.preventDefault();
    refocusTask = task.id;
    ontaskmove(task, mv.due, mv.start);
  }
  $effect(() => {
    void dayTasks;
    if (refocusTask == null) return;
    const id = refocusTask;
    tick().then(() => {
      const el = gridEl?.querySelector<HTMLElement>(`[data-task="${id}"]`);
      if (el) {
        el.focus();
        refocusTask = null;
      }
    });
  });
  const titleOf = (task: Task) => calTaskTitle(task, parentTitles);
  const MARK = { start: '↦', middle: '→', end: '⇥' } as const;
  const taskAria = (x: DayTask<Task>) => {
    const due = new Date(x.task.due!);
    const when = x.task.dueHasTime ? `${fmtShortDate(due)}, ${fmtTime(due)}` : fmtShortDate(due);
    return t('grid.taskAria', {
      title: titleOf(x.task) + (x.span ? ` (${t(`span.${x.span}`)})` : ''),
      when: x.overdue ? `${when} (${t('group.overdue')})` : when,
      status: STATUS_LABEL[x.task.status],
    });
  };

  function dblclick(e: MouseEvent, day: Date) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const min = Math.floor((e.clientY - rect.top) / PX_MIN / 30) * 30;
    oncreate(new Date(day.getFullYear(), day.getMonth(), day.getDate(), 0, min));
  }

  onMount(() => {
    if (scroller) scroller.scrollTop = Math.max(0, (now.getHours() - 1.5) * HOUR);
    const t = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(t);
  });

  const hours = Array.from({ length: 24 }, (_, h) => h);
  const hourLabel = (h: number) => (h === 0 ? '' : fmtHour(h));
  const today = new Date();
</script>

<div class="grid" style="--n:{days.length}" bind:this={gridEl}>
  <div class="heads">
    <div class="gutter"></div>
    {#each days as d}
      <div class="head" class:today={sameDay(d, today)}>
        <span class="wd">{fmtWeekdayShort(d)}</span>
        <span class="num">{d.getDate()}</span>
      </div>
    {/each}
  </div>
  <div class="allday">
    <div class="gutter small">{t('grid.allDay')}</div>
    {#each allDay as items, i}
      <div class="adcol">
        {#each items as o (key(o))}
          <button class="chip" class:is-selected={isSelected('event', o.eventId, o.start)} style="--c:{o.color}"
            onclick={(e) => activate(e, () => onselect(o), () => select({ kind: 'event', id: o.eventId, occurrence: o.start }))}
            ondblclick={() => onselect(o)}>{o.title}</button>
        {/each}
        {#each dayTasks[i].allDay as x (x.task.id)}
          {@const dr = dragging(x.task)}
          <!-- svelte-ignore a11y_no_static_element_interactions (drag surface; the open button is the keyboard path) -->
          <div
            class="tchip"
            class:is-selected={isSelected('task', x.task.id)}
            ondblclick={() => ontaskopen(x.task)}
            class:done={x.task.status === 'done'}
            class:overdue={x.overdue}
            class:dragging={!!dr}
            class:span-start={x.span === 'start'}
            class:span-middle={x.span === 'middle'}
            class:span-end={x.span === 'end'}
            style="--c:{x.task.color}; transform:translateX({dr ? dr.dDay * dr.colW : 0}px)"
            onpointerdown={(e) => beginTask(e, x.task, false)}
            onpointermove={moveTask}
            onpointerup={endTask}
            onpointercancel={cancelTask}
          >
            <StatusGlyph status={x.task.status} size={13} onchange={(st) => ontaskstatus(x.task, st)} />
            <button class="open-btn opener" data-task={x.task.id} aria-label={taskAria(x)}
              onclick={(e) => e.detail === 0 && ontaskopen(x.task)} onkeydown={(e) => taskKey(e, x.task)}>
              {#if x.span}<span class="mark" aria-hidden="true">{MARK[x.span]}</span>{/if}
              <span class="tt">{titleOf(x.task)}</span>
              {#if !x.span && !sameDay(new Date(x.task.due!), days[i])}<span class="when">{fmtShortDate(new Date(x.task.due!))}</span>{/if}
            </button>
          </div>
        {/each}
      </div>
    {/each}
  </div>
  <div class="scroll" bind:this={scroller}>
    <div class="body">
      <div class="hours">
        {#each hours as h}<div class="hour"><span>{hourLabel(h)}</span></div>{/each}
      </div>
      <div class="cols" role="grid" tabindex="-1" bind:this={colsEl} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={() => (drag = null)}>
        {#each days as d, i}
          <div class="col" role="gridcell" tabindex="-1" ondblclick={(e) => dblclick(e, d)}>
            {#each hours as _}<div class="slot"></div>{/each}
            {#if sameDay(d, now)}
              <div class="now" style="top:{(now.getHours() * 60 + now.getMinutes()) * PX_MIN}px"></div>
            {/if}
            {#each timed[i] as item (item.p.id)}
              {#if item.kind === 'task'}
                {@const x = item.x}
                {@const p = item.p}
                {@const dr = dragging(x.task)}
                <!-- svelte-ignore a11y_no_static_element_interactions (drag surface; the open button is the keyboard path) -->
                <div
                  class="task"
                  class:is-selected={isSelected('task', x.task.id)}
                  ondblclick={() => ontaskopen(x.task)}
                  class:done={x.task.status === 'done'}
                  class:overdue={x.overdue}
                  class:dragging={!!dr}
                  style="--c:{x.task.color}; top:{(p.top + (dr?.dMin ?? 0)) * PX_MIN}px; height:{TASK_SLOT_MIN * PX_MIN - 2}px;
                         left:calc({(p.col / p.cols) * 100}% + {(dr?.dDay ?? 0) * 100}%); width:calc({100 / p.cols}% - 3px)"
                  onpointerdown={(e) => beginTask(e, x.task, true)}
                  onpointermove={moveTask}
                  onpointerup={endTask}
                  onpointercancel={cancelTask}
                >
                  <StatusGlyph status={x.task.status} size={13} onchange={(st) => ontaskstatus(x.task, st)} />
                  <button class="open-btn opener" data-task={x.task.id} aria-label={taskAria(x)}
                    onclick={(e) => e.detail === 0 && ontaskopen(x.task)} onkeydown={(e) => taskKey(e, x.task)}>
                    <span class="tt">{titleOf(x.task)}</span>
                    <span class="time">{fmtTime(new Date(x.task.due!))}</span>
                  </button>
                </div>
              {:else}
              {@const o = item.o}
              {@const p = item.p}
              {@const v = preview(o, p.top, p.height)}
              <div
                class="ev"
                class:is-selected={isSelected('event', o.eventId, o.start)}
                ondblclick={() => onselect(o)}
                class:compact={v.height * PX_MIN < 40}
                class:dragging={drag && key(drag.o) === p.id}
                style="--c:{o.color}; top:{v.top * PX_MIN}px; height:{v.height * PX_MIN - 2}px;
                       left:calc({(p.col / p.cols) * 100}% + {v.shift * 100}%); width:calc({100 / p.cols}% - 3px)"
                role="button"
                tabindex="0"
                data-event={o.eventId}
                aria-label={t('grid.eventAria', { title: o.title, start: fmtTime(new Date(o.start)), end: fmtTime(new Date(o.end)) })}
                onpointerdown={(e) => begin(e, o, 'move')}
                onkeydown={(e) => keydown(e, o)}
              >
                <span class="t">{o.title}</span>
                <span class="time">{fmtTime(new Date(o.start))}</span>
                <div class="resize" role="presentation" onpointerdown={(e) => begin(e, o, 'resize')}></div>
              </div>
              {/if}
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .grid { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .heads, .allday { display: grid; grid-template-columns: 54px repeat(var(--n), 1fr); }
  .heads { border-bottom: 1px solid var(--border); }
  .head { display: flex; align-items: baseline; justify-content: center; gap: 5px; padding: 6px 0; }
  .wd { color: var(--muted); font-size: 12px; }
  .num { font-size: 17px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .head.today .num {
    background: var(--today);
    color: var(--on-today);
    border-radius: 13px;
    min-width: 26px;
    text-align: center;
    padding: 0 6px;
  }
  .gutter { width: 54px; }
  .gutter.small { font-size: 11px; color: var(--muted); text-align: right; padding: 4px 6px 0 0; }
  .allday { border-bottom: 1px solid var(--border); min-height: 26px; }
  .adcol { border-left: 1px solid var(--border); padding: 2px; display: grid; gap: 2px; align-content: start; }
  .chip {
    border: none;
    text-align: left;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    background: color-mix(in srgb, var(--c) 22%, var(--bg));
    color: color-mix(in srgb, var(--c) 40%, var(--text));
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .scroll { flex: 1; overflow-y: auto; }
  .body { display: grid; grid-template-columns: 54px 1fr; position: relative; }
  .hour { height: 48px; position: relative; }
  .hour span {
    position: absolute;
    top: -7px;
    right: 6px;
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .cols { display: grid; grid-template-columns: repeat(var(--n), 1fr); position: relative; }
  .col { position: relative; border-left: 1px solid var(--border); }
  .slot { height: 48px; border-top: 1px solid var(--border); }
  .slot:first-child { border-top: none; }
  .now { position: absolute; left: -1px; right: 0; height: 2px; background: var(--today); z-index: 3; }
  .now::before {
    content: '';
    position: absolute;
    left: -4px;
    top: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--today);
  }
  .ev {
    position: absolute;
    margin-left: 1px;
    border-radius: 5px;
    padding: 2px 6px;
    background: color-mix(in srgb, var(--c) 20%, var(--bg));
    border-left: 3px solid var(--c);
    color: color-mix(in srgb, var(--c) 40%, var(--text));
    font-size: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 2;
    touch-action: none;
  }
  .ev:hover { background: color-mix(in srgb, var(--c) 28%, var(--bg)); }
  .ev:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; z-index: 5; }
  .ev.dragging { opacity: 0.85; box-shadow: var(--shadow); z-index: 4; }
  .t { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* Short events: title and time on one line, title first. */
  .ev.compact { flex-direction: row; align-items: baseline; gap: 5px; padding-top: 1px; padding-bottom: 0; }
  .ev.compact .t { flex: 0 1 auto; min-width: 0; }
  .ev.compact .time { flex: none; }
  .time { font-size: 11px; }
  .resize { position: absolute; left: 0; right: 0; bottom: 0; height: 6px; cursor: ns-resize; }

  /* Tasks are moments, not blocks: an outlined one-line row with the status mark. */
  .task, .tchip {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    border-radius: 5px;
    background: var(--bg);
    border: 1px solid color-mix(in srgb, var(--c) 55%, var(--border));
    color: var(--text);
    font-size: 12px;
    overflow: hidden;
    touch-action: none;
  }
  .task { position: absolute; margin-left: 1px; padding: 0 6px 0 3px; z-index: 2; }
  .tchip { padding: 1px 6px 1px 3px; }
  .task:hover, .tchip:hover { background: color-mix(in srgb, var(--c) 8%, var(--bg)); }
  .task:has(:focus-visible), .tchip:has(:focus-visible) { outline: 2px solid var(--accent); outline-offset: 1px; z-index: 5; }
  .task .opener, .tchip .opener { flex: 1; display: flex; align-items: center; gap: 5px; height: 100%; }
  .opener:focus-visible { outline: none; }
  .task.dragging, .tchip.dragging { box-shadow: var(--shadow); z-index: 4; position: relative; }
  .task.dragging { position: absolute; }
  .tt { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .task .time { flex: none; color: var(--muted); font-size: 11px; }
  .tchip.span-start { border-top-right-radius: 0; border-bottom-right-radius: 0; margin-right: -4px; }
  .tchip.span-middle { border-radius: 0; margin-inline: -4px; }
  .tchip.span-end { border-top-left-radius: 0; border-bottom-left-radius: 0; margin-left: -4px; }
  .tchip .mark { flex: none; color: var(--muted); font-size: 11px; }
  .tchip .when { flex: none; font-size: 11px; }
  .done .tt { color: var(--muted); text-decoration: line-through; }
  .overdue .time, .overdue .when { color: var(--danger); }
</style>
