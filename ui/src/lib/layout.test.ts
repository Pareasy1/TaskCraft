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

import { nudgeEvent } from './layout';

describe('nudgeEvent', () => {
  const s = new Date(2026, 9, 21, 9, 0);
  const e = new Date(2026, 9, 21, 10, 0);
  const hm = (d: Date) => `${d.getDate()} ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;

  it('↑/↓ move by 15 minutes keeping the length', () => {
    const up = nudgeEvent(s, e, 'ArrowUp', false)!;
    expect([hm(up.start), hm(up.end)]).toEqual(['21 8:45', '21 9:45']);
    const down = nudgeEvent(s, e, 'ArrowDown', false)!;
    expect([hm(down.start), hm(down.end)]).toEqual(['21 9:15', '21 10:15']);
  });

  it('Shift+↑/↓ change the length, never below 15 minutes', () => {
    expect(hm(nudgeEvent(s, e, 'ArrowDown', true)!.end)).toBe('21 10:15');
    expect(hm(nudgeEvent(s, e, 'ArrowUp', true)!.end)).toBe('21 9:45');
    const short = nudgeEvent(s, new Date(2026, 9, 21, 9, 15), 'ArrowUp', true)!;
    expect(hm(short.end)).toBe('21 9:15');
  });

  it('←/→ move a day at the same local time, across the DST change', () => {
    const sat = nudgeEvent(new Date(2026, 9, 24, 9, 0), new Date(2026, 9, 24, 10, 0), 'ArrowRight', false)!;
    expect([hm(sat.start), hm(sat.end)]).toEqual(['25 9:00', '25 10:00']);
    expect(hm(nudgeEvent(s, e, 'ArrowLeft', false)!.start)).toBe('20 9:00');
  });

  it('other keys do nothing', () => expect(nudgeEvent(s, e, 'a', false)).toBeNull());
});

import { monthBars } from './layout';

describe('monthBars', () => {
  const grid = monthGrid(2026, 9, 1); // Mon 28 Sep … Sun 8 Nov
  const iso = (m: number, d: number, h = 0) => new Date(2026, m, d, h).toISOString();
  const ev = (title: string, start: string, end: string, allDay = true) => ({ title, start, end, allDay });

  it('a two-day all-day event is one bar in its week', () => {
    const [b] = monthBars([ev('Trip to Milan', iso(9, 22), iso(9, 24))], grid);
    expect({ row: b.row, col: b.col, span: b.span, lane: b.lane, fromPrev: b.fromPrev, toNext: b.toNext })
      .toEqual({ row: 3, col: 3, span: 2, lane: 0, fromPrev: false, toNext: false });
  });

  it('an event crossing a week boundary becomes two joined bars', () => {
    const bars = monthBars([ev('Weekend away', iso(9, 24), iso(9, 27))], grid); // Sat–Mon
    expect(bars.map((b) => [b.row, b.col, b.span, b.fromPrev, b.toNext])).toEqual([
      [3, 5, 2, false, true],
      [4, 0, 1, true, false],
    ]);
  });

  it('overlapping bars stack into lanes; timed single-day events are left to the cells', () => {
    const bars = monthBars([
      ev('A', iso(9, 20), iso(9, 23)),
      ev('B', iso(9, 21), iso(9, 22)),
      ev('C', iso(9, 23), iso(9, 24)),
      ev('Lunch', iso(9, 21, 13), iso(9, 21, 14), false),
    ], grid);
    expect(bars.map((b) => [b.item.title, b.lane])).toEqual([['A', 0], ['B', 1], ['C', 0]]);
  });

  it('a timed event running past midnight is a bar too', () => {
    const [b] = monthBars([ev('Night shift', iso(9, 21, 22), iso(9, 22, 6), false)], grid);
    expect([b.row, b.col, b.span]).toEqual([3, 2, 2]);
  });

  it('events outside the grid produce nothing', () =>
    expect(monthBars([ev('Old', iso(7, 1), iso(7, 3))], grid)).toEqual([]));
});

import { gridKeyMove } from './layout';

describe('gridKeyMove (calendar grid keyboard)', () => {
  const d = new Date(2026, 9, 21); // Wed 21 Oct
  const ymd = (x: Date | null) => (x ? `${x.getMonth() + 1}-${x.getDate()}` : null);
  it('arrows move a day or a week', () => {
    expect(ymd(gridKeyMove(d, 'ArrowLeft', 1))).toBe('10-20');
    expect(ymd(gridKeyMove(d, 'ArrowRight', 1))).toBe('10-22');
    expect(ymd(gridKeyMove(d, 'ArrowUp', 1))).toBe('10-14');
    expect(ymd(gridKeyMove(d, 'ArrowDown', 1))).toBe('10-28');
  });
  it('Home/End go to the start/end of the week, honouring the week start', () => {
    expect(ymd(gridKeyMove(d, 'Home', 1))).toBe('10-19');
    expect(ymd(gridKeyMove(d, 'End', 1))).toBe('10-25');
    expect(ymd(gridKeyMove(d, 'Home', 0))).toBe('10-18');
  });
  it('PageUp/PageDown change month, clamping to its length', () => {
    expect(ymd(gridKeyMove(new Date(2026, 0, 31), 'PageDown', 1))).toBe('2-28');
    expect(ymd(gridKeyMove(d, 'PageUp', 1))).toBe('9-21');
  });
  it('other keys do nothing', () => expect(gridKeyMove(d, 'a', 1)).toBeNull());
});
