// Dev-only fake backend so the UI can be previewed in a normal browser
// (`npm run dev`, then open http://localhost:5173/?window=main|quick|settings).
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';

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
  const task = (id: number, title: string, due: string | null, dueHasTime: boolean, done = false) => ({
    id, calendarId: 1, title, due, dueHasTime, tz: 'Europe/Rome', completedAt: done ? d(0, 8) : null,
    recurring: false, rrule: null, notes: null, color: '#3b82f6', alerts: [],
  });
  const tasks = {
    overdue: [task(10, 'Pay electricity bill', midnight(-2), false)],
    today: [task(11, 'Call mom', d(0, 18), true)],
    upcoming: [task(12, 'Buy milk', midnight(3), false), task(13, 'Read book', null, false)],
    completed: [task(14, 'Book dentist', midnight(-1), false, true)],
    all: [],
  };
  mockIPC((cmd, args: any) => {
    switch (cmd) {
      case 'list_calendars': return calendars;
      case 'get_settings': return { hotkey: 'Alt+Super+Space', launchAtLogin: false, defaultCalendarId: 1, defaultEventMinutes: 60, weekStart: 'Mon', dateOrder: 'dayMonth' };
      case 'list_range': return occ;
      case 'list_tasks': return tasks[args.filter as keyof typeof tasks];
      case 'local_time_zone': return 'Europe/Rome';
      case 'take_pending_focus': return null;
      case 'search': return [];
      case 'parse_preview': {
        const t: string = args.text;
        const i = t.indexOf(' tomorrow at 1');
        return {
          kind: 'event', title: i > 0 ? t.slice(0, i) : t, start: { date: '2026-10-21', time: '13:00:00' },
          end: { date: '2026-10-21', time: '14:00:00' }, allDay: false, location: null, attendees: ['Sam'],
          calendar: null, recurrence: null, alerts: [], url: null,
          spans: i > 0 ? [{ start: 0, end: i, label: 'title' }, { start: i + 1, end: i + 9, label: 'date' }, { start: i + 10, end: i + 12, label: 'separator' }, { start: i + 13, end: i + 14, label: 'time' }] : [],
        };
      }
      case 'get_event': return { id: args.id, ...occ[0], attendees: [], notes: null, url: null, rrule: null, alerts: [], tz: 'Europe/Rome' };
      default: return null;
    }
  });
}
