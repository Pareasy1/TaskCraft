<script lang="ts">
  import type { Span } from '../lib/types';
  import { byteToIndex } from '../lib/format';

  let {
    value = $bindable(''),
    spans = [],
    placeholder = '',
    label = '',
    onkeydown,
    input = $bindable<HTMLInputElement | undefined>(),
  }: {
    value?: string;
    spans?: Span[];
    placeholder?: string;
    label?: string;
    onkeydown?: (e: KeyboardEvent) => void;
    input?: HTMLInputElement;
  } = $props();

  const LABEL_VAR: Record<string, string> = {
    date: '--label-date', time: '--label-time', duration: '--label-duration', location: '--label-location',
    attendees: '--label-attendees', recurrence: '--label-recurrence', alert: '--label-alert',
    calendar: '--label-calendar', url: '--label-url', taskPrefix: '--label-prefix', tag: '--label-tag', plan: '--label-calendar',
    priority: '--label-alert', template: '--label-calendar',
  };

  // Split the text into plain and highlighted segments.
  let segments = $derived.by(() => {
    const out: { text: string; color: string | null }[] = [];
    let pos = 0;
    for (const s of spans) {
      const color = LABEL_VAR[s.label];
      if (!color) continue;
      const a = byteToIndex(value, s.start);
      const b = byteToIndex(value, s.end);
      if (a < pos || b > value.length) continue;
      if (a > pos) out.push({ text: value.slice(pos, a), color: null });
      out.push({ text: value.slice(a, b), color });
      pos = b;
    }
    if (pos < value.length) out.push({ text: value.slice(pos), color: null });
    return out;
  });

  let mirror: HTMLDivElement | undefined = $state();
  const sync = () => {
    if (mirror && input) mirror.scrollLeft = input.scrollLeft;
  };
</script>

<div class="wrap">
  <div class="mirror" bind:this={mirror} aria-hidden="true">
    {#each segments as seg}
      {#if seg.color}
        <span class="hl" style="--c: var({seg.color})">{seg.text}</span>
      {:else}<span>{seg.text}</span>{/if}
    {/each}
  </div>
  <input
    bind:this={input}
    bind:value
    {placeholder}
    aria-label={label || placeholder}
    {onkeydown}
    oninput={sync}
    onscroll={sync}
    spellcheck="false"
    autocomplete="off"
    autocorrect="off"
  />
</div>

<style>
  .wrap {
    position: relative;
    font-size: 16px;
    border-radius: 9px;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    box-shadow: inset 0 1px 1px rgba(0, 0, 0, 0.03);
  }
  .wrap:focus-within { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
  .mirror, input {
    font: inherit;
    padding: 9px 12px;
    white-space: pre;
    letter-spacing: normal;
  }
  .mirror {
    position: absolute;
    inset: 0;
    overflow: hidden;
    color: var(--text);
    pointer-events: none;
  }
  .hl {
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
    border-radius: 4px;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--c) 14%, transparent);
  }
  input {
    position: relative;
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    color: transparent;
    caret-color: var(--text);
  }
  input::placeholder { color: var(--faint); }
  input::selection { background: var(--accent-soft); color: transparent; }
</style>
