import { describe, expect, it } from 'vitest';
import { duplicatedMessage, nameConflict, templateMenuItems } from './templates';
import type { TemplateInfo } from './types';

const tpl = (id: number, name: string, subtaskCount = 0): TemplateInfo => ({ id, name, subtaskCount });

describe('nameConflict', () => {
  it('compares Unicode case-insensitively', () => {
    const list = [tpl(1, 'Città'), tpl(2, 'Weekly review')];
    expect(nameConflict('CITTÀ', list)).toEqual(tpl(1, 'Città'));
    expect(nameConflict('weekly REVIEW', list)?.id).toBe(2);
  });
  it('ignores surrounding spaces', () => {
    expect(nameConflict('  Città ', [tpl(1, 'città')])?.id).toBe(1);
  });
  it('is null for a new name, or an empty one', () => {
    expect(nameConflict('Citta', [tpl(1, 'Città')])).toBeNull();
    expect(nameConflict('   ', [tpl(1, 'Città')])).toBeNull();
    expect(nameConflict('Anything', [])).toBeNull();
  });
  it('skips the template being renamed', () => {
    expect(nameConflict('CITTÀ', [tpl(1, 'Città')], 1)).toBeNull();
    expect(nameConflict('CITTÀ', [tpl(1, 'Città'), tpl(2, 'x')], 2)?.id).toBe(1);
  });
});

describe('templateMenuItems', () => {
  it('lists templates sorted by name, case-insensitively', () => {
    const items = templateMenuItems([tpl(1, 'zebra'), tpl(2, 'Apple', 2), tpl(3, 'banana', 1)], 'en');
    expect(items.map((i) => (i.kind === 'template' ? i.name : null))).toEqual(['Apple', 'banana', 'zebra']);
  });
  it('carries the id and a subtask-count detail', () => {
    const items = templateMenuItems([tpl(2, 'Apple', 2), tpl(3, 'Banana', 1), tpl(4, 'Cherry', 0)], 'en');
    expect(items).toEqual([
      { kind: 'template', id: 2, name: 'Apple', detail: '2 subtasks' },
      { kind: 'template', id: 3, name: 'Banana', detail: '1 subtask' },
      { kind: 'template', id: 4, name: 'Cherry', detail: '' },
    ]);
  });
  it('gives a single empty-state item when there are none', () => {
    expect(templateMenuItems([], 'en')).toEqual([{ kind: 'empty', label: 'No templates yet. Save a task as a template from its menu.' }]);
    expect(templateMenuItems([], 'it')).toEqual([{ kind: 'empty', label: 'Nessun modello. Salva un\'attività come modello dal suo menu.' }]);
  });
  it('does not reorder the caller\'s list', () => {
    const list = [tpl(1, 'b'), tpl(2, 'a')];
    templateMenuItems(list, 'en');
    expect(list.map((x) => x.id)).toEqual([1, 2]);
  });
});

describe('duplicatedMessage', () => {
  it('confirms the copy when every file came along', () => {
    expect(duplicatedMessage('Report', [], 'en')).toBe('Duplicated “Report”');
  });
  it('names the files left behind instead', () => {
    expect(duplicatedMessage('Report', ['a.pdf', 'b.png'], 'en')).toBe('Copied without a.pdf, b.png (file no longer available).');
    expect(duplicatedMessage('Report', ['a.pdf'], 'it')).toBe('Copiata senza a.pdf (file non più disponibile).');
  });
});
