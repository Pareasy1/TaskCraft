<script lang="ts">
  // A pencil sketch from the app icon's world: two ticked boxes and an empty one
  // on a slightly crooked sheet. Decorative; the empty-state text carries the meaning.
  let { size = 56 }: { size?: number } = $props();
</script>

<svg class="doodle" width={size} height={size * 0.82} viewBox="0 0 64 52" aria-hidden="true">
  <path d="M10.5 4.2 C24 3.4 40 3.1 53.6 4.4 C54.2 18 54.6 33 53.9 47.3 C40 48.4 24.5 48.1 10.2 47.1 C9.6 33 9.8 18.4 10.5 4.2 Z" />
  <path d="M17 12.4 h6.6 v6.4 h-6.8 z" />
  <path class="ink" d="M17.8 15.6 C18.6 16.3 19.4 17.2 20.2 18.2 C21.5 15.8 23.2 13.6 25.6 11.4" />
  <path d="M29 15.4 C35 15 41.5 15.1 47 15.6" />
  <path d="M17 24.6 h6.6 v6.4 h-6.8 z" />
  <path class="ink" d="M17.8 27.8 C18.6 28.5 19.4 29.4 20.2 30.4 C21.5 28 23.2 25.8 25.6 23.6" />
  <path d="M29 27.6 C34 27.2 38.5 27.3 43 27.8" />
  <path d="M17 36.8 h6.6 v6.4 h-6.8 z" />
  <path d="M29 39.9 C33.5 39.6 37.5 39.7 41 40.1" />
</svg>

<style>
  .doodle { display: block; margin: 0 auto 10px; color: var(--muted); }
  path { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .ink { stroke: var(--accent); stroke-width: 1.9; }
</style>
