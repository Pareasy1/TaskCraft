import { describe, expect, it } from 'vitest';
import { dayLabel } from './format';

describe('dayLabel (spoken name of a calendar day)', () => {
  const today = new Date(2026, 9, 20);
  it('names the full date', () => expect(dayLabel(new Date(2026, 9, 21), today, false)).toBe('Wednesday, October 21, 2026'));
  it('adds today and events when they apply', () =>
    expect(dayLabel(new Date(2026, 9, 20), today, true)).toBe('Tuesday, October 20, 2026, today, has events'));
});
