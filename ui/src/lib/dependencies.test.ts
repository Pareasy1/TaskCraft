import { describe, expect, it } from 'vitest';
import { arrowConflict, arrowPath, blockedPrompt, blockers, blocking, pickerExclude, pickerMatches, wouldCycle } from './dependencies';
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
  it('arrowPath forward: out 8, down to the target row, in', () => {
    expect(arrowPath({ x: 100, y: 16 }, { x: 200, y: 80 })).toBe('M100 16H108V80H200');
    expect(arrowPath({ x: 100, y: 80 }, { x: 116, y: 16 })).toBe('M100 80H108V16H116'); // exactly 16 apart, upwards
  });
  it('arrowPath backward routes around, left of the target', () => {
    // Target starts before the source ends: out 8, half a row towards it, back past to.x - 8, then in.
    expect(arrowPath({ x: 200, y: 16 }, { x: 100, y: 80 })).toBe('M200 16H208V32H92V80H100');
    expect(arrowPath({ x: 200, y: 80 }, { x: 190, y: 16 })).toBe('M200 80H208V64H182V16H190');
  });
  it('arrowConflict: B starts before A is due', () => {
    expect(arrowConflict({ from: 1, to: 5 }, { from: 3, to: 8 })).toBe(true);
    expect(arrowConflict({ from: 1, to: 5 }, { from: 6, to: 8 })).toBe(false);
    expect(arrowConflict({ from: 1, to: 5 }, { from: 5, to: 8 })).toBe(false); // same day is fine
  });
});
