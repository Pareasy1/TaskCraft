// House style rules that apply across every component.
/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const root = new URL('..', import.meta.url).pathname;
function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? files(p) : /\.(svelte|css|ts)$/.test(f) && !f.endsWith('.test.ts') ? [p] : [];
  });
}

describe('house style', () => {
  it('headings and labels use sentence case, never ALL CAPS', () => {
    const offenders = files(root).filter((f) => /text-transform:\s*uppercase/.test(readFileSync(f, 'utf8')));
    expect(offenders.map((f) => f.slice(root.length))).toEqual([]);
  });

  // WebKit (the macOS app's webview) rejects a drop when the drag allows only
  // "move" but the drop target leaves dropEffect at its default.
  it('drags limited to move set a matching dropEffect on their drop targets', () => {
    const offenders = files(root).filter((f) => {
      const src = readFileSync(f, 'utf8');
      return /effectAllowed\s*=\s*'move'/.test(src) && !/dropEffect\s*=\s*'move'/.test(src);
    });
    expect(offenders.map((f) => f.slice(root.length))).toEqual([]);
  });

  // The Planner's drags use pointer events: HTML5 drag-and-drop has misbehaved in WebKit before.
  it('the Planner timeline uses no HTML5 drag-and-drop', () => {
    const offenders = files(join(root, 'main/planner')).filter((f) => /\bdraggable\b|ondragstart/.test(readFileSync(f, 'utf8')));
    expect(offenders.map((f) => f.slice(root.length))).toEqual([]);
  });

  it('dates and times are formatted in the app language (lib/format.ts), not the raw system locale', () => {
    const offenders = files(root).filter((f) => /toLocale\w*String\(undefined/.test(readFileSync(f, 'utf8')));
    expect(offenders.map((f) => f.slice(root.length))).toEqual([]);
  });
});
