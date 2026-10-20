// House style rules that apply across every component.
/// <reference types="node" />
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const root = new URL('..', import.meta.url).pathname;
function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? files(p) : /\.(svelte|css)$/.test(f) ? [p] : [];
  });
}

describe('house style', () => {
  it('headings and labels use sentence case, never ALL CAPS', () => {
    const offenders = files(root).filter((f) => /text-transform:\s*uppercase/.test(readFileSync(f, 'utf8')));
    expect(offenders.map((f) => f.slice(root.length))).toEqual([]);
  });
});
