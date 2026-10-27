<script lang="ts">
  import { t } from '../../lib/i18n';

  // "🔒 2": the task waits on that many open tasks. Names are optional (not always loaded).
  let { count, titles }: { count: number; titles?: string[] } = $props();
  const label = $derived(titles?.length ? t('dep.waitingFor', { names: titles.join(', ') }) : t('dep.openCount', { n: count }));
</script>

<span class="blocked" title={label} role="img" aria-label={label}><span aria-hidden="true">🔒 {count}</span></span>

<style>
  .blocked { display: inline-flex; align-items: center; gap: 3px; font-size: 11.5px; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; flex: none; }
</style>
