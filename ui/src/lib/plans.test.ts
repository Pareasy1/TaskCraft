import { describe, expect, it } from 'vitest';
import { bucketColumns, dropIndex, dropNeighbours, insertAt, planOrder, quickAddNeedsMove, dueColumns, labelColumns, nextPlanColor, uniqueName } from './plans';
import { errorMessage } from './api';
import { t } from './i18n';
import type { Bucket, Plan } from './types';

const b = (id: number, sort: number): Bucket => ({ id, planId: 1, name: `b${id}`, sort });
const card = (id: number, bucketId: number, sortOrder: number) => ({ id, bucketId, sortOrder });

describe('bucketColumns', () => {
  it('orders buckets and cards, keeps empty buckets', () => {
    const cols = bucketColumns([b(2, 1), b(1, 0), b(3, 2)], [card(5, 1, 20), card(4, 1, 10), card(6, 1, 10), card(7, 2, 0)]);
    expect(cols.map((c) => c.bucket.id)).toEqual([1, 2, 3]);
    expect(cols[0].tasks.map((x) => x.id)).toEqual([4, 6, 5]);
    expect(cols[1].tasks.map((x) => x.id)).toEqual([7]);
    expect(cols[2].tasks).toEqual([]);
  });
});

describe('dropNeighbours', () => {
  it('finds the cards above and below the insertion point', () => {
    expect(dropNeighbours([1, 2, 3], 3, 0)).toEqual({ before: null, after: 1 });
    expect(dropNeighbours([1, 2, 3], 1, 1)).toEqual({ before: 2, after: 3 });
    expect(dropNeighbours([1, 2, 3], 9, 3)).toEqual({ before: 3, after: null });
    expect(dropNeighbours([], 9, 0)).toEqual({ before: null, after: null });
  });
});

describe('dropIndex', () => {
  it('counts the card midpoints above the pointer', () => {
    expect(dropIndex([10, 30, 50], 5)).toBe(0);
    expect(dropIndex([10, 30, 50], 20)).toBe(1);
    expect(dropIndex([10, 30, 50], 60)).toBe(3);
    expect(dropIndex([], 60)).toBe(0);
  });
});

describe('insertAt', () => {
  it('moves an id to an insertion index of the original list', () => {
    expect(insertAt([1, 2, 3], 1, 3)).toEqual([2, 3, 1]);
    expect(insertAt([1, 2, 3], 3, 0)).toEqual([3, 1, 2]);
    expect(insertAt([1, 2, 3], 2, 1)).toEqual([1, 2, 3]);
    expect(insertAt([1, 2, 3], 2, 2)).toEqual([1, 2, 3]);
    expect(insertAt([1, 2, 3], 1, 2)).toEqual([2, 1, 3]);
  });
});

describe('planOrder', () => {
  it('reads like the board: bucket order, then card order', () => {
    const cmp = planOrder([b(1, 1), b(2, 0)]);
    const cards = [card(1, 1, 0), card(2, 2, 20), card(3, 2, 10), card(4, 1, 0)];
    expect([...cards].sort(cmp).map((c) => c.id)).toEqual([3, 2, 1, 4]);
  });
});

describe('quickAddNeedsMove', () => {
  // Viewing plan 2; the default plan is 1.
  const at = (hint: string | null, taskPlan: number) => quickAddNeedsMove({ hint, taskPlanId: taskPlan, scopePlanId: 2, defaultPlanId: 1 });
  it('moves a task without a plan hint from the default plan into the plan in view', () => expect(at(null, 1)).toBe(true));
  it('keeps an explicit plan hint', () => expect(at('3D print', 1)).toBe(false));
  it('leaves a task already in the plan in view', () => expect(at(null, 2)).toBe(false));
  it('leaves a task the store put somewhere other than the default plan', () => expect(at(null, 3)).toBe(false));
});

