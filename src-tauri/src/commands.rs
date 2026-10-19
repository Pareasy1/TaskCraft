//! Tauri commands. Logic lives in plain `*_impl` functions over `AppState`
//! so it can be tested without a running app.

use crate::settings::Settings;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use taskcraft_parse::{parse, ParseOptions, ParsedItem};
use taskcraft_store::*;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl CmdError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        CmdError { code: code.into(), message: message.into() }
    }
}

impl From<StoreError> for CmdError {
    fn from(e: StoreError) -> Self {
        tracing::warn!(error = %e, "store error");
        let code = match e {
            StoreError::NotFound => "not_found",
            StoreError::Invalid(_) => "invalid",
            StoreError::Db(_) => "db",
            StoreError::Io(_) => "io",
        };
        CmdError::new(code, e.to_string())
    }
}

fn poisoned<T>(_: T) -> CmdError {
    CmdError::new("internal", "internal state lock poisoned")
}

type CmdResult<T> = std::result::Result<T, CmdError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    pub kind: ItemKindDb,
    pub id: i64,
}

/// The Mac's current time zone (falls back to UTC).
pub fn local_tz() -> Tz {
    iana_time_zone::get_timezone().ok().and_then(|n| n.parse().ok()).unwrap_or(Tz::UTC)
}

pub fn local_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&local_tz())
}

fn parse_options(st: &AppState) -> CmdResult<ParseOptions> {
    let settings = st.settings.read().map_err(poisoned)?.clone();
    let calendars = st.store.lock().map_err(poisoned)?.calendars()?.into_iter().map(|c| c.name).collect();
    Ok(ParseOptions { default_event_minutes: settings.default_event_minutes, calendars, date_order: settings.date_order })
}

pub fn parse_preview_impl(st: &AppState, text: &str, now: DateTime<Tz>) -> CmdResult<ParsedItem> {
    Ok(parse(text, now, &parse_options(st)?))
}

pub fn create_from_text_impl(st: &AppState, text: &str, now: DateTime<Tz>) -> CmdResult<ItemRef> {
    if text.trim().is_empty() {
        return Err(CmdError::new("empty", "Type something to create an item"));
    }
    let opts = parse_options(st)?;
    let p = parse(text, now, &opts);
    let settings = st.settings.read().map_err(poisoned)?.clone();
    let store = st.store.lock().map_err(poisoned)?;
    let calendars = store.calendars()?;
    let default_cal = calendars
        .iter()
        .find(|c| c.id == settings.default_calendar_id)
        .or_else(|| calendars.first())
        .map(|c| c.id)
        .ok_or_else(|| CmdError::new("no_calendar", "Create a calendar first"))?;
    let item = from_parsed(&p, now.timezone(), default_cal, &calendars, settings.default_event_minutes);
    let r = match item {
        NewItem::Event(e) => ItemRef { kind: ItemKindDb::Event, id: store.insert_event(&e)? },
        NewItem::Task(t) => ItemRef { kind: ItemKindDb::Task, id: store.insert_task(&t)? },
    };
    tracing::info!(kind = ?r.kind, id = r.id, "created from text");
    Ok(r)
}

fn changed(app: &AppHandle) {
    let _ = app.emit("items-changed", ());
}

// ---------- commands ----------

#[tauri::command]
pub fn parse_preview(st: State<AppState>, text: String) -> CmdResult<ParsedItem> {
    parse_preview_impl(&st, &text, local_now())
}

#[tauri::command]
pub fn create_from_text(app: AppHandle, st: State<AppState>, text: String) -> CmdResult<ItemRef> {
    let r = create_from_text_impl(&st, &text, local_now())?;
    changed(&app);
    Ok(r)
}

#[tauri::command]
pub fn get_event(st: State<AppState>, id: i64) -> CmdResult<Event> {
    Ok(st.store.lock().map_err(poisoned)?.get_event(id)?)
}

