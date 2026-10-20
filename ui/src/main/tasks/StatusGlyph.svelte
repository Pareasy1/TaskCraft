<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { TaskStatus } from '../../lib/types';
  import { nextStatus, STATUSES, STATUS_LABEL } from '../../lib/tasks';

  let {
    status,
    size = 18,
    onchange,
  }: { status: TaskStatus; size?: number; onchange?: (s: TaskStatus) => void } = $props();

  let menu = $state(false);
  const pick = (s: TaskStatus) => {
    menu = false;
    if (s !== status) onchange?.(s);
  };
</script>

{#snippet glyph(s: TaskStatus, px: number)}
  <svg class="g {s}" width={px} height={px} viewBox="0 0 20 20" aria-hidden="true">
    <circle class="ring" cx="10" cy="10" r="8.25" />
    <path class="half" d="M10 1.75 A8.25 8.25 0 0 1 10 18.25 Z" />
    <circle class="fill" cx="10" cy="10" r="8.25" />
    <path class="tick" d="M6.2 10.4 L8.8 13 L13.9 7.6" />
  </svg>
{/snippet}

{#if onchange}
  <span class="wrap">
    <button
      class="btn-glyph"
      title={t('status.title', { status: STATUS_LABEL[status], next: STATUS_LABEL[nextStatus(status)] })}
      aria-label={t('status.aria', { status: STATUS_LABEL[status], next: STATUS_LABEL[nextStatus(status)] })}
      onclick={(e) => { e.stopPropagation(); onchange(nextStatus(status)); }}
      oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); menu = !menu; }}
    >{@render glyph(status, size)}</button>
    {#if menu}
      <div class="menu" role="menu" tabindex="-1" onmouseleave={() => (menu = false)}>
        {#each STATUSES as s}
          <button role="menuitemradio" aria-checked={s === status} onclick={(e) => { e.stopPropagation(); pick(s); }}>
            {@render glyph(s, 14)}<span>{STATUS_LABEL[s]}</span>
          </button>
        {/each}
      </div>
    {/if}
  </span>
{:else}
  {@render glyph(status, size)}
{/if}

<style>
  .wrap { position: relative; display: inline-flex; }
  .btn-glyph { border: none; background: none; padding: 2px; margin: -2px; border-radius: 50%; display: inline-flex; }
  .btn-glyph:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .g { flex: none; display: block; }
  .ring { fill: none; stroke: var(--faint); stroke-width: 1.5; }
  .half, .fill { fill: var(--st, transparent); }
  .half { opacity: 0; }
  .fill { opacity: 0; transform-origin: 10px 10px; transform: scale(0.4); }
  .tick { fill: none; stroke: var(--bg); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; stroke-dasharray: 12; stroke-dashoffset: 12; }
  .inProgress { --st: var(--status-progress); }
  .inProgress .ring { stroke: var(--status-progress); }
  .inProgress .half { opacity: 1; }
  .done { --st: var(--status-done); }
  .done .ring { stroke: var(--status-done); }
  .done .fill { opacity: 1; transform: scale(1); }
  .done .tick { stroke-dashoffset: 0; }
  @media (prefers-reduced-motion: no-preference) {
    .half, .fill, .ring { transition: opacity 0.18s ease, transform 0.22s cubic-bezier(0.3, 1.4, 0.5, 1), stroke 0.18s; }
    .tick { transition: stroke-dashoffset 0.25s ease 0.12s; }
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: -4px;
    z-index: 30;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 4px;
    min-width: 140px;
  }
  .menu button { display: flex; align-items: center; gap: 8px; width: 100%; border: none; background: none; padding: 5px 8px; border-radius: 5px; text-align: left; }
  .menu button:hover, .menu button:focus-visible { background: var(--hover); outline: none; }
  .menu button[aria-checked='true'] span { font-weight: 600; }
</style>
