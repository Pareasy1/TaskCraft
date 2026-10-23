import { describe, expect, it } from 'vitest';
import { dayLabel } from './format';

describe('dayLabel (spoken name of a calendar day)', () => {
  const today = new Date(2026, 9, 20);
  it('names the full date', () => expect(dayLabel(new Date(2026, 9, 21), today, false)).toBe('Wednesday, October 21, 2026'));
  it('adds today and events when they apply', () =>
    expect(dayLabel(new Date(2026, 9, 20), today, true)).toBe('Tuesday, October 20, 2026, today, has events'));
});

import { fmtSpan } from './format';

describe('fmtSpan (start → due on a card)', () => {
  it('same month: the month once', () => expect(fmtSpan(new Date(2026, 9, 24), new Date(2026, 9, 28))).toBe('Oct 24 → 28'));
  it('different months: both in full', () => expect(fmtSpan(new Date(2026, 9, 30), new Date(2026, 10, 2))).toBe('Oct 30 → Nov 2'));
});
