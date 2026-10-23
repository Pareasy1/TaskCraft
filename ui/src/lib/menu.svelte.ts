// A small popup menu ("…" button, right-click, Shift+F10) shared by the plan list,
// the bucket board and the Group-by button. Create it while a component initialises.
import { tick } from 'svelte';

/** Index to focus after `key` in a menu of `n` items (`i` = current, -1 for none); null if not a menu key. */
export function menuStep(key: string, i: number, n: number): number | null {
  if (!n) return null;
  switch (key) {
    case 'ArrowDown':
    case 'ArrowRight':
      return (i + 1) % n;
    case 'ArrowUp':
    case 'ArrowLeft':
      return i < 0 ? n - 1 : (i - 1 + n) % n;
    case 'Home':
      return 0;
    case 'End':
      return n - 1;
    default:
      return null;
  }
}

/** True for the keys that open a context menu from the keyboard. */
export const isMenuKey = (e: KeyboardEvent) => e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey);

export class PopupMenu<T> {
  /** What the open menu acts on, and where it sits (viewport coordinates). */
  current = $state<{ target: T; x: number; y: number } | null>(null);
  el = $state<HTMLElement | undefined>();
  private opener: HTMLElement | null = null;

  constructor() {
    // A click anywhere else closes the menu without moving focus.
    $effect(() => {
      if (!this.current) return;
      const away = (e: PointerEvent) =>
        !this.el?.contains(e.target as Node) && !this.opener?.contains(e.target as Node) && this.close(false);
      window.addEventListener('pointerdown', away, true);
      return () => window.removeEventListener('pointerdown', away, true);
    });
  }

  /** Opens at (x, y), kept inside the window, and focuses the first item. */
  async open(target: T, x: number, y: number, opener: HTMLElement) {
    this.opener = opener;
    this.current = { target, x, y };
    await tick();
    if (!this.el) return;
    const r = this.el.getBoundingClientRect();
    this.current = {
      target,
      x: Math.max(8, Math.min(x, innerWidth - r.width - 8)),
      y: y + r.height > innerHeight - 8 ? Math.max(8, innerHeight - r.height - 8) : y,
    };
    this.items()[0]?.focus();
  }

  /** Toggles the menu under a "…" button. */
  openBelow(target: T, button: HTMLElement, same: (a: T) => boolean) {
    if (this.current && same(this.current.target)) return this.close();
    const r = button.getBoundingClientRect();
    this.open(target, r.left, r.bottom + 4, button);
  }

  /** Opens from Shift+F10 / the menu key on `el`; returns true if it did. */
  openFromKey(e: KeyboardEvent, target: T, el = e.currentTarget as HTMLElement): boolean {
    if (!isMenuKey(e)) return false;
    e.preventDefault();
    const r = el.getBoundingClientRect();
    this.open(target, r.left + 16, r.bottom, el);
    return true;
  }

  close(refocus = true) {
    this.current = null;
    if (refocus && this.opener?.isConnected) this.opener.focus();
  }

  items = () => [...(this.el?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? [])];

  keys = (e: KeyboardEvent) => {
    e.stopPropagation();
    const list = this.items();
    const next = menuStep(e.key, list.indexOf(document.activeElement as HTMLElement), list.length);
    if (next != null) list[next]?.focus();
    else if (e.key === 'Escape') this.close();
    else if (e.key === 'Tab') return this.close(false);
    else return;
    e.preventDefault();
  };

  /** Runs a command on the menu's target once the menu has closed. */
  act(fn: (target: T) => void, refocus = true) {
    const target = this.current?.target;
    this.close(refocus);
    if (target !== undefined) fn(target);
  }
}
