<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { Settings as Gear, AppWindow, Power, X } from 'lucide-svelte';
  import { api, errorMessage, onItemsChanged } from '../lib/api';
  import type { Calendar, ItemRef, Occurrence, ParsedItem, Settings, Task, TaskStatus } from '../lib/types';
  import { STATUS_LABEL } from '../lib/tasks';
  import { monthGrid, addDays, startOfDay } from '../lib/layout';
  import { ymd } from '../lib/format';
  import HighlightInput from './HighlightInput.svelte';
  import PreviewCard from './PreviewCard.svelte';
  import MiniMonth from './MiniMonth.svelte';
  import Upcoming from './Upcoming.svelte';

  const win = getCurrentWindow();
  let text = $state('');
  let parsed = $state<ParsedItem | null>(null);
  let error = $state('');
  let flash = $state('');
  let flashTimer: ReturnType<typeof setTimeout> | undefined;
  function showFlash(m: string) {
    clearTimeout(flashTimer);
    flash = m;
    flashTimer = setTimeout(() => (flash = ''), 2200);
  }
  let input = $state<HTMLInputElement | undefined>();
  let month = $state(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
  let selected = $state<Date | null>(null);
  let occurrences = $state<Occurrence[]>([]);
  let tasks = $state<Task[]>([]);
  let upcomingTasks = $state<Task[]>([]);
  let calendars = $state<Calendar[]>([]);
  let settings = $state<Settings | null>(null);

  let weekStart = $derived<0 | 1>(settings?.weekStart === 'Sun' ? 0 : 1);
  let grid = $derived(monthGrid(month.getFullYear(), month.getMonth(), weekStart));
  let marked = $derived(new Set(occurrences.map((o) => ymd(new Date(o.start)))));
  let listFrom = $derived(selected ?? new Date());
  let listDays = $derived(selected ? 1 : 14);
  let previewColor = $derived.by(() => {
    const byName = parsed?.calendar && calendars.find((c) => c.name === parsed!.calendar);
    const def = calendars.find((c) => c.id === settings?.defaultCalendarId) ?? calendars[0];
    return (byName || def)?.color ?? 'var(--accent)';
  });

  async function refresh() {
    try {
      const start = grid[0] < startOfDay(new Date()) ? grid[0] : startOfDay(new Date());
      const end = addDays(grid[41] > addDays(new Date(), 14) ? grid[41] : addDays(new Date(), 14), 1);
      const [occ, overdue, today, upcoming, cals, s] = await Promise.all([
        api.listRange(start, end), api.listTasks('overdue'), api.listTasks('today'), api.listTasks('upcoming'), api.listCalendars(), api.getSettings(),
      ]);
      upcomingTasks = upcoming;
      occurrences = occ;
      tasks = [...overdue, ...today];
      calendars = cals;
      settings = s;
    } catch (e) {
      error = errorMessage(e);
    }
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const t = text;
    clearTimeout(timer);
    if (!t.trim()) {
      parsed = null;
      return;
    }
    timer = setTimeout(async () => {
      try {
        parsed = await api.parsePreview(t);
      } catch (e) {
        error = errorMessage(e);
      }
    }, 50);
  });

  $effect(() => {
    void grid;
    refresh();
  });

  async function submit(open: boolean) {
    if (!text.trim()) return;
    try {
      const ref: ItemRef = await api.createFromText(text);
      showFlash(`Added ${parsed?.kind === 'task' ? 'task' : 'event'} “${parsed?.title || text}”`);
      text = '';
      parsed = null;
      error = '';
      if (open) {
        await invoke('open_item', { item: ref });
        await win.hide();
      }
    } catch (e) {
      error = errorMessage(e);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      submit(e.metaKey);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (text) text = '';
      else win.hide();
    }
  }

  async function setStatus(t: Task, s: TaskStatus) {
    try {
      await api.setTaskStatus(t.id, s);
      showFlash(s === 'done' ? `Completed “${t.title}”` : `Moved “${t.title}” to ${STATUS_LABEL[s]}`);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function openItem(kind: 'event' | 'task', id: number) {
    await invoke('open_item', { item: { kind, id } });
    await win.hide();
  }

  onMount(() => {
    const un = [onItemsChanged(refresh), win.onFocusChanged(({ payload }) => {
      if (payload) {
        refresh();
        input?.focus();
        input?.select();
      }
    })];
    input?.focus();
    return () => un.forEach((p) => p.then((f) => f()));
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.activeElement !== input) win.hide();
  });
</script>

<main class="pop">
  <HighlightInput
    label="New event or task"
    bind:value={text}
    bind:input
    spans={parsed?.spans ?? []}
    placeholder="Lunch with Sam tomorrow at 1 · todo buy milk friday"
    {onkeydown}
  />
  {#if parsed}
    <PreviewCard item={parsed} calendarColor={previewColor} />
    <div class="hint">↩ Add · ⌘↩ Add &amp; open · esc Clear</div>
  {/if}
  <div role="status" aria-live="polite">{#if flash}<div class="flash">{flash}</div>{/if}</div>
  {#if error}<div class="banner" role="alert"><span>{error}</span><button class="icon-btn" aria-label="Dismiss message" onclick={() => (error = '')}><X size={14} aria-hidden="true" /></button></div>{/if}

  <MiniMonth bind:month {selected} {marked} {weekStart} onselect={(d) => (selected = d)} />
  <div class="scroll">
    <Upcoming {occurrences} pinned={selected ? [] : tasks} dated={selected ? [...tasks, ...upcomingTasks] : upcomingTasks} from={listFrom} days={listDays} onstatus={setStatus} onopen={openItem} />
  </div>
  <footer>
    <button class="icon-btn" title="Open TaskCraft" aria-label="Open TaskCraft window" onclick={() => api.openMain()}><AppWindow size={16} /></button>
    <button class="icon-btn" title="Settings" aria-label="Settings" onclick={() => api.openSettings()}><Gear size={16} /></button>
    <span class="spacer"></span>
    <button class="icon-btn" title="Quit TaskCraft" aria-label="Quit TaskCraft" onclick={() => invoke('quit_app')}><Power size={16} /></button>
  </footer>
</main>

<style>
  .pop {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 12px 6px;
    background: var(--surface);
    border-radius: 12px;
  }
  .hint { font-size: 12px; color: var(--faint); text-align: right; margin-top: -4px; }
  .flash {
    font-size: 12px;
    color: var(--label-time);
    background: color-mix(in srgb, var(--label-time) 12%, transparent);
    border-radius: 7px;
    padding: 5px 9px;
  }
  .scroll { flex: 1; overflow-y: auto; margin: 0 -6px; padding: 0 2px; border-top: 1px solid var(--border); }
  footer { display: flex; align-items: center; gap: 2px; border-top: 1px solid var(--border); padding-top: 5px; }
  .spacer { flex: 1; }
</style>
