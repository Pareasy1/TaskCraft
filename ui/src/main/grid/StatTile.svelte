<script lang="ts">
  // One headline number in the charts panel.
  import { fmtCount } from '../../lib/charts';
  let { label, value, alert = false }: { label: string; value: number; alert?: boolean } = $props();
</script>

<div class="tile">
  <span class="label">{label}</span>
  <span class="value" class:alert={alert && value > 0}>{fmtCount(value)}</span>
</div>

<style>
  .tile { display: flex; flex-direction: column; gap: 2px; padding: 8px 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); min-width: 0; }
  .label { font-size: 12px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .value { font-size: 22px; font-weight: 600; color: var(--text); line-height: 1.2; }
  .value.alert { color: var(--danger); }
</style>
