import { describe, expect, it } from 'vitest';
import { hotkeyLabel, isMacUA, isMod, modLabel } from './platform';

describe('platform', () => {
  it('recognises the Mac from the user agent', () => {
    expect(isMacUA('Mozilla/5.0 (Macintosh; Intel Mac OS X 15_0) AppleWebKit/605.1.15')).toBe(true);
    expect(isMacUA('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Edg/120')).toBe(false);
    expect(isMacUA('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15')).toBe(false);
  });

  it('the command key is ⌘ on the Mac and Ctrl elsewhere', () => {
    expect(isMod({ metaKey: true, ctrlKey: false }, true)).toBe(true);
    expect(isMod({ metaKey: false, ctrlKey: true }, true)).toBe(false);
    expect(isMod({ metaKey: false, ctrlKey: true }, false)).toBe(true);
    expect(isMod({ metaKey: true, ctrlKey: false }, false)).toBe(false);
  });

  it('writes shortcuts the way each system does', () => {
    expect(modLabel('N', true)).toBe('⌘N');
    expect(modLabel('N', false)).toBe('Ctrl+N');
    expect(hotkeyLabel('Alt+Super+Space', true)).toBe('⌥⌘Space');
    expect(hotkeyLabel('Control+Alt+Space', false)).toBe('Ctrl+Alt+Space');
    expect(hotkeyLabel('Control+Shift+KeyK', false)).toBe('Ctrl+Shift+K');
    expect(hotkeyLabel('Super+Digit1', false)).toBe('Win+1');
  });
});
