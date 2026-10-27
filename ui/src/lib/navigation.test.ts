import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { loadCalendarMode, saveCalendarMode, shortcutFor } from './navigation';

const k = (key: string, mod = false, inTextField = false) => ({ key, mod, inTextField });

describe('shortcutFor', () => {
  it('maps Mod+1/2/3 to areas', () => {
    expect(shortcutFor(k('1', true), 'tasks')).toEqual({ area: 'calendar' });
    expect(shortcutFor(k('2', true), 'calendar')).toEqual({ area: 'tasks' });
    expect(shortcutFor(k('3', true), 'calendar')).toEqual({ area: 'grid' });
  });
  it('maps Mod+4 to the planner', () => {
    expect(shortcutFor(k('4', true), 'calendar')).toEqual({ area: 'planner' });
    expect(shortcutFor(k('4', true), 'grid')).toEqual({ area: 'planner' });
  });
  it('ignores Mod+5', () => expect(shortcutFor(k('5', true), 'calendar')).toBeNull());
  it('D/W/M set the mode in calendar outside text fields', () => {
    expect(shortcutFor(k('d'), 'calendar')).toEqual({ mode: 'day' });
    expect(shortcutFor(k('w'), 'calendar')).toEqual({ mode: 'week' });
    expect(shortcutFor(k('M'), 'calendar')).toEqual({ mode: 'month' });
  });
  it('ignores letters in a text field', () => expect(shortcutFor(k('d', false, true), 'calendar')).toBeNull());
  it('ignores letters outside calendar', () => {
    expect(shortcutFor(k('w'), 'tasks')).toBeNull();
    expect(shortcutFor(k('w'), 'grid')).toBeNull();
  });
  it('D/W/M do nothing in the planner', () => {
    for (const key of ['d', 'w', 'm']) expect(shortcutFor(k(key), 'planner')).toBeNull();
  });
  it('ignores letters with mod and other keys', () => {
    expect(shortcutFor(k('d', true), 'calendar')).toBeNull();
    expect(shortcutFor(k('x'), 'calendar')).toBeNull();
  });
});

describe('shortcutFor letter guards', () => {
  it('ignores letters while an overlay is open', () => {
    expect(shortcutFor({ ...k('d'), inOverlay: true }, 'calendar')).toBeNull();
  });
  it('ignores letters with ctrl or meta held', () => {
    expect(shortcutFor({ ...k('d'), ctrl: true }, 'calendar')).toBeNull();
    expect(shortcutFor({ ...k('d'), meta: true }, 'calendar')).toBeNull();
  });
  it('ignores auto-repeat', () => {
    expect(shortcutFor({ ...k('d'), repeat: true }, 'calendar')).toBeNull();
  });
  it('area shortcuts still work with an overlay flag absent', () => {
    expect(shortcutFor(k('3', true), 'calendar')).toEqual({ area: 'grid' });
  });
});

describe('calendar mode storage', () => {
  beforeEach(() => {
    const m = new Map<string, string>();
    vi.stubGlobal('localStorage', { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) });
  });
  afterEach(() => vi.unstubAllGlobals());
  it('round-trips', () => {
    saveCalendarMode('month');
    expect(localStorage.getItem('taskcraft.calendarMode')).toBe('month');
    expect(loadCalendarMode()).toBe('month');
  });
  it('defaults to week', () => expect(loadCalendarMode()).toBe('week'));
  it('falls back on an invalid value', () => {
    localStorage.setItem('taskcraft.calendarMode', 'year');
    expect(loadCalendarMode()).toBe('week');
  });
  it('survives throwing storage', () => {
    const bad = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };
    vi.stubGlobal('localStorage', bad);
    expect(loadCalendarMode()).toBe('week');
    expect(() => saveCalendarMode('day')).not.toThrow();
  });
});
