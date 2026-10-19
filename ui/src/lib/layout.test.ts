import { describe, expect, it } from 'vitest';
import { monthGrid } from './layout';

const ymd = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

describe('monthGrid', () => {
  it('October 2026, Monday start', () => {
    const g = monthGrid(2026, 9, 1);
    expect(g.length).toBe(42);
    expect(ymd(g[0])).toBe('2026-09-28');
    expect(ymd(g[41])).toBe('2026-11-08');
  });

  it('Sunday start begins on the preceding Sunday', () => {
    const g = monthGrid(2026, 9, 0);
    expect(ymd(g[0])).toBe('2026-09-27');
  });

  it('survives the DST change inside the grid', () => {
    const g = monthGrid(2026, 9, 1);
    // Oct 25 2026 is the DST end in Europe; every cell must be a distinct day.
    expect(new Set(g.map(ymd)).size).toBe(42);
  });
});

import { layoutDay, snap15, weekDays } from './layout';

const at = (h: number, m = 0) => new Date(2026, 9, 21, h, m);

describe('layoutDay', () => {
  it('splits overlapping items into columns', () => {
    const r = layoutDay(
      [
        { id: 'a', start: at(9), end: at(10) },
        { id: 'b', start: at(9, 30), end: at(11) },
        { id: 'c', start: at(10), end: at(10, 30) },
      ],
      at(0),
    );
    const by = Object.fromEntries(r.map((x) => [x.id, x]));
    expect([by.a.col, by.b.col, by.c.col]).toEqual([0, 1, 0]);
    expect([by.a.cols, by.b.cols, by.c.cols]).toEqual([2, 2, 2]);
    expect(by.a.top).toBe(540);
    expect(by.b.height).toBe(90);
  });

  it('separate clusters keep full width', () => {
    const r = layoutDay([{ id: 'a', start: at(9), end: at(10) }, { id: 'b', start: at(14), end: at(15) }], at(0));
    expect(r.map((x) => x.cols)).toEqual([1, 1]);
  });

  it('clips items that cross midnight to the day', () => {
    const r = layoutDay([{ id: 'n', start: new Date(2026, 9, 20, 22), end: at(6) }], at(0));
    expect(r[0].top).toBe(0);
    expect(r[0].height).toBe(360);
  });

  it('gives very short items a minimum height', () => {
    const r = layoutDay([{ id: 's', start: at(9), end: at(9) }], at(0));
    expect(r[0].height).toBe(15);
  });
});

describe('weekDays', () => {
  it('Monday start week of Wed Oct 21 2026', () => {
    const w = weekDays(new Date(2026, 9, 21), 1);
    expect(w.map((d) => d.getDate())).toEqual([19, 20, 21, 22, 23, 24, 25]);
  });
  it('Sunday start', () => {
    expect(weekDays(new Date(2026, 9, 21), 0)[0].getDate()).toBe(18);
  });
});

describe('snap15', () => {
  it('rounds to the nearest quarter hour', () => {
    expect(snap15(67)).toBe(60);
    expect(snap15(68)).toBe(75);
    expect(snap15(-5)).toBe(0);
  });
});

import { recurrenceToRrule } from './edit';

describe('recurrenceToRrule', () => {
  it('matches the Rust encoding', () => {
    expect(
      recurrenceToRrule({ rule: { freq: 'weekly', interval: 2, days: ['Mon', 'Thu'] }, until: '2026-12-31' }),
    ).toBe('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH;UNTIL=20261231');
    expect(recurrenceToRrule({ rule: { freq: 'monthly', interval: 1, monthDay: 1 }, until: null })).toBe(
      'FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=1',
    );
  });
});

import { sameInstant } from './edit';
import { dayGroups } from './layout';

describe('review fixes', () => {
  // Finding 6: Rust sends "…:00Z", JS produces "…:00.000Z".
  it('sameInstant ignores ISO formatting differences', () => {
    expect(sameInstant('2026-10-20T08:00:00Z', '2026-10-20T08:00:00.000Z')).toBe(true);
    expect(sameInstant('2026-10-20T08:00:00Z', '2026-10-20T08:15:00.000Z')).toBe(false);
  });

  // Finding 14: dated tasks appear under their day in the upcoming list.
  it('dayGroups places events and dated tasks under their local day', () => {
    const from = new Date(2026, 9, 20);
    const occ = [{ start: new Date(2026, 9, 21, 9).toISOString(), end: new Date(2026, 9, 21, 10).toISOString(), id: 'e' }];
    const tasks = [
      { due: new Date(2026, 9, 23).toISOString(), id: 't1' },
      { due: null, id: 't2' },
      { due: new Date(2026, 10, 30).toISOString(), id: 'far' },
    ];
    const g = dayGroups(occ, tasks, from, 14);
    expect(g.map((x) => x.day.getDate())).toEqual([21, 23]);
    expect(g[0].events.map((e) => e.id)).toEqual(['e']);
    expect(g[1].tasks.map((t) => t.id)).toEqual(['t1']);
  });
});
