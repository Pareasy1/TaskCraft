// Helpers for task templates: the "New from template" menu and name checks.
import { dictionaries, format, lang as appLang, type Lang } from './i18n';
import type { TemplateInfo } from './types';

/** The store's comparison key for template names (trimmed, Unicode lower case). */
const nameKey = (name: string) => name.trim().toLowerCase();

/**
 * The template whose name equals `name` ignoring case (Unicode-aware, like the store), or null.
 * `exceptId` skips the template being renamed.
 */
export function nameConflict(name: string, templates: TemplateInfo[], exceptId?: number): TemplateInfo | null {
  const key = nameKey(name);
  if (!key) return null;
  return templates.find((x) => x.id !== exceptId && nameKey(x.name) === key) ?? null;
}

export type TemplateMenuItem =
  | { kind: 'template'; id: number; name: string; detail: string }
  | { kind: 'empty'; label: string };

/** "2 subtasks", "1 subtask", or '' for none. */
export function subtaskCountLabel(n: number, lang: Lang = appLang): string {
  const d = dictionaries[lang];
  return n === 0 ? '' : n === 1 ? d['template.subtasksOne'] : format(d['template.subtasks'], { n });
}

/** The "New from template" menu: templates by name (ignoring case), or one empty-state item. */
export function templateMenuItems(templates: TemplateInfo[], lang: Lang = appLang): TemplateMenuItem[] {
  if (!templates.length) return [{ kind: 'empty', label: dictionaries[lang]['template.none'] }];
  return [...templates]
    .sort((a, b) => a.name.localeCompare(b.name, lang, { sensitivity: 'base' }) || a.id - b.id)
    .map((x) => ({ kind: 'template', id: x.id, name: x.name, detail: subtaskCountLabel(x.subtaskCount, lang) }));
}

/** The toast after duplicating: a confirmation, or which files could not be copied. */
export function duplicatedMessage(title: string, skipped: string[], lang: Lang = appLang): string {
  const d = dictionaries[lang];
  return skipped.length ? format(d['task.duplicateSkipped'], { names: skipped.join(', ') }) : format(d['task.duplicated'], { title });
}
