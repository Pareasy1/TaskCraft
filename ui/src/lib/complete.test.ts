import { describe, expect, it } from 'vitest';
import { answerComplete, pending, requestComplete } from './complete.svelte';
import type { Task } from './types';

const t = { id: 1, title: 'Ship Rafael' } as Task;

describe('requestComplete', () => {
  it('shows the request until answered, then resolves with the answer', async () => {
    const p = requestComplete(t, 2);
    expect(pending.current?.task.id).toBe(1);
    expect(pending.current?.openChildren).toBe(2);
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
