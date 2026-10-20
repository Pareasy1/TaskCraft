// Keyboard conventions per system: the command key is ⌘ on the Mac and Ctrl
// on Windows and Linux, and shortcuts are written the way each system does.

export const isMacUA = (ua: string) => /Mac|iPhone|iPad/.test(ua);

// In development, `?platform=pc` previews the Windows/Linux conventions in a browser on a Mac.
const devPlatform = import.meta.env?.DEV && typeof location !== 'undefined' ? new URLSearchParams(location.search).get('platform') : null;
export const isMac = devPlatform ? devPlatform === 'mac' : typeof navigator !== 'undefined' && isMacUA(navigator.userAgent);

/** True when the system's command modifier is held (⌘ on Mac, Ctrl elsewhere). */
export const isMod = (e: { metaKey: boolean; ctrlKey: boolean }, mac = isMac) => (mac ? e.metaKey : e.ctrlKey);

/** "⌘N" on the Mac, "Ctrl+N" elsewhere. */
export const modLabel = (key: string, mac = isMac) => (mac ? `⌘${key}` : `Ctrl+${key}`);

const MAC_SYMBOL: Record<string, string> = { Control: '⌃', Alt: '⌥', Shift: '⇧', Super: '⌘' };
const PC_NAME: Record<string, string> = { Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Super: 'Win' };

/** A stored shortcut ("Alt+Super+Space") as the user sees it: "⌥⌘Space" or "Ctrl+Alt+Space". */
export function hotkeyLabel(hotkey: string, mac = isMac): string {
  const parts = hotkey.split('+').map((p) => {
    if (p in MAC_SYMBOL) return mac ? MAC_SYMBOL[p] : PC_NAME[p];
    return p.startsWith('Key') ? p.slice(3) : p.startsWith('Digit') ? p.slice(5) : p;
  });
  return parts.join(mac ? '' : '+');
}
