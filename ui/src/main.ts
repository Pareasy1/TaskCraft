import { mount } from 'svelte';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { lang } from './lib/i18n';
import { isMac } from './lib/platform';
import './styles.css';
import Quick from './quick/Quick.svelte';
import Main from './main/Main.svelte';

if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window && (window as any).__TAURI_INTERNALS__.invoke)) {
  (await import('./dev/mock')).installMock();
}

const label = getCurrentWebviewWindow().label;
const component = label === 'quick' ? Quick : Main;
document.documentElement.dataset.window = label;
document.documentElement.lang = lang;
// Mac-only window styling (traffic-light gap, translucent popover) keys off this.
document.documentElement.dataset.platform = isMac ? 'mac' : 'pc';
// The menu-bar menu and notifications follow the UI language.
invoke('set_ui_language', { lang }).catch(() => {});

export default mount(component, { target: document.getElementById('app')! });
