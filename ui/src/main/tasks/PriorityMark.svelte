<script lang="ts">
  // A small mark before a task's title; Medium (the default) shows nothing.
  import { t } from '../../lib/i18n';
  import type { Priority } from '../../lib/types';

  let { priority }: { priority: Priority } = $props();
  const GLYPH: Record<Priority, string> = { urgent: '!!', important: '!', medium: '', low: '↓' };
</script>

{#if priority !== 'medium'}
  <span class="pm {priority}" role="img" aria-label={t(`priority.${priority}`)} title={t(`priority.${priority}`)}>{GLYPH[priority]}</span>
{/if}

<style>
  .pm { flex: none; font-weight: 800; font-size: 12px; line-height: 1.5; letter-spacing: -1px; min-width: 8px; text-align: center; }
  .urgent { color: var(--danger); }
  .important { color: var(--warning); }
  .low { color: var(--muted); letter-spacing: 0; font-weight: 600; }
</style>
