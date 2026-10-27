// Dev-only fake backend so the UI can be previewed in a normal browser
// (`npm run dev`, then open http://localhost:5173/?window=main|quick|settings).
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { lang } from '../lib/i18n';

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
    parentId: number | null; priority: 'urgent' | 'important' | 'medium' | 'low'; start: string | null; startHasTime: boolean;
    subtaskOpen: number; subtaskTotal: number; subtaskInProgress: number; checklistDone: number; checklistTotal: number; attachmentCount: number;
  };
  type MockItem = { id: number; taskId: number; text: string; done: boolean; sort: number };
  const task = (id: number, title: string, due: string | null, dueHasTime: boolean, status: MockTask['status'] = 'todo', lbl: number[] = []): MockTask => ({
    id, calendarId: 1, title, due, dueHasTime, tz: 'Europe/Rome', completedAt: status === 'done' ? d(0, 8) : null,
    recurring: false, rrule: null, notes: null, color: '#3b82f6', alerts: [], status, labels: L(...lbl),
    planId: 1, bucketId: 1, sortOrder: id * 10,
    parentId: null, priority: 'medium', start: null, startHasTime: false,
    subtaskOpen: 0, subtaskTotal: 0, subtaskInProgress: 0, checklistDone: 0, checklistTotal: 0, attachmentCount: 0,
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
  // Subtasks of "Quarterly report draft" (16), a spanning task and a checklist.
  all.push(
    { ...task(30, 'Collect figures', midnight(1), false), parentId: 16, sortOrder: 10, priority: 'important' },
    { ...task(31, 'Write summary', midnight(2), false), parentId: 16, sortOrder: 20 },
    { ...task(32, 'Review with team', null, false, 'done'), parentId: 16, sortOrder: 30 },
    // "Ship order #1042" (23) is still To do: starting one of these starts it.
    { ...task(33, 'Pack the order', null, false), parentId: 23, planId: 2, bucketId: 6, sortOrder: 10 },
    { ...task(34, 'Print the label', null, false, 'inProgress'), parentId: 23, planId: 2, bucketId: 6, sortOrder: 20 },
  );
  Object.assign(all.find((t) => t.id === 11)!, { priority: 'urgent' });
  // `?bulk=500` adds that many generated tasks (every fifth with two subtasks) to try the Grid at size.
  const bulk = Math.min(5000, Number(new URLSearchParams(location.search).get('bulk')) || 0);
  for (let i = 0; i < bulk; i++) {
    const id = 100_000 + i * 3, plan = i % 2 ? 2 : 1, bucket = plan === 1 ? 1 + (i % 3) : 4 + (i % 3);
    const st = (['todo', 'inProgress', 'done'] as const)[i % 3];
    const prio = (['urgent', 'important', 'medium', 'low'] as const)[i % 4];
    all.push({ ...task(id, `Bulk task ${i + 1}`, i % 4 ? midnight((i % 40) - 10) : null, false, st, i % 5 ? [] : [1 + (i % 3)]), planId: plan, bucketId: bucket, sortOrder: 1000 + i, priority: prio });
    if (i % 5 === 0) {
      for (const k of [1, 2]) all.push({ ...task(id + k, `Step ${k} of bulk ${i + 1}`, null, false, k === 1 ? 'done' : 'todo'), parentId: id, planId: plan, bucketId: bucket, sortOrder: k * 10 });
    }
  }
  Object.assign(all.find((t) => t.id === 12)!, { start: midnight(-1), due: midnight(1), priority: 'low' });
  let items: MockItem[] = [
    { id: 1, taskId: 17, text: 'Outline', done: true, sort: 0 },
    { id: 2, taskId: 17, text: 'Pick images', done: false, sort: 1 },
    { id: 3, taskId: 17, text: 'Rehearse', done: false, sort: 2 },
  ];
  type MockAtt = { id: number; taskId: number; kind: 'file' | 'link'; name: string; stored: string | null; url: string | null; size: number | null; createdAt: string; sort: number };
  let atts: MockAtt[] = [];
  let picks = 0;
  const nextSort = (taskId: number) => Math.max(-1, ...atts.filter((a) => a.taskId === taskId).map((a) => a.sort)) + 1;
  // A template keeps a copy of its task's content, like the store's TemplateBody (no dates, no attachments).
  type MockBody = { title: string; notes: string | null; labels: number[]; priority: MockTask['priority']; checklist: string[] };
  let templates: { id: number; name: string; subtaskCount: number; body: MockBody & { subtasks: MockBody[] } }[] = [];
  const bodyOf = (t: MockTask): MockBody => ({
    title: t.title, notes: t.notes, labels: t.labels.map((l) => l.id), priority: t.priority,
    checklist: items.filter((i) => i.taskId === t.id).sort((a, b) => a.sort - b.sort).map((i) => i.text),
  });
  const kidsOf = (id: number) => all.filter((t) => t.parentId === id).sort((a, b) => a.sortOrder - b.sortOrder);
  const addChecklist = (taskId: number, texts: string[], done: boolean[] = []) =>
    texts.forEach((text, n) => items.push({ id: nextId++, taskId, text, done: done[n] ?? false, sort: n }));
  /** A sort order right after `t` among its siblings (same parent, or same bucket for top-level tasks). */
  const sortAfter = (t: MockTask) => {
    const next = all
      .filter((x) => x.id !== t.id && x.parentId === t.parentId && (t.parentId != null || x.bucketId === t.bucketId) && x.sortOrder > t.sortOrder)
      .reduce((m, x) => Math.min(m, x.sortOrder), Infinity);
    return next === Infinity ? t.sortOrder + 10 : (t.sortOrder + next) / 2;
  };
  /** Copies a task's content under `parentId` (attachments too, except "missing" files, whose names go in `skipped`). */
  const copyTask = (src: MockTask, patch: Partial<MockTask>, skipped: string[]): MockTask => {
    const copy: MockTask = { ...src, labels: [...src.labels], id: nextId++, completedAt: null, status: 'todo', ...patch };
    all = [...all, copy];
    // Checklists come unticked.
    addChecklist(copy.id, items.filter((i) => i.taskId === src.id).sort((a, b) => a.sort - b.sort).map((i) => i.text));
    atts.filter((a) => a.taskId === src.id).forEach((a) => {
      if (a.kind === 'file' && a.stored?.includes('missing')) skipped.push(a.name);
      else atts.push({ ...a, id: nextId++, taskId: copy.id });
    });
    return copy;
  };
  const templateBy = (word: string) => {
    const key = (s: string) => [...s].filter((c) => /[\p{L}\p{N}]/u.test(c)).join('').toLowerCase();
    return templates.find((x) => key(x.name) === key(word));
  };
  /** Creates a task from a template: the input wins; the template's checklist and subtasks come along. */
  const fromTemplate = (tpl: (typeof templates)[number], i: any): number => {
    const b = tpl.body;
    const lbl = [...new Set([...labels.filter((l) => i.labels.includes(l.name)).map((l) => l.id), ...b.labels])];
    const t = task(nextId++, i.title, i.due, i.dueHasTime, 'todo', lbl);
    Object.assign(t, { notes: i.notes?.trim() ? i.notes : b.notes, priority: i.priority && i.priority !== 'medium' ? i.priority : b.priority });
    place(t, i.planId ?? defaultPlan, i.bucketId);
    all = [...all, t];
    addChecklist(t.id, b.checklist);
    b.subtasks.forEach((s, n) => {
      const sub = task(nextId++, s.title, null, false, 'todo', s.labels);
      Object.assign(sub, { notes: s.notes, priority: s.priority, parentId: t.id, sortOrder: (n + 1) * 10, planId: t.planId, bucketId: t.bucketId });
      all = [...all, sub];
      addChecklist(sub.id, s.checklist);
    });
    return t.id;
  };
  const cleanName = (n: unknown, max: number, code: string) => {
    const v = String(n ?? '').trim();
    if (!v || [...v].length > max) throw { code, message: code };
    return v;
  };
  const attOf = (id: number) => {
    const a = atts.find((x) => x.id === id);
    if (!a) throw { code: 'not_found', message: 'not_found' };
    return a;
  };
  let defaultPlan = 1;
  let nextId = 1000;
  const openCount = (planId: number) => all.filter((t) => t.planId === planId && !t.completedAt && t.parentId == null).length;
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
  // Keeps the counts on every returned task consistent with the data.
  const withCounts = (t: MockTask): MockTask => {
    const kids = all.filter((x) => x.parentId === t.id);
    const list = items.filter((i) => i.taskId === t.id);
    return Object.assign(t, {
      subtaskTotal: kids.length, subtaskOpen: kids.filter((k) => !k.completedAt).length,
      subtaskInProgress: kids.filter((k) => k.status === 'inProgress').length,
      checklistTotal: list.length, checklistDone: list.filter((i) => i.done).length,
      attachmentCount: atts.filter((a) => a.taskId === t.id).length,
    });
  };
  const changed = () => setTimeout(() => emit('items-changed'), 0);
  const setStatus = (t: MockTask, status: MockTask['status']) => {
    const started = status === 'inProgress' && t.status !== 'inProgress';
    t.status = status;
    t.completedAt = status === 'done' ? new Date().toISOString() : null;
    // As the store does: a subtask put in progress (a change, not a re-save) starts a To do parent.
    const parent = started && t.parentId != null ? all.find((x) => x.id === t.parentId) : undefined;
    if (parent?.status === 'todo') parent.status = 'inProgress';
  };
  // A bucket decides its plan; a plan alone means its first bucket (placed last).
  const place = (t: MockTask, planId?: number | null, bucketId?: number | null) => {
    if (bucketId == null && (planId == null || planId === t.planId)) return;
    const b = bucketId != null ? buckets.find((x) => x.id === bucketId)! : buckets.filter((x) => x.planId === planId).sort((p, q) => p.sort - q.sort)[0];
    if (b.id === t.bucketId) return;
    Object.assign(t, { planId: b.planId, bucketId: b.id, sortOrder: Math.max(0, ...all.filter((x) => x.bucketId === b.id).map((x) => x.sortOrder)) + 10 });
  };
  mockIPC((cmd, args: any) => {
    switch (cmd) {
      case 'list_calendars': return calendars;
      case 'get_settings': return { hotkey: 'Alt+Super+Space', launchAtLogin: false, defaultCalendarId: 1, defaultEventMinutes: 60, weekStart: 'Mon', dateOrder: 'dayMonth' };
      case 'list_range': return occ;
      case 'list_tasks': return byFilter[args.filter]().map(withCounts);
      case 'get_task': return withCounts(all.find((t) => t.id === args.id)!);
      case 'set_task_status': setStatus(all.find((t) => t.id === args.id)!, args.status); changed(); return null;
      case 'set_task_completed': setStatus(all.find((t) => t.id === args.id)!, args.done ? 'done' : 'todo'); changed(); return null;
      case 'update_task': {
        const t = all.find((x) => x.id === args.id)!;
        const i = args.input;
        if (i.start && i.due && new Date(i.start) > new Date(i.due)) throw { code: 'start_after_due', message: 'start_after_due' };
        if (i.parentId != null && all.some((x) => x.parentId === t.id)) throw { code: 'has_subtasks', message: 'has_subtasks' };
        Object.assign(t, {
          title: i.title, due: i.due, dueHasTime: i.dueHasTime, notes: i.notes, labels: labels.filter((l) => i.labels.includes(l.name)),
          ...(i.priority ? { priority: i.priority } : {}), ...(i.start !== undefined ? { start: i.start, startHasTime: !!i.startHasTime } : {}),
          ...(i.parentId !== undefined ? { parentId: i.parentId } : {}),
        });
        setStatus(t, i.status);
        place(t, i.planId, i.bucketId);
        changed();
        return null;
      }
      case 'create_task': {
        const i = args.input;
        if (i.start && i.due && new Date(i.start) > new Date(i.due)) throw { code: 'start_after_due', message: 'start_after_due' };
        if (i.parentId != null && all.find((x) => x.id === i.parentId)?.parentId != null) throw { code: 'parent_is_subtask', message: 'parent_is_subtask' };
        const t = task(nextId++, i.title, i.due, i.dueHasTime, i.status, labels.filter((l) => i.labels.includes(l.name)).map((l) => l.id));
        Object.assign(t, { priority: i.priority ?? 'medium', start: i.start ?? null, startHasTime: !!i.startHasTime, parentId: i.parentId ?? null });
        if (i.parentId != null) t.sortOrder = Math.max(0, ...all.filter((x) => x.parentId === i.parentId).map((x) => x.sortOrder)) + 10;
        place(t, i.planId ?? defaultPlan, i.bucketId);
        all = [...all, t];
        changed();
        return t.id;
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
        const tpl = [...String(args.text).matchAll(/\+(\S+)/g)].map((m) => templateBy(m[1])).find(Boolean);
        if (tpl) {
          const title = String(args.text).replace(/\+\S+/, (w) => (templateBy(w.slice(1)) === tpl ? '' : w)).replace(/\s+/g, ' ').trim();
          const id = fromTemplate(tpl, { title, due: null, dueHasTime: false, labels: [], notes: null, planId: null, bucketId: null });
          changed();
          return { kind: 'task', id };
        }
        const tags = [...String(args.text).matchAll(/#(\w+)/g)].map((m) => m[1]);
        const title = String(args.text).replace(/#\w+/g, '').trim();
        const lbl = tags.map((n) => labels.find((l) => l.name === n)?.id).filter((x): x is number => !!x);
        const id = nextId++;
        all = [...all, task(id, title, null, false, 'todo', lbl)];
        changed();
        return { kind: 'task', id };
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
      case 'delete_task': {
        const gone = new Set(all.filter((t) => t.id === args.id || t.parentId === args.id).map((t) => t.id));
        all = all.filter((t) => !gone.has(t.id));
        items = items.filter((i) => !gone.has(i.taskId));
        atts = atts.filter((a) => !gone.has(a.taskId));
        changed();
        return null;
      }
      case 'list_subtasks': return all.filter((t) => t.parentId === args.parentId).sort((a, b) => a.sortOrder - b.sortOrder).map(withCounts);
      case 'reorder_subtasks': args.ids.forEach((id: number, i: number) => { const x = all.find((y) => y.id === id); if (x) x.sortOrder = (i + 1) * 10; }); changed(); return null;
      case 'set_task_parent': {
        const t = all.find((x) => x.id === args.taskId)!;
        if (args.parentId != null) {
          if (all.find((x) => x.id === args.parentId)?.parentId != null) throw { code: 'parent_is_subtask', message: 'parent_is_subtask' };
          if (all.some((x) => x.parentId === t.id)) throw { code: 'has_subtasks', message: 'has_subtasks' };
          if (t.recurring) throw { code: 'repeating_subtask', message: 'repeating_subtask' };
        }
        t.parentId = args.parentId;
        t.sortOrder = Math.max(0, ...all.filter((x) => x.parentId === args.parentId && x.id !== t.id).map((x) => x.sortOrder)) + 10;
        changed();
        return null;
      }
      case 'complete_with_subtasks':
        all.filter((t) => t.id === args.id || t.parentId === args.id).forEach((t) => !t.completedAt && setStatus(t, 'done'));
        changed();
        return null;
      case 'list_checklist': return items.filter((i) => i.taskId === args.taskId).sort((a, b) => a.sort - b.sort);
      case 'add_checklist_item': {
        const text = String(args.text).trim();
        if (!text || text.length > 200) throw { code: 'checklist_text', message: 'checklist_text' };
        const id = nextId++;
        items.push({ id, taskId: args.taskId, text, done: false, sort: items.filter((i) => i.taskId === args.taskId).length });
        changed();
        return id;
      }
      case 'update_checklist_item': {
        const i = items.find((x) => x.id === args.id)!;
        if (args.text != null) {
          const text = String(args.text).trim();
          if (!text || text.length > 200) throw { code: 'checklist_text', message: 'checklist_text' };
          i.text = text;
        }
        if (args.done != null) i.done = args.done;
        changed();
        return null;
      }
      case 'reorder_checklist': reorder(items.filter((i) => i.taskId === args.taskId), args.ids); changed(); return null;
      case 'delete_checklist_item': items = items.filter((i) => i.id !== args.id); changed(); return null;
      case 'checklist_item_to_subtask': {
        const i = items.find((x) => x.id === args.id)!;
        const parent = all.find((x) => x.id === i.taskId)!;
        const t = task(nextId++, i.text, null, false);
        Object.assign(t, { parentId: parent.id, planId: parent.planId, bucketId: parent.bucketId, sortOrder: Math.max(0, ...all.filter((x) => x.parentId === parent.id).map((x) => x.sortOrder)) + 10 });
        if (i.done) setStatus(t, 'done');
        all = [...all, t];
        items = items.filter((x) => x.id !== i.id);
        changed();
        return t.id;
      }
      case 'list_attachments': return atts.filter((a) => a.taskId === args.taskId).sort((a, b) => a.sort - b.sort);
      case 'pick_and_add_files': {
        // Cycles through three picks; every third also reports a file that was too large, and its
        // "missing" file fails to open (not_found), so both toasts can be checked by eye.
        const n = picks++ % 3;
        const [name, size] = ([['notes.pdf', 1_250_000], ['photo.jpg', 482_000], ['missing.docx', 18_400]] as const)[n];
        const a: MockAtt = { id: nextId++, taskId: args.taskId, kind: 'file', name, stored: `mock-${name}`, url: null, size, createdAt: new Date().toISOString(), sort: nextSort(args.taskId) };
        atts.push(a);
        changed();
        return { added: [a], errors: n === 2 ? [{ name: 'holiday-video.mov', code: 'attachment_too_large' }] : [] };
      }
      case 'add_link_attachment': {
        const url = String(args.url).trim();
        if (!/^https?:\/\/\S+/i.test(url)) throw { code: 'attachment_url', message: 'attachment_url' };
        const a: MockAtt = { id: nextId++, taskId: args.taskId, kind: 'link', name: args.name?.trim() || new URL(url).hostname.replace(/^www\./, ''), stored: null, url, size: null, createdAt: new Date().toISOString(), sort: nextSort(args.taskId) };
        atts.push(a);
        changed();
        return a;
      }
      case 'rename_attachment': attOf(args.id).name = cleanName(args.name, 200, 'empty'); changed(); return null;
      case 'reorder_attachments': reorder(atts.filter((a) => a.taskId === args.taskId), args.ids); changed(); return null;
      case 'remove_attachment': { const a = attOf(args.id); atts = atts.filter((x) => x.id !== a.id); changed(); return a; }
      case 'restore_file_attachment': {
        const a: MockAtt = { id: nextId++, taskId: args.taskId, kind: 'file', name: args.name, stored: args.stored, url: null, size: args.size, createdAt: new Date().toISOString(), sort: nextSort(args.taskId) };
        atts.push(a);
        changed();
        return a;
      }
      case 'open_attachment':
      case 'reveal_attachment': {
        const a = attOf(args.id);
        if (a.stored?.includes('missing')) throw { code: 'not_found', message: 'not_found' };
        return null;
      }
      case 'duplicate_task': {
        // Like the store: title + the UI language's suffix, right after the original, subtasks copied;
        // a file attachment named "missing…" stands for one whose file is gone (reported in `skipped`).
        const src = all.find((t) => t.id === args.id);
        if (!src) throw { code: 'not_found', message: 'not_found' };
        const skipped: string[] = [];
        const suffix = lang === 'it' ? ' (copia)' : ' (copy)';
        const copy = copyTask(src, { title: `${src.title}${suffix}`, sortOrder: sortAfter(src) }, skipped);
        kidsOf(src.id).forEach((k) => copyTask(k, { parentId: copy.id, planId: copy.planId, bucketId: copy.bucketId }, skipped));
        changed();
        return { id: copy.id, skipped };
      }
      case 'save_template': {
        const name = cleanName(args.name, 60, 'template_name');
        const dup = templates.find((x) => x.name.toLowerCase() === name.toLowerCase());
        if (dup && !args.replace) throw { code: 'duplicate', message: 'duplicate' };
        const src = all.find((t) => t.id === args.taskId);
        if (!src) throw { code: 'not_found', message: 'not_found' };
        if (src.parentId != null) throw { code: 'invalid', message: 'A subtask can\'t be a template' };
        const subs = kidsOf(src.id);
        const body = { ...bodyOf(src), subtasks: subs.map(bodyOf) };
        if (dup) { Object.assign(dup, { subtaskCount: subs.length, body }); changed(); return dup.id; }
        const id = nextId++;
        templates.push({ id, name, subtaskCount: subs.length, body });
        changed();
        return id;
      }
      case 'list_templates': return templates.map(({ id, name, subtaskCount }) => ({ id, name, subtaskCount })).sort((a, b) => a.name.localeCompare(b.name));
      case 'rename_template': {
        const name = cleanName(args.name, 60, 'template_name');
        if (templates.some((x) => x.id !== args.id && x.name.toLowerCase() === name.toLowerCase())) throw { code: 'duplicate', message: 'duplicate' };
        const tpl = templates.find((x) => x.id === args.id);
        if (!tpl) throw { code: 'not_found', message: 'not_found' };
        tpl.name = name;
        changed();
        return null;
      }
      case 'delete_template': templates = templates.filter((x) => x.id !== args.id); changed(); return null;
      case 'create_from_template': {
        const tpl = templates.find((x) => x.id === args.templateId);
        if (!tpl) throw { code: 'not_found', message: 'not_found' };
        const id = fromTemplate(tpl, args.input);
        changed();
        return id;
      }
      case 'get_default_plan': return defaultPlan;
      case 'set_default_plan': defaultPlan = args.id; changed(); return null;
      case 'local_time_zone': return 'Europe/Rome';
      case 'take_pending_focus': return null;
      case 'search': return [];
      case 'parse_preview': {
        const t: string = args.text;
        // A task with "+name" of a saved template: highlight it (spans are UTF-8 byte offsets).
        const plus = [...t.matchAll(/\+(\S+)/g)].find((m) => templateBy(m[1]));
        if (plus) {
          const bytes = (s: string) => new TextEncoder().encode(s).length;
          const start = bytes(t.slice(0, plus.index)), end = start + bytes(plus[0]);
          return {
            kind: 'task', title: (t.slice(0, plus.index) + t.slice(plus.index! + plus[0].length)).replace(/\s+/g, ' ').trim(), start: null, end: null,
            allDay: false, location: null, attendees: [], calendar: null, recurrence: null, alerts: [], url: null, tags: [], plan: null, priority: null,
            template: templateBy(plus[1])!.name,
            spans: [...(start > 0 ? [{ start: 0, end: start, label: 'title' }] : []), { start, end, label: 'template' }],
          };
        }
        const i = t.indexOf(' tomorrow at 1');
        return {
          kind: 'event', title: i > 0 ? t.slice(0, i) : t, start: { date: '2026-10-21', time: '13:00:00' },
          end: { date: '2026-10-21', time: '14:00:00' }, allDay: false, location: null, attendees: ['Sam'],
          calendar: null, recurrence: null, alerts: [], url: null, tags: [], plan: null, priority: null, template: null,
          spans: i > 0 ? [{ start: 0, end: i, label: 'title' }, { start: i + 1, end: i + 9, label: 'date' }, { start: i + 10, end: i + 12, label: 'separator' }, { start: i + 13, end: i + 14, label: 'time' }] : [],
        };
      }
      case 'get_event': return { id: args.id, ...occ[0], attendees: [], notes: null, url: null, rrule: null, alerts: [], tz: 'Europe/Rome' };
      default: return null;
    }
  }, { shouldMockEvents: true });
}