describe('dueColumns', () => {
  const now = new Date('2026-10-22T10:00:00');
  const task = (id: number, due: string | null, completedAt: string | null = null, dueHasTime = false) => ({ id, due, dueHasTime, completedAt });
  it('assigns open tasks to due buckets', () => {
    const cols = dueColumns(
      [
        task(1, '2026-10-20T00:00:00'),
        task(2, '2026-10-22T15:00:00', null, true),
        task(3, '2026-10-25T00:00:00'),
        task(4, '2026-10-26T00:00:00'),
        task(5, null),
        task(6, '2026-10-20T00:00:00', '2026-10-21T00:00:00'),
      ],
      now,
      1,
    );
    const ids = Object.fromEntries(cols.map((c) => [c.key, c.tasks.map((x) => x.id)]));
    expect(cols.map((c) => c.key)).toEqual(['overdue', 'today', 'thisWeek', 'later', 'nodate']);
    expect(ids).toEqual({ overdue: [1], today: [2], thisWeek: [3], later: [4], nodate: [5] });
  });
  const weekOf = (d: Date, ws: 0 | 1, dues: string[]) => {
    const cols = dueColumns(dues.map((x, i) => task(i + 1, x)), d, ws);
    return Object.fromEntries(cols.map((c) => [c.key, c.tasks.map((x) => x.id)]));
  };
  it('ends the week on Saturday when the week starts Sunday', () => {
    const dues = ['2026-10-24T00:00:00', '2026-10-25T00:00:00'];
    expect(weekOf(now, 0, dues)).toMatchObject({ thisWeek: [1], later: [2] });
    expect(weekOf(now, 1, dues)).toMatchObject({ thisWeek: [1, 2], later: [] });
  });
  it('on a Sunday the week ends today (Mon start) or next Saturday (Sun start)', () => {
    const sun = new Date('2026-10-25T10:00:00');
    const dues = ['2026-10-26T00:00:00', '2026-10-31T00:00:00', '2026-11-01T00:00:00'];
    expect(weekOf(sun, 1, dues)).toMatchObject({ thisWeek: [], later: [1, 2, 3] });
    expect(weekOf(sun, 0, dues)).toMatchObject({ thisWeek: [1, 2], later: [3] });
  });
});

describe('labelColumns', () => {
  it('lists a task under each label and unlabeled under nolabel', () => {
    const labels = [{ id: 1, name: 'a', color: '#000' }, { id: 2, name: 'b', color: '#111' }];
    const cols = labelColumns(
      [{ id: 1, labels: [labels[0], labels[1]] }, { id: 2, labels: [labels[1]] }, { id: 3, labels: [] }],
      labels,
    );
    const by = Object.fromEntries(cols.map((c) => [c.key, c.tasks.map((x) => x.id)]));
    expect(by).toEqual({ '1': [1], '2': [1, 2], nolabel: [3] });
  });
});

describe('nextPlanColor', () => {
  const plan = (color: string): Plan => ({ id: 1, name: 'p', color, sort: 0, openCount: 0 });
  it('skips used colours', () => {
    expect(nextPlanColor([])).toBe('#3b82f6');
    expect(nextPlanColor([plan('#3b82f6'), plan('#f97316')])).toBe('#10b981');
  });
});

describe('errorMessage', () => {
  it('translates the plan error codes', () => {
    expect(errorMessage({ code: 'duplicate', message: 'x' })).toBe(t('error.duplicate'));
    expect(errorMessage({ code: 'last_plan', message: 'x' })).toBe(t('del.lastPlan'));
    expect(errorMessage({ code: 'last_bucket', message: 'x' })).toBe(t('del.lastBucket'));
  });
});

describe('uniqueName', () => {
  it('keeps a free name and numbers taken ones, ignoring case', () => {
    expect(uniqueName('New plan', ['Other'])).toBe('New plan');
    expect(uniqueName('New plan', ['New plan'])).toBe('New plan 2');
    expect(uniqueName('New plan', ['New plan', 'new plan 2'])).toBe('New plan 3');
    expect(uniqueName('New plan', ['new plan'])).toBe('New plan 2');
  });
});

import { lastTopLevelIn } from './plans';

describe('lastTopLevelIn (quick add appends after it)', () => {
  const x = (id: number, bucketId: number, sortOrder: number, parentId: number | null = null) => ({ id, bucketId, sortOrder, parentId });
  it('ignores subtasks, which share their parent bucket and have sibling sort orders', () => {
    const tasks = [x(1, 5, 10), x(2, 5, 20), x(3, 5, 99, 1), x(4, 6, 50)];
    expect(lastTopLevelIn(tasks, 5)?.id).toBe(2);
  });
  it('null for an empty bucket or one with only subtasks', () => {
    expect(lastTopLevelIn([x(3, 5, 99, 1)], 5)).toBeNull();
    expect(lastTopLevelIn([], 5)).toBeNull();
  });
});
