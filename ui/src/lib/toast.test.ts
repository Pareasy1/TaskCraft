import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { dismiss, notify, toast, undo } from './toast.svelte';

describe('toast', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    dismiss();
    vi.useRealTimers();
  });

  it('shows one message at a time; a new one replaces the old', () => {
    notify('Deleted “A”');
    notify('Deleted “B”');
    expect(toast.current?.message).toBe('Deleted “B”');
  });

  it('disappears after 6 seconds', () => {
    notify('Moved “A”');
    vi.advanceTimersByTime(5999);
    expect(toast.current).not.toBeNull();
    vi.advanceTimersByTime(1);
    expect(toast.current).toBeNull();
  });

  it('undo runs the action once and hides the message', async () => {
    const fn = vi.fn(async () => {});
    notify('Completed “A”', fn);
    expect(toast.current?.canUndo).toBe(true);
    await undo();
    await undo();
    expect(fn).toHaveBeenCalledTimes(1);
    expect(toast.current).toBeNull();
  });

  it('a replaced message can no longer be undone', async () => {
    const first = vi.fn(async () => {});
    notify('A', first);
    notify('B');
    await undo();
    expect(first).not.toHaveBeenCalled();
  });

  it('reports a failed undo instead of throwing', async () => {
    notify('A', async () => {
      throw new Error('task not found');
    });
    await undo();
    expect(toast.current?.message).toBe('Couldn’t undo: task not found');
    expect(toast.current?.canUndo).toBe(false);
  });
});

import { runAction } from './toast.svelte';

describe('toast action', () => {
  afterEach(() => dismiss());
  it('shows an extra button that runs once and hides the message', async () => {
    const run = vi.fn(async () => {});
    notify('All subtasks of A are done', undefined, { label: 'Complete it', run });
    expect(toast.current?.action).toBe('Complete it');
    expect(toast.current?.canUndo).toBe(false);
    await runAction();
    await runAction();
    expect(run).toHaveBeenCalledTimes(1);
    expect(toast.current).toBeNull();
  });
  it('a failed action is reported', async () => {
    notify('x', undefined, { label: 'Go', run: async () => { throw new Error('nope'); } });
    await runAction();
    expect(toast.current?.message).toContain('nope');
  });
});