#[tauri::command]
pub fn create_event(app: AppHandle, st: State<AppState>, input: EventInput) -> CmdResult<i64> {
    let id = st.store.lock().map_err(poisoned)?.insert_event(&input)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_event(app: AppHandle, st: State<AppState>, id: i64, input: EventInput) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.update_event(id, &input)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn move_event(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    occurrence: Option<DateTime<Utc>>,
    new_start: DateTime<Utc>,
    new_end: DateTime<Utc>,
    scope: Scope,
) -> CmdResult<i64> {
    let r = st.store.lock().map_err(poisoned)?.move_event(id, occurrence, new_start, new_end, scope)?;
    changed(&app);
    Ok(r)
}

#[tauri::command]
pub fn delete_event(app: AppHandle, st: State<AppState>, id: i64, occurrence: Option<DateTime<Utc>>, scope: Scope) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_event(id, occurrence, scope)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_task(st: State<AppState>, id: i64) -> CmdResult<Task> {
    Ok(st.store.lock().map_err(poisoned)?.get_task(id)?)
}

#[tauri::command]
pub fn create_task(app: AppHandle, st: State<AppState>, input: TaskInput) -> CmdResult<i64> {
    let id = st.store.lock().map_err(poisoned)?.insert_task(&input)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_task(app: AppHandle, st: State<AppState>, id: i64, input: TaskInput) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.update_task(id, &input)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_task_completed(app: AppHandle, st: State<AppState>, id: i64, done: bool) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.set_task_completed(id, done, Utc::now())?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_task(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_task(id)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_range(st: State<AppState>, start: DateTime<Utc>, end: DateTime<Utc>) -> CmdResult<Vec<Occurrence>> {
    Ok(st.store.lock().map_err(poisoned)?.list_range(start, end)?)
}

#[tauri::command]
pub fn list_tasks(st: State<AppState>, filter: TaskFilter) -> CmdResult<Vec<Task>> {
    Ok(st.store.lock().map_err(poisoned)?.list_tasks(filter, Utc::now(), local_tz())?)
}

#[tauri::command]
pub fn search(st: State<AppState>, q: String) -> CmdResult<Vec<Hit>> {
    Ok(st.store.lock().map_err(poisoned)?.search(&q)?)
}

#[tauri::command]
pub fn list_calendars(st: State<AppState>) -> CmdResult<Vec<Calendar>> {
    Ok(st.store.lock().map_err(poisoned)?.calendars()?)
}

#[tauri::command]
pub fn upsert_calendar(app: AppHandle, st: State<AppState>, input: CalendarInput) -> CmdResult<i64> {
    let id = st.store.lock().map_err(poisoned)?.upsert_calendar(&input)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn delete_calendar(app: AppHandle, st: State<AppState>, id: i64, move_to: i64) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_calendar(id, move_to)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_settings(st: State<AppState>) -> CmdResult<Settings> {
    Ok(st.settings.read().map_err(poisoned)?.clone())
}

#[tauri::command]
pub fn export_ics(st: State<AppState>, path: String) -> CmdResult<()> {
    let ics = st.store.lock().map_err(poisoned)?.export_ics()?;
    std::fs::write(&path, ics).map_err(|e| CmdError::new("io", e.to_string()))?;
    tracing::info!(%path, "exported ics");
    Ok(())
}

#[tauri::command]
pub fn local_time_zone() -> String {
    local_tz().name().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use chrono::TimeZone;
    use chrono_tz::Europe::Rome;
    use taskcraft_store::ItemKindDb;

    fn now() -> chrono::DateTime<chrono_tz::Tz> {
        Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
    }

    #[test]
    fn creates_event_in_default_calendar() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "Lunch with Sam tomorrow at 1", now()).unwrap();
        assert_eq!(r.kind, ItemKindDb::Event);
        let e = st.store.lock().unwrap().get_event(r.id).unwrap().input;
        assert_eq!(e.title, "Lunch");
        assert_eq!(e.calendar_id, st.settings.read().unwrap().default_calendar_id);
        assert_eq!(e.attendees, vec!["Sam"]);
    }

    #[test]
    fn creates_task_from_prefix() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "todo buy milk friday", now()).unwrap();
        assert_eq!(r.kind, ItemKindDb::Task);
        assert_eq!(st.store.lock().unwrap().get_task(r.id).unwrap().title, "buy milk");
    }

    #[test]
    fn slash_calendar_overrides_default() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "Standup tomorrow 9am /work", now()).unwrap();
        let store = st.store.lock().unwrap();
        let work = store.calendars().unwrap().into_iter().find(|c| c.name == "Work").unwrap().id;
        assert_eq!(store.get_event(r.id).unwrap().input.calendar_id, work);
    }

    #[test]
    fn empty_text_is_rejected() {
        let st = AppState::in_memory();
        let err = create_from_text_impl(&st, "   ", now()).unwrap_err();
        assert_eq!(err.code, "empty");
    }
}
