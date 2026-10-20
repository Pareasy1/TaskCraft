<script lang="ts">
  let {
    name,
    color,
    active = false,
    count,
    onclick,
  }: { name: string; color: string; active?: boolean; count?: number; onclick?: (e: MouseEvent) => void } = $props();
</script>

{#if onclick}
  <button class="chip" class:active style="--c:{color}" aria-pressed={active} onclick={(e) => { e.stopPropagation(); onclick(e); }}>
    <span class="hash">#</span>{name}{#if count != null}<span class="n">{count}</span>{/if}
  </button>
{:else}
  <span class="chip" style="--c:{color}"><span class="hash">#</span>{name}</span>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: baseline;
    gap: 1px;
    border: none;
    background: none;
    padding: 1px 4px;
    margin: 0 -2px;
    border-radius: 5px;
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .hash { color: var(--c); font-weight: 700; }
  button.chip:hover { background: var(--hover); color: var(--text); }
  button.chip:focus-visible { outline: 2px solid var(--accent); }
  .chip.active { background: color-mix(in srgb, var(--c) 16%, transparent); color: var(--text); }
  .n { margin-left: 5px; font-size: 11px; color: var(--faint); font-variant-numeric: tabular-nums; }
</style>
