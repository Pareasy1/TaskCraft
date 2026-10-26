// Top-level areas and the calendar's Day/Week/Month mode.

export type Area = 'calendar' | 'tasks' | 'grid';
export type CalendarMode = 'day' | 'week' | 'month';

const KEY = 'taskcraft.calendarMode';
const MODES: CalendarMode[] = ['day', 'week', 'month'];

export function loadCalendarMode(): CalendarMode {
  try {
    const v = localStorage.getItem(KEY);
    if (MODES.includes(v as CalendarMode)) return v as CalendarMode;
  } catch { /* storage unavailable */ }
  return 'week';
}

export function saveCalendarMode(m: CalendarMode) {
  try {
    localStorage.setItem(KEY, m);
  } catch { /* storage unavailable */ }
}

const AREAS: Record<string, Area> = { '1': 'calendar', '2': 'tasks', '3': 'grid' };
const LETTERS: Record<string, CalendarMode> = { d: 'day', w: 'week', m: 'month' };

/** What a key press changes: Mod+1/2/3 pick the area (Mod+4 is reserved); D/W/M pick the calendar mode. */
export function shortcutFor(
  e: { key: string; mod: boolean; inTextField: boolean },
  area: Area,
): { area?: Area; mode?: CalendarMode } | null {
  if (e.mod) return AREAS[e.key] ? { area: AREAS[e.key] } : null;
  if (e.inTextField || area !== 'calendar') return null;
  const mode = LETTERS[e.key.toLowerCase()];
  return mode ? { mode } : null;
}
