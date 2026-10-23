import { describe, expect, it } from 'vitest';
import { dictionaries, format, pickLang } from './i18n';

describe('pickLang', () => {
  it('follows the first supported Mac language', () => {
    expect(pickLang(['it-IT', 'en-US'])).toBe('it');
    expect(pickLang(['en-GB', 'it-IT'])).toBe('en');
    expect(pickLang(['de-DE', 'it-IT'])).toBe('it');
  });
  it('falls back to English', () => expect(pickLang(['de-DE', 'fr-FR'])).toBe('en'));
  it('handles an empty list', () => expect(pickLang([])).toBe('en'));
});

describe('format', () => {
  it('fills placeholders', () => expect(format('Deleted “{title}”', { title: 'Buy milk' })).toBe('Deleted “Buy milk”'));
  it('leaves unknown placeholders visible', () => expect(format('Hi {who}', {})).toBe('Hi {who}'));
});

describe('dictionaries', () => {
  const keys = (d: Record<string, string>) => Object.keys(d).sort();
  const vars = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

  it('Italian has exactly the English keys', () => expect(keys(dictionaries.it)).toEqual(keys(dictionaries.en)));

  it('each Italian string uses the same placeholders as English', () => {
    for (const k of Object.keys(dictionaries.en) as (keyof typeof dictionaries.en)[]) {
      expect(vars(dictionaries.it[k]), k).toEqual(vars(dictionaries.en[k]));
    }
  });

  it('no Italian string is left untranslated by accident', () => {
    const same = (Object.keys(dictionaries.en) as (keyof typeof dictionaries.en)[]).filter(
      (k) => dictionaries.it[k] === dictionaries.en[k] && /[a-z]{4,}/i.test(dictionaries.en[k]),
    );
    // Words that are the same in both languages.
    expect(same.filter((k) => !['inspector.peoplePlaceholder', 'settings.ics', 'groupBy.bucket', 'details.bucket'].includes(k))).toEqual([]);
  });
});
