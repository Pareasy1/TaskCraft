<script lang="ts">
  import { t } from '../../lib/i18n';
  import { arrowConflict, arrowPath } from '../../lib/dependencies';
  import { barSpan, dayToX, DAY_WIDTH, ROW_H, type TimelineRow, type Zoom } from '../../lib/timeline';
  import type { Dependency, Task } from '../../lib/types';

  // Dependency arrows over the bar lanes (SVG, same coordinates as the rows: row i's middle is at
  // i × ROW_H + ROW_H / 2). An arrow is drawn only when both tasks have a visible, dated row.
  // Each arrow is a focusable button: focus (or a click) selects it, Delete / Backspace removes it.
  let { rows, deps, range, zoom, nameW, top, selected, onselect, onremove }: {
    rows: TimelineRow<Task>[];
    deps: Dependency[];
    range: { from: number; to: number };
    zoom: Zoom;
    nameW: number;
    /** Where the rows start in the canvas (under the header). */
    top: number;
    /** The selected arrow, as `taskId:dependsOn`. */
    selected: string | null;
    onselect: (d: Dependency | null) => void;
    onremove: (d: Dependency) => void;
  } = $props();

  const keyOf = (d: Dependency) => `${d.taskId}:${d.dependsOn}`;
  const HEAD = 5;

  let arrows = $derived.by(() => {
    const at = new Map<number, { i: number; task: Task; span: { from: number; to: number } }>();
    rows.forEach((r, i) => {
      if (r.kind !== 'task' || r.dimmed) return; // a dimmed row is only context for a filtered child
      const span = barSpan(r.task);
      if (span) at.set(r.task.id, { i, task: r.task, span });
    });
    const w = DAY_WIDTH[zoom];
    const mid = ROW_H / 2;
    return deps.flatMap((d) => {
      const a = at.get(d.dependsOn), b = at.get(d.taskId);
      if (!a || !b) return [];
      const from = { x: nameW + dayToX(a.span.to, range, zoom) + w, y: a.i * ROW_H + mid };
      const to = { x: nameW + dayToX(b.span.from, range, zoom), y: b.i * ROW_H + mid };
      return [{
        dep: d,
        key: keyOf(d),
        path: arrowPath(from, to),
        head: `M${to.x} ${to.y}l${-HEAD - 2} ${-HEAD + 1}v${2 * HEAD - 2}z`,
        warn: arrowConflict(a.span, b.span),
        label: t('dep.arrow', { a: a.task.title, b: b.task.title }),
      }];
    });
  });
  let height = $derived(rows.length * ROW_H);

  function keys(e: KeyboardEvent, d: Dependency) {
    if (e.key === 'Delete' || e.key === 'Backspace') onremove(d);
    else if (e.key === 'Escape') (e.currentTarget as SVGElement).blur();
    else if (e.key === 'Enter' || e.key === ' ') onselect(d);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault(); // focus by hand: WebKit doesn't focus SVG on click
    (e.currentTarget as SVGElement).focus({ preventScroll: true });
  }
  function blur(d: Dependency) {
    if (selected === keyOf(d)) onselect(null);
  }
</script>

{#if arrows.length}
  <p id="arrow-help" class="sr">{t('dep.arrowHelp')}</p>
  <svg class="arrows" style="top:{top}px" width={nameW + (range.to - range.from + 1) * DAY_WIDTH[zoom]} {height} aria-label={t('dep.section')} role="group">
    {#each arrows as a (a.key)}
      <g class="arrow" class:warn={a.warn} class:selected={selected === a.key} role="button" tabindex="0"
        aria-label={a.label} aria-describedby="arrow-help" aria-pressed={selected === a.key}
        onfocus={() => onselect(a.dep)} onblur={() => blur(a.dep)} onkeydown={(e) => keys(e, a.dep)} onpointerdown={down}>
        <path class="hit" d={a.path} />
        <path class="line" d={a.path} />
        <path class="head" d={a.head} />
      </g>
    {/each}
  </svg>
{/if}

<style>
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  /* Above the bars, under the connector dots, the sticky names and the header. */
  .arrows { position: absolute; left: 0; z-index: 1; overflow: visible; pointer-events: none; }
  .arrow { cursor: pointer; outline: none; color: var(--muted); }
  .arrow.warn { color: var(--warning); }
  .arrow.selected, .arrow:focus-visible { color: var(--accent); }
  .hit { fill: none; stroke: transparent; stroke-width: 10; pointer-events: stroke; }
  .line { fill: none; stroke: currentColor; stroke-width: 1.25; stroke-linejoin: round; }
  .head { fill: currentColor; stroke: currentColor; stroke-width: 1; stroke-linejoin: round; pointer-events: all; }
  .arrow:hover .line { stroke-width: 1.75; }
  .arrow.selected .line { stroke-width: 2.5; }
  .arrow.selected .head { stroke-width: 2; }
</style>
