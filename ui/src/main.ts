import { mount } from 'svelte';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import './styles.css';
import Quick from './quick/Quick.svelte';
import Main from './main/Main.svelte';
import Settings from './settings/Settings.svelte';

const label = getCurrentWebviewWindow().label;
const component = label === 'quick' ? Quick : label === 'settings' ? Settings : Main;
document.documentElement.dataset.window = label;

export default mount(component, { target: document.getElementById('app')! });
