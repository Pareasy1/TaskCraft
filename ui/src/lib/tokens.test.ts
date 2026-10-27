// Contrast checks for the colour tokens in styles.css (WCAG 2.1 AA).
/// <reference types="node" />
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');


function block(selector: RegExp): Record<string, string> {
  const m = css.match(selector);
  if (!m) throw new Error(`no block for ${selector}`);
  return Object.fromEntries([...m[1].matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6})/g)].map((x) => [x[1], x[2]]));
}
const light = block(/^:root \{([^}]*)\}/m);
const dark = { ...light, ...block(/@media \(prefers-color-scheme: dark\) \{\s*:root \{([^}]*)\}/) };

function lum(hex: string) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
const ratio = (a: string, b: string) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

const TEXT = ['text', 'muted', 'faint', 'accent', 'danger', 'warning', 'label-date', 'label-time', 'label-duration', 'label-location',
  'label-attendees', 'label-recurrence', 'label-alert', 'label-calendar', 'label-url', 'label-prefix', 'label-tag', 'today'];
const GRAPHIC = ['status-progress', 'status-done', 'chart-neutral'];

for (const [mode, t] of [['light', light], ['dark', dark]] as const) {
  describe(`${mode} mode contrast`, () => {
    const surfaces = mode === 'light' ? ['#ffffff', t['bg-sunken']] : [t.bg, t['bg-sunken'], t['surface-strong']];
    for (const name of TEXT) {
      it(`${name} text ≥ 4.5:1 on every surface`, () => {
        for (const s of surfaces) expect(ratio(t[name], s), `${name} ${t[name]} on ${s}`).toBeGreaterThanOrEqual(4.5);
      });
    }
    for (const name of GRAPHIC) {
      it(`${name} graphic ≥ 3:1 on every surface`, () => {
        for (const s of surfaces) expect(ratio(t[name], s), `${name} on ${s}`).toBeGreaterThanOrEqual(3);
      });
    }
    it('today badge text ≥ 4.5:1', () => {
      expect(ratio(t['on-today'], t.today)).toBeGreaterThanOrEqual(4.5);
    });
    it('primary button text ≥ 4.5:1', () => {
      const fg = t['on-accent'];
      expect(ratio(fg, t.accent)).toBeGreaterThanOrEqual(4.5);
    });
  });
}
