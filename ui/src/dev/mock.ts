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
    planId: number; bucketId: number; sortOrder: number;
  };
  const task = (id: number, title: string, due: string | null, dueHasTime: boolean, status: MockTask['status'] = 'todo', lbl: number[] = []): MockTask => ({
    id, calendarId: 1, title, due, dueHasTime, tz: 'Europe/Rome', completedAt: status === 'done' ? d(0, 8) : null,
    recurring: false, rrule: null, notes: null, color: '#3b82f6', alerts: [], status, labels: L(...lbl),
    planId: 1, bucketId: 1, sortOrder: id * 10,
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
  // Plans and buckets: tasks 10-17 live in "Le mie attività"; 20-23 in "3D print".
  const plans = [
    { id: 1, name: 'Le mie attività', color: '#3b82f6', sort: 0 },
    { id: 2, name: '3D print', color: '#f97316', sort: 1 },
  ];
  const buckets = [
    { id: 1, planId: 1, name: 'Da fare', sort: 0 },
    { id: 2, planId: 1, name: 'In corso', sort: 1 },
    { id: 3, planId: 1, name: 'Fatto', sort: 2 },
    { id: 4, planId: 2, name: 'Da stampare', sort: 0 },
    { id: 5, planId: 2, name: 'Post-produzione', sort: 1 },
    { id: 6, planId: 2, name: 'Da spedire', sort: 2 },
  ];
  all.forEach((t) => (t.bucketId = t.status === 'done' ? 3 : t.status === 'inProgress' ? 2 : 1));
  all.push(
    { ...task(20, 'Print phone stand', midnight(1), false), planId: 2, bucketId: 4 },
    { ...task(21, 'Print gear housing', midnight(3), false, 'todo', [2]), planId: 2, bucketId: 4 },
    { ...task(22, 'Sand and paint figurine', null, false, 'inProgress'), planId: 2, bucketId: 5 },
    { ...task(23, 'Ship order #1042', midnight(2), false), planId: 2, bucketId: 6 },
  );
  let defaultPlan = 1;
  let nextId = 1000;
  const openCount = (planId: number) => all.filter((t) => t.planId === planId && !t.completedAt).length;
  const reorder = <T extends { id: number; sort: number }>(list: T[], ids: number[]) =>
    ids.forEach((id, i) => { const x = list.find((y) => y.id === id); if (x) x.sort = i; });
  // Removes a plan's/bucket's tasks per `mode`: moved to bucket `moveTo` (or that plan's first bucket) or deleted.
  const evacuate = (belongs: (t: MockTask) => boolean, mode: any, planOf: boolean) => {
    if (mode === 'deleteTasks') { all = all.filter((t) => !belongs(t)); return; }
    const target = planOf
      ? buckets.filter((b) => b.planId === mode.moveTo).sort((a, b) => a.sort - b.sort)[0]
      : buckets.find((b) => b.id === mode.moveTo)!;
    all.filter(belongs).forEach((t) => Object.assign(t, { planId: target.planId, bucketId: target.id }));
  };
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
      case 'update_task': {
        const t = all.find((x) => x.id === args.id)!;
        const i = args.input;
        Object.assign(t, { title: i.title, due: i.due, dueHasTime: i.dueHasTime, notes: i.notes, labels: labels.filter((l) => i.labels.includes(l.name)) });
        setStatus(t, i.status);
        changed();
        return null;
      }
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
      case 'list_plans': return [...plans].sort((a, b) => a.sort - b.sort).map((p) => ({ ...p, openCount: openCount(p.id) }));
      case 'create_plan': {
        if (plans.some((p) => p.name.toLowerCase() === String(args.name).toLowerCase())) throw { code: 'duplicate', message: 'duplicate' };
        const id = nextId++;
        plans.push({ id, name: args.name, color: args.color, sort: plans.length });
        buckets.push({ id: nextId++, planId: id, name: 'To do', sort: 0 });
        changed();
        return id;
      }
      case 'update_plan': {
        const p = plans.find((x) => x.id === args.id)!;
        if (args.name != null) p.name = args.name;
        if (args.color != null) p.color = args.color;
        changed();
        return null;
      }
      case 'reorder_plans': reorder(plans, args.ids); changed(); return null;
      case 'delete_plan': {
        if (plans.length <= 1) throw { code: 'last_plan', message: 'last_plan' };
        evacuate((t) => t.planId === args.id, args.mode, true);
        plans.splice(plans.findIndex((p) => p.id === args.id), 1);
        for (let i = buckets.length - 1; i >= 0; i--) if (buckets[i].planId === args.id) buckets.splice(i, 1);
        if (defaultPlan === args.id) defaultPlan = plans[0].id;
        changed();
        return null;
      }
      case 'list_buckets': return buckets.filter((b) => b.planId === args.planId).sort((a, b) => a.sort - b.sort);
      case 'create_bucket': {
        if (buckets.some((b) => b.planId === args.planId && b.name.toLowerCase() === String(args.name).toLowerCase())) throw { code: 'duplicate', message: 'duplicate' };
        const id = nextId++;
        buckets.push({ id, planId: args.planId, name: args.name, sort: buckets.filter((b) => b.planId === args.planId).length });
        changed();
        return id;
      }
      case 'rename_bucket': buckets.find((b) => b.id === args.id)!.name = args.name; changed(); return null;
      case 'reorder_buckets': reorder(buckets, args.ids); changed(); return null;
      case 'delete_bucket': {
        const b = buckets.find((x) => x.id === args.id)!;
        if (buckets.filter((x) => x.planId === b.planId).length <= 1) throw { code: 'last_bucket', message: 'last_bucket' };
        evacuate((t) => t.bucketId === args.id, args.mode, false);
        buckets.splice(buckets.indexOf(b), 1);
        changed();
        return null;
      }
      case 'move_task': {
        const t = all.find((x) => x.id === args.taskId)!;
        const b = buckets.find((x) => x.id === args.bucketId)!;
        Object.assign(t, { planId: b.planId, bucketId: b.id });
        const col = all.filter((x) => x.bucketId === b.id && x.id !== t.id).sort((p, q) => p.sortOrder - q.sortOrder);
        const at = args.before != null ? col.findIndex((x) => x.id === args.before) + 1 : args.after != null ? col.findIndex((x) => x.id === args.after) : col.length;
        col.splice(Math.max(at, 0), 0, t);
        col.forEach((x, i) => (x.sortOrder = i * 10));
        changed();
        return null;
      }
      case 'get_default_plan': return defaultPlan;
      case 'set_default_plan': defaultPlan = args.id; changed(); return null;
      case 'local_time_zone': return 'Europe/Rome';
      case 'take_pending_focus': return null;
      case 'search': return [];
      case 'parse_preview': {
        const t: string = args.text;
        const i = t.indexOf(' tomorrow at 1');
        return {
          kind: 'event', title: i > 0 ? t.slice(0, i) : t, start: { date: '2026-10-21', time: '13:00:00' },
          end: { date: '2026-10-21', time: '14:00:00' }, allDay: false, location: null, attendees: ['Sam'],
          calendar: null, recurrence: null, alerts: [], url: null, tags: [], plan: null,
          spans: i > 0 ? [{ start: 0, end: i, label: 'title' }, { start: i + 1, end: i + 9, label: 'date' }, { start: i + 10, end: i + 12, label: 'separator' }, { start: i + 13, end: i + 14, label: 'time' }] : [],
        };
      }
      case 'get_event': return { id: args.id, ...occ[0], attendees: [], notes: null, url: null, rrule: null, alerts: [], tz: 'Europe/Rome' };
      default: return null;
    }
  }, { shouldMockEvents: true });
}
