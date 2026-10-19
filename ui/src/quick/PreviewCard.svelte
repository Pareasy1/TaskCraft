<script lang="ts">
  import type { ParsedItem } from '../lib/types';
  import { describeParsed, describeRecurrence } from '../lib/format';
  import { Calendar, CircleCheck, MapPin, Users, Repeat, Bell, Link, Tag } from 'lucide-svelte';

  let { item, calendarColor }: { item: ParsedItem; calendarColor: string } = $props();

  let alerts = $derived(
    item.alerts.map((a) => (a.type === 'before' ? `${a.value >= 60 && a.value % 60 === 0 ? `${a.value / 60} h` : `${a.value} min`} before` : `at ${a.value.time?.slice(0, 5)}`)),
  );
</script>

<div class="card">
  <div class="title">
    <span class="bar" style="background:{calendarColor}"></span>
    {#if item.kind === 'task'}<CircleCheck size={15} />{:else}<Calendar size={15} />{/if}
    <strong>{item.title || (item.kind === 'task' ? 'New task' : 'New event')}</strong>
  </div>
  <div class="row"><span class="when">{describeParsed(item)}</span></div>
  {#if item.location}<div class="row"><MapPin size={13} />{item.location}</div>{/if}
  {#if item.attendees.length}<div class="row"><Users size={13} />{item.attendees.join(', ')}</div>{/if}
  {#if describeRecurrence(item)}<div class="row"><Repeat size={13} />{describeRecurrence(item)}</div>{/if}
  {#if alerts.length}<div class="row"><Bell size={13} />{alerts.join(', ')}</div>{/if}
  {#if item.calendar}<div class="row"><Tag size={13} />{item.calendar}</div>{/if}
  {#if item.url}<div class="row url"><Link size={13} />{item.url}</div>{/if}
</div>

<style>
  .card {
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 9px 12px 9px 14px;
    display: grid;
    gap: 4px;
    position: relative;
    overflow: hidden;
  }
  .bar { position: absolute; left: 0; top: 0; bottom: 0; width: 4px; }
  .title { display: flex; align-items: center; gap: 6px; font-size: 14px; }
  .row { display: flex; align-items: center; gap: 6px; color: var(--muted); }
  .when { color: var(--text); }
  .url { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
