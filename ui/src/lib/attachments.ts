// Helpers for attachment display.
import { lang as appLang, dateLocale, type Lang } from './i18n';

const UNITS = ['B', 'KB', 'MB', 'GB'];

/** A file size such as "812 B" or "1.2 MB" ("1,2 MB" in Italian). */
export function formatSize(bytes: number, lang: Lang = appLang): string {
  let v = Math.max(0, bytes);
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) { v /= 1024; u++; }
  const locale = lang === appLang ? dateLocale : lang;
  const n = new Intl.NumberFormat(locale, { maximumFractionDigits: u === 0 ? 0 : 1 }).format(v);
  return `${n} ${UNITS[u]}`;
}

/** The host of a link without "www.", or the input when it is not a URL. */
export function urlHost(url: string): string {
  try {
    return new URL(url.trim()).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

/** True for http(s) URLs, the only links attachments accept. */
export function isWebUrl(s: string): boolean {
  try {
    const u = new URL(s.trim());
    return u.protocol === 'http:' || u.protocol === 'https:';
  } catch {
    return false;
  }
}

/**
 * One toast for the files "Add file…" could not add: a sentence per reason, listing its names,
 * in the order the reasons first appear. `reason` gives null for codes without a message of
 * their own (io, db…); those share `fallback`. Null when all were added.
 */
export function notAddedMessage(
  errors: { name: string; code: string }[],
  reason: (code: string) => string | null,
  sentence: (names: string, reason: string) => string,
  fallback = '',
): string | null {
  if (!errors.length) return null;
  const byReason = new Map<string, string[]>();
  for (const e of errors) {
    const why = reason(e.code) ?? fallback;
    byReason.set(why, [...(byReason.get(why) ?? []), e.name]);
  }
  return [...byReason].map(([why, names]) => sentence(names.join(', '), why)).join(' ');
}

/**
 * For a drag carrying OS files (dragover/drop): stops the webview from navigating to the file
 * and shows the "no drop" cursor. Tauri's own file-drop handler is off for the main window, so
 * files are added with the Add file button. Returns whether it was a file drag.
 */
export function refuseFileDrop(e: DragEvent): boolean {
  const dt = e.dataTransfer;
  if (!dt || !Array.from(dt.types ?? []).includes('Files')) return false;
  e.preventDefault();
  dt.dropEffect = 'none';
  return true;
}
