// Dev-only fake backend so the UI can be previewed in a normal browser
// (`npm run dev`, then open http://localhost:5173/?window=main|quick|settings).
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';

export function installMock() {
  const label = new URLSearchParams(location.search).get('window') ?? 'main';
  mockWindows(label);
  const d = (days: number, h: number, m = 0) => {
    const x = new Date();
    x.setHours(h, m, 0, 0);
    x.setDate(x.getDate() + days);
    return x.toISOString();
  };
  const midnight = (days: number) => d(days, 0);
  const calendars = [
    { id: 1, name: 'Personal', color: '#3b82f6', visible: true, sort: 0 },
    { id: 2, name: 'Work', color: '#f97316', visible: true, sort: 1 },
  ];
  const occ = [
    { eventId: 1, calendarId: 2, title: 'Standup', start: d(0, 9, 30), end: d(0, 10), allDay: false, location: null, recurring: true, color: '#f97316' },
    { eventId: 2, calendarId: 1, title: 'Lunch with Sam', start: d(0, 13), end: d(0, 14), allDay: false, location: 'Trattoria', recurring: false, color: '#3b82f6' },
    { eventId: 3, calendarId: 2, title: 'Design review', start: d(0, 13, 30), end: d(0, 15), allDay: false, location: null, recurring: false, color: '#f97316' },
    { eventId: 4, calendarId: 1, title: 'Trip to Milan', start: midnight(2), end: midnight(4), allDay: true, location: null, recurring: false, color: '#3b82f6' },
    { eventId: 5, calendarId: 1, title: 'Dinner with Anna', start: d(1, 20), end: d(1, 21, 30), allDay: false, location: null, recurring: false, color: '#3b82f6' },
  ];
  const labels = [
    { id: 1, name: 'home', color: '#10b981' },
    { id: 2, name: 'work', color: '#f97316' },
    { id: 3, name: 'errands', color: '#a855f7' },
  ];
  const L = (...ids: number[]) => ids.map((id) => labels.find((l) => l.id === id)!);
  type MockTask = {
    id: number; calendarId: number; title: string; due: string | null; dueHasTime: boolean; tz: string;
    completedAt: string | null; recurring: boolean; rrule: string | null; notes: string | null; color: string;
    alerts: []; status: 'todo' | 'inProgress' | 'done'; labels: { id: number; name: string; color: string }[];
  };
  const task = (id: number, title: string, due: string | null, dueHasTime: boolean, status: MockTask['status'] = 'todo', lbl: number[] = []): MockTask => ({
    id, calendarId: 1, title, due, dueHasTime, tz: 'Europe/Rome', completedAt: status === 'done' ? d(0, 8) : null,
    recurring: false, rrule: null, notes: null, color: '#3b82f6', alerts: [], status, labels: L(...lbl),
  });
  let all: MockTask[] = [
    task(10, 'Pay electricity bill', midnight(-2), false, 'todo', [1]),
    task(11, 'Call mom', d(0, 18), true, 'todo', [1]),
    task(12, 'Buy milk', midnight(3), false, 'todo', [3]),
    task(13, 'Read book', null, false),
    task(15, 'Renew passport', d(1, 11), true, 'inProgress', [3]),
    task(16, 'Quarterly report draft', d(2, 17), true, 'inProgress', [2]),
    task(17, 'Prepare slides for review', midnight(4), false, 'todo', [2]),
    task(14, 'Book dentist', midnight(-1), false, 'done', [1]),
  ];
  const startToday = new Date(midnight(0)).getTime();
  const startTomorrow = new Date(midnight(1)).getTime();
  const open = () => all.filter((t) => !t.completedAt);
  const byFilter: Record<string, () => MockTask[]> = {
    all: () => all,
    completed: () => all.filter((t) => t.completedAt),
    overdue: () => open().filter((t) => t.due && new Date(t.due).getTime() < startToday),
    today: () => open().filter((t) => t.due && new Date(t.due).getTime() >= startToday && new Date(t.due).getTime() < startTomorrow),
    upcoming: () => open().filter((t) => !t.due || new Date(t.due).getTime() >= startTomorrow),
  };
  const changed = () => setTimeout(() => emit('items-changed'), 0);
  const setStatus = (t: MockTask, status: MockTask['status']) => {
    t.status = status;
    t.completedAt = status === 'done' ? new Date().toISOString() : null;
  };
  mockIPC((cmd, args: any) => {
    switch (cmd) {
      case 'list_calendars': return calendars;
      case 'get_settings': return { hotkey: 'Alt+Super+Space', launchAtLogin: false, defaultCalendarId: 1, defaultEventMinutes: 60, weekStart: 'Mon', dateOrder: 'dayMonth' };
      case 'list_range': return occ;
      case 'list_tasks': return byFilter[args.filter]();
      case 'get_task': return all.find((t) => t.id === args.id);
      case 'set_task_status': setStatus(all.find((t) => t.id === args.id)!, args.status); changed(); return null;
      case 'set_task_completed': setStatus(all.find((t) => t.id === args.id)!, args.done ? 'done' : 'todo'); changed(); return null;
      case 'list_labels': return [...labels].sort((a, b) => a.name.localeCompare(b.name));
      case 'update_label': {
        const l = labels.find((x) => x.id === args.id)!;
        Object.assign(l, { name: args.name, color: args.color });
        all.forEach((t) => t.labels.forEach((x) => x.id === l.id && Object.assign(x, l)));
        changed();
        return null;
      }
      case 'delete_label': {
        labels.splice(labels.findIndex((x) => x.id === args.id), 1);
        all.forEach((t) => (t.labels = t.labels.filter((x) => x.id !== args.id)));
        changed();
        return null;
      }
      case 'create_from_text': {
        const tags = [...String(args.text).matchAll(/#(\w+)/g)].map((m) => m[1]);
        const title = String(args.text).replace(/#\w+/g, '').trim();
        const lbl = tags.map((n) => labels.find((l) => l.name === n)?.id).filter((x): x is number => !!x);
        all = [...all, task(100 + all.length, title, null, false, 'todo', lbl)];
        changed();
        return { kind: 'task', id: 100 + all.length };
      }
      case 'local_time_zone': return 'Europe/Rome';
      case 'take_pending_focus': return null;
      case 'search': return [];
      case 'parse_preview': {
        const t: string = args.text;
        const i = t.indexOf(' tomorrow at 1');
        return {
          kind: 'event', title: i > 0 ? t.slice(0, i) : t, start: { date: '2026-10-21', time: '13:00:00' },
          end: { date: '2026-10-21', time: '14:00:00' }, allDay: false, location: null, attendees: ['Sam'],
          calendar: null, recurrence: null, alerts: [], url: null, tags: [],
          spans: i > 0 ? [{ start: 0, end: i, label: 'title' }, { start: i + 1, end: i + 9, label: 'date' }, { start: i + 10, end: i + 12, label: 'separator' }, { start: i + 13, end: i + 14, label: 'time' }] : [],
        };
      }
      case 'get_event': return { id: args.id, ...occ[0], attendees: [], notes: null, url: null, rrule: null, alerts: [], tz: 'Europe/Rome' };
      default: return null;
    }
  }, { shouldMockEvents: true });
}
