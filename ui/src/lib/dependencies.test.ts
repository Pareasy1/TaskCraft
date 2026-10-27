import { describe, expect, it } from 'vitest';
import { blockedPrompt, blockers, blocking, pickerExclude, pickerMatches, wouldCycle } from './dependencies';
import type { Dependency, Task } from './types';

const mk = (id: number, done = false): Task => ({ id, title: `T${id}`, completedAt: done ? '2026-10-23T10:00:00Z' : null, status: done ? 'done' : 'todo' }) as Task;
const tasks = [mk(1), mk(2), mk(3, true), mk(4)];
const deps: Dependency[] = [{ taskId: 1, dependsOn: 2 }, { taskId: 1, dependsOn: 3 }, { taskId: 4, dependsOn: 1 }];

describe('dependencies', () => {
  it('blockers lists open tasks only', () => expect(blockers(1, deps, tasks).map((x) => x.id)).toEqual([2]));
  it('blocking lists dependants', () => expect(blocking(1, deps, tasks).map((x) => x.id)).toEqual([4]));
  it('wouldCycle: self, direct, long chain, unrelated', () => {
    expect(wouldCycle(1, 1, deps)).toBe(true);
    expect(wouldCycle(2, 1, deps)).toBe(true); // 1 depends on 2
    expect(wouldCycle(2, 4, deps)).toBe(true); // 4 -> 1 -> 2
    expect(wouldCycle(4, 2, deps)).toBe(false);
    expect(wouldCycle(3, 2, [])).toBe(false);
  });
  it('blockedPrompt one vs many', () => {
    expect(blockedPrompt([{ title: 'A' }])).toBe('Depends on “A”, still open. Continue?');
    expect(blockedPrompt([{ title: 'A' }, { title: 'B' }])).toBe('Depends on 2 open tasks. Continue?');
  });
  it('pickerMatches ignores case and accents; empty matches all', () => {
    expect(pickerMatches('Città da visitare', 'CITTA')).toBe(true);
    expect(pickerMatches('Plan café', 'cafe')).toBe(true);
    expect(pickerMatches('Plan', 'xyz')).toBe(false);
    expect(pickerMatches('Anything', '  ')).toBe(true);
  });
  it('pickerExclude hides self, direct blockers and transitive cycles', () => {
    const g: Dependency[] = [{ taskId: 4, dependsOn: 1 }, { taskId: 1, dependsOn: 2 }];
    expect([...pickerExclude(2, g)].sort()).toEqual([1, 2, 4]); // 4 -> 1 -> 2 would close a loop
    expect([...pickerExclude(4, g)].sort()).toEqual([1, 4]); // 1 already; 2 is fine
  });
});
