<script lang="ts">
  import { modLabel } from '../lib/platform';
  import { t } from '../lib/i18n';
  import type { Calendar, Task, TaskLabel, TaskStatus } from '../lib/types';
  import { Settings as SettingsIcon, Tags } from 'lucide-svelte';
  import LabelManager from './tasks/LabelManager.svelte';
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
    onstatus,
    onopentask,
    tasksMode = false,
    labels = [],
    tasks = [],
    activeLabels = $bindable<number[]>([]),
    onerror = () => {},
    onopensettings = () => {},
  }: {
    calendars: Calendar[];
    month?: Date;
    selected: Date;
    marked: Set<string>;
    weekStart: 0 | 1;
    taskSections: { key: string; name: string; tasks: Task[] }[];
    onselectday: (d: Date) => void;
    ontogglecalendar: (c: Calendar) => void;
    onstatus: (t: Task, s: TaskStatus) => void;
    onopentask: (t: Task) => void;
    /** On the Tasks view the page lists the tasks; the sidebar offers label filters instead. */
    tasksMode?: boolean;
    labels?: TaskLabel[];
    tasks?: Task[];
    activeLabels?: number[];
    onerror?: (m: string) => void;
    onopensettings?: () => void;
  } = $props();

  let managing = $state(false);
  let usage = $derived(new Map(labels.map((l) => [l.id, tasks.filter((x) => !x.completedAt && x.labels.some((y) => y.id === l.id)).length])));
  const toggle = (id: number) => (activeLabels = activeLabels.includes(id) ? activeLabels.filter((x) => x !== id) : [...activeLabels, id]);
</script>

<aside>
  <div class="traffic" data-tauri-drag-region></div>
  <MiniMonth bind:month {selected} {marked} {weekStart} onselect={(d) => d && onselectday(d)} />
  <div class="label">{t('main.calendars')}</div>
  {#each calendars as c (c.id)}
    <label class="cal">
      <input type="checkbox" checked={c.visible} style="accent-color:{c.color}" onchange={() => ontogglecalendar(c)} />
      <span>{c.name}</span>
    </label>
  {/each}
  {#if tasksMode}
    <div class="label row">
      <span>{t('labels.title')}</span>
      <span class="mgr">
        {#if activeLabels.length}<button class="clear" onclick={() => (activeLabels = [])}>{t('tasks.clear')}</button>{/if}
        <button class="icon-btn" title={t('tasks.manageLabels')} aria-label={t('tasks.manageLabels')} aria-expanded={managing}
          onclick={() => (managing = !managing)}><Tags size={14} aria-hidden="true" /></button>
        {#if managing}<LabelManager {labels} onclose={() => (managing = false)} {onerror} />{/if}
      </span>
    </div>
    <div class="lbls" role="group" aria-label={t('labels.title')}>
      {#each labels as l (l.id)}
        <button class="lbl" aria-pressed={activeLabels.includes(l.id)} class:on={activeLabels.includes(l.id)} style="--c:{l.color}" onclick={() => toggle(l.id)}>
          <span class="hash" aria-hidden="true">#</span><span class="nm">{l.name}</span><span class="n">{usage.get(l.id) ?? 0}</span>
        </button>
      {:else}
        <p class="none">{t('labels.empty')}</p>
      {/each}
    </div>
  {:else}
    <div class="tasks">
      <TaskList sections={taskSections} {onstatus} onopen={onopentask} />
    </div>
  {/if}
  <div class="foot">
    <button class="icon-btn" aria-label={t('settings.title')} title="{t('settings.title')} ({modLabel(',')})" onclick={onopensettings}>
      <SettingsIcon size={16} aria-hidden="true" />
    </button>
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
  /* Only macOS overlays its window buttons on the sidebar. */
  :global(html:not([data-platform='mac'])) .traffic { height: 10px; }
  /* Settings sit in the bottom-left corner, under whatever the sidebar lists. */
  .foot { margin-top: auto; padding-top: 8px; display: flex; flex: none; }
  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    padding: 14px 8px 4px;
  }
  .row { display: flex; align-items: center; justify-content: space-between; }
  .mgr { position: relative; display: flex; align-items: center; gap: 4px; }
  /* The sidebar is narrower than the popover: open it towards the content. */
  .mgr :global(.pop) { right: auto; left: 0; }
  .clear { border: none; background: none; color: var(--accent); font-size: 12px; padding: 2px 4px; }
  .lbls { display: flex; flex-direction: column; gap: 1px; overflow-y: auto; }
  .lbl { display: flex; align-items: baseline; gap: 2px; border: none; background: none; text-align: left; padding: 4px 8px; border-radius: 6px; color: var(--text); }
  .lbl:hover { background: var(--hover); }
  .lbl.on { background: color-mix(in srgb, var(--c) 16%, transparent); }
  .lbl .hash { color: var(--c); font-weight: 700; }
  .lbl .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lbl .n { color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  .none { color: var(--muted); font-size: 12px; margin: 4px 8px; }
  .cal { display: flex; align-items: center; gap: 7px; padding: 3px 8px; border-radius: 6px; }
  .cal:hover { background: var(--hover); }
  .cal input { margin: 0; }
  .tasks { flex: 1; overflow-y: auto; margin-top: 6px; border-top: 1px solid var(--border); }
</style>
