import { describe, expect, it } from 'vitest';
import { answerComplete, pending, requestComplete, requestCompleteParent } from './complete.svelte';
import type { Task } from './types';

const t = { id: 1, title: 'Ship Rafael' } as Task;

describe('requestComplete', () => {
  it('shows the request until answered, then resolves with the answer', async () => {
    const p = requestComplete(t, 2);
    expect(pending.current).toMatchObject({ task: { id: 1 } });
    expect(pending.current).toMatchObject({ kind: 'children', openChildren: 2 });
    answerComplete('all');
    expect(pending.current).toBeNull();
    expect(await p).toBe('all');
  });
  it('a second request cancels the first', async () => {
    const a = requestComplete(t, 1);
    const b = requestComplete(t, 3);
    expect(await a).toBe('cancel');
    answerComplete('only');
    expect(await b).toBe('only');
  });
});

describe('requestCompleteParent', () => {
  it('asks about the parent; Yes resolves true, anything else false', async () => {
    let p = requestCompleteParent(t);
    expect(pending.current).toMatchObject({ kind: 'parent', task: { id: 1 } });
    answerComplete('yes');
    expect(await p).toBe(true);
    p = requestCompleteParent(t);
    answerComplete('cancel');
    expect(await p).toBe(false);
  });
  it('a newer question cancels it (No)', async () => {
    const a = requestCompleteParent(t);
    const b = requestComplete(t, 1);
    expect(await a).toBe(false);
    answerComplete('all');
    expect(await b).toBe('all');
  });
});
