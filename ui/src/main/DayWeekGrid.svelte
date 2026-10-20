<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Occurrence } from '../lib/types';
  import { addDays, layoutDay, nudgeEvent, sameDay, snap15 } from '../lib/layout';
  import { fmtTime } from '../lib/format';

  const HOUR = 48; // px per hour
  const PX_MIN = HOUR / 60;

  let {
    days,
    occurrences,
    onselect,
    oncreate,
    onmove,
  }: {
    days: Date[];
    occurrences: Occurrence[];
    onselect: (o: Occurrence) => void;
    oncreate: (start: Date) => void;
    onmove: (o: Occurrence, start: Date, end: Date) => void;
  } = $props();

  let now = $state(new Date());
  let scroller: HTMLDivElement | undefined = $state();
  let colsEl: HTMLDivElement | undefined = $state();

  const key = (o: Occurrence) => `${o.eventId}@${o.start}`;
  const isAllDay = (o: Occurrence) => o.allDay || new Date(o.end).getTime() - new Date(o.start).getTime() >= 86_400_000;

  let allDay = $derived(
    days.map((d) => occurrences.filter((o) => isAllDay(o) && new Date(o.start) < addDays(d, 1) && new Date(o.end) > d)),
  );
  let timed = $derived(
    days.map((d) => {
      const items = occurrences.filter((o) => !isAllDay(o) && new Date(o.start) < addDays(d, 1) && new Date(o.end) > d);
      const pos = layoutDay(items.map((o) => ({ id: key(o), start: new Date(o.start), end: new Date(o.end) })), d);
      return pos.map((p) => ({ p, o: items.find((o) => key(o) === p.id)! }));
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
    if (!d.moved) return onselect(d.o);
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
  const hourLabel = (h: number) => (h === 0 ? '' : fmtTime(new Date(2026, 0, 1, h)));
  const today = new Date();
</script>

<div class="grid" style="--n:{days.length}">
  <div class="heads">
    <div class="gutter"></div>
    {#each days as d}
      <div class="head" class:today={sameDay(d, today)}>
        <span class="wd">{d.toLocaleDateString(undefined, { weekday: 'short' })}</span>
        <span class="num">{d.getDate()}</span>
      </div>
    {/each}
  </div>
  <div class="allday">
    <div class="gutter small">all-day</div>
    {#each allDay as items}
      <div class="adcol">
        {#each items as o (key(o))}
          <button class="chip" style="--c:{o.color}" onclick={() => onselect(o)}>{o.title}</button>
        {/each}
      </div>
    {/each}
  </div>
  <div class="scroll" bind:this={scroller}>
    <div class="body">
      <div class="hours">
        {#each hours as h}<div class="hour"><span>{hourLabel(h)}</span></div>{/each}
      </div>
      <div class="cols" role="grid" tabindex="-1" bind:this={colsEl} onpointermove={moveDrag} onpointerup={endDrag}>
        {#each days as d, i}
          <div class="col" role="gridcell" tabindex="-1" ondblclick={(e) => dblclick(e, d)}>
            {#each hours as _}<div class="slot"></div>{/each}
            {#if sameDay(d, now)}
              <div class="now" style="top:{(now.getHours() * 60 + now.getMinutes()) * PX_MIN}px"></div>
            {/if}
            {#each timed[i] as { p, o } (p.id)}
              {@const v = preview(o, p.top, p.height)}
              <div
                class="ev"
                class:dragging={drag && key(drag.o) === p.id}
                style="--c:{o.color}; top:{v.top * PX_MIN}px; height:{v.height * PX_MIN - 2}px;
                       left:calc({(p.col / p.cols) * 100}% + {v.shift * 100}%); width:calc({100 / p.cols}% - 3px)"
                role="button"
                tabindex="0"
                data-event={o.eventId}
                aria-label="{o.title}, {fmtTime(new Date(o.start))} to {fmtTime(new Date(o.end))}. Enter to open, arrow keys to move, Shift plus up or down to change length."
                onpointerdown={(e) => begin(e, o, 'move')}
                onkeydown={(e) => keydown(e, o)}
              >
                <span class="t">{o.title}</span>
                {#if v.height >= 30}<span class="time">{fmtTime(new Date(o.start))}</span>{/if}
                <div class="resize" role="presentation" onpointerdown={(e) => begin(e, o, 'resize')}></div>
              </div>
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
  .gutter.small { font-size: 10px; color: var(--faint); text-align: right; padding: 4px 6px 0 0; }
  .allday { border-bottom: 1px solid var(--border); min-height: 26px; }
  .adcol { border-left: 1px solid var(--border); padding: 2px; display: grid; gap: 2px; align-content: start; }
  .chip {
    border: none;
    text-align: left;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    background: color-mix(in srgb, var(--c) 22%, var(--bg));
    color: color-mix(in srgb, var(--c) 70%, var(--text));
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
    font-size: 10px;
    color: var(--faint);
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
    color: color-mix(in srgb, var(--c) 65%, var(--text));
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
  .time { font-size: 11px; opacity: 0.8; }
  .resize { position: absolute; left: 0; right: 0; bottom: 0; height: 6px; cursor: ns-resize; }
</style>
