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

  it('dates and times are formatted in the app language (lib/format.ts), not the raw system locale', () => {
    const offenders = files(root).filter((f) => /toLocale\w*String\(undefined/.test(readFileSync(f, 'utf8')));
    expect(offenders.map((f) => f.slice(root.length))).toEqual([]);
  });
});
