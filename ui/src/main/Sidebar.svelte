<script lang="ts">
  import type { Calendar, Task } from '../lib/types';
  import MiniMonth from '../quick/MiniMonth.svelte';
  import TaskList from './TaskList.svelte';

  let {
    calendars,
    month = $bindable(new Date()),
    selected,
    marked,
    weekStart,
    taskSections,
    onselectday,
    ontogglecalendar,
    oncomplete,
    onopentask,
  }: {
    calendars: Calendar[];
    month?: Date;
    selected: Date;
    marked: Set<string>;
    weekStart: 0 | 1;
    taskSections: { name: string; tasks: Task[] }[];
    onselectday: (d: Date) => void;
    ontogglecalendar: (c: Calendar) => void;
    oncomplete: (t: Task) => void;
    onopentask: (t: Task) => void;
  } = $props();
</script>

<aside>
  <div class="traffic" data-tauri-drag-region></div>
  <MiniMonth bind:month {selected} {marked} {weekStart} onselect={(d) => d && onselectday(d)} />
  <div class="label">Calendars</div>
  {#each calendars as c (c.id)}
    <label class="cal">
      <input type="checkbox" checked={c.visible} style="accent-color:{c.color}" onchange={() => ontogglecalendar(c)} />
      <span>{c.name}</span>
    </label>
  {/each}
  <div class="tasks">
    <TaskList sections={taskSections} {oncomplete} onopen={onopentask} />
  </div>
</aside>

<style>
  aside {
    width: 236px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg-sunken);
    border-right: 1px solid var(--border);
    padding: 0 10px 10px;
    overflow: hidden;
  }
  .traffic { height: 46px; flex: none; }
  .label {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 14px 8px 4px;
  }
  .cal { display: flex; align-items: center; gap: 7px; padding: 3px 8px; border-radius: 6px; }
  .cal:hover { background: var(--hover); }
  .cal input { margin: 0; }
  .tasks { flex: 1; overflow-y: auto; margin-top: 6px; border-top: 1px solid var(--border); }
</style>
