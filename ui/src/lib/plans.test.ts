import { describe, expect, it } from 'vitest';
import { bucketColumns, dropNeighbours, dueColumns, labelColumns, nextPlanColor } from './plans';
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
    );
    const ids = Object.fromEntries(cols.map((c) => [c.key, c.tasks.map((x) => x.id)]));
    expect(cols.map((c) => c.key)).toEqual(['overdue', 'today', 'thisWeek', 'later', 'nodate']);
    expect(ids).toEqual({ overdue: [1], today: [2], thisWeek: [3], later: [4], nodate: [5] });
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
