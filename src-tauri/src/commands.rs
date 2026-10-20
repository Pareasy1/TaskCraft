//! Tauri commands. Logic lives in plain `*_impl` functions over `AppState`
//! so it can be tested without a running app.

use crate::settings::Settings;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use taskcraft_parse::{parse, parse_task, ParseOptions, ParsedItem};
use taskcraft_store::*;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl CmdError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        CmdError {
            code: code.into(),
            message: message.into(),
        }
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
    iana_time_zone::get_timezone()
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(Tz::UTC)
}

pub fn local_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&local_tz())
}

fn parse_options(st: &AppState) -> CmdResult<ParseOptions> {
    let settings = st.settings.read().map_err(poisoned)?.clone();
    let calendars = st
        .store
        .lock()
        .map_err(poisoned)?
        .calendars()?
        .into_iter()
        .map(|c| c.name)
        .collect();
    Ok(ParseOptions {
        default_event_minutes: settings.default_event_minutes,
        calendars,
        date_order: settings.date_order,
    })
}

pub fn parse_preview_impl(
    st: &AppState,
    text: &str,
    now: DateTime<Tz>,
    as_task: bool,
) -> CmdResult<ParsedItem> {
    let opts = parse_options(st)?;
    Ok(if as_task {
        parse_task(text, now, &opts)
    } else {
        parse(text, now, &opts)
    })
}

pub fn create_from_text_impl(
    st: &AppState,
    text: &str,
    now: DateTime<Tz>,
    as_task: bool,
) -> CmdResult<ItemRef> {
    if text.trim().is_empty() {
        return Err(CmdError::new("empty", "Type something to create an item"));
    }
    let p = parse_preview_impl(st, text, now, as_task)?;
    let settings = st.settings.read().map_err(poisoned)?.clone();
    let store = st.store.lock().map_err(poisoned)?;
    let calendars = store.calendars()?;
    let default_cal = calendars
        .iter()
        .find(|c| c.id == settings.default_calendar_id)
        .or_else(|| calendars.first())
        .map(|c| c.id)
        .ok_or_else(|| CmdError::new("no_calendar", "Create a calendar first"))?;
    let item = from_parsed(
        &p,
        now.timezone(),
        default_cal,
        &calendars,
        settings.default_event_minutes,
    );
    let r = match item {
        NewItem::Event(e) => ItemRef {
            kind: ItemKindDb::Event,
            id: store.insert_event(&e)?,
        },
        NewItem::Task(t) => ItemRef {
            kind: ItemKindDb::Task,
            id: store.insert_task(&t)?,
        },
    };
    tracing::info!(kind = ?r.kind, id = r.id, "created from text");
    Ok(r)
}

fn changed(app: &AppHandle) {
    let _ = app.emit("items-changed", ());
}

// ---------- commands ----------

#[tauri::command]
pub fn parse_preview(
    st: State<AppState>,
    text: String,
    as_task: Option<bool>,
) -> CmdResult<ParsedItem> {
    parse_preview_impl(&st, &text, local_now(), as_task.unwrap_or(false))
}

#[tauri::command]
pub fn create_from_text(
    app: AppHandle,
    st: State<AppState>,
    text: String,
    as_task: Option<bool>,
) -> CmdResult<ItemRef> {
    let r = create_from_text_impl(&st, &text, local_now(), as_task.unwrap_or(false))?;
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
pub fn update_event(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    input: EventInput,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .update_event(id, &input)?;
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
    let r = st
        .store
        .lock()
        .map_err(poisoned)?
        .move_event(id, occurrence, new_start, new_end, scope)?;
    changed(&app);
    Ok(r)
}

#[tauri::command]
pub fn delete_event(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    occurrence: Option<DateTime<Utc>>,
    scope: Scope,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .delete_event(id, occurrence, scope)?;
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
pub fn update_task(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    input: TaskInput,
) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.update_task(id, &input)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_task_completed(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    done: bool,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .set_task_completed(id, done, Utc::now())?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_task_status(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    status: TaskStatus,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .set_task_status(id, status, Utc::now())?;
    changed(&app);
    Ok(())
}

/// The UI reports the language it picked from the Mac's settings; the
/// menu-bar menu, settings window title and notifications follow it.
#[tauri::command]
pub fn set_ui_language(app: AppHandle, st: State<AppState>, lang: String) -> CmdResult<()> {
    let lang = crate::i18n::Lang::from_code(&lang);
    *st.lang.write().map_err(poisoned)? = lang;
    if let Err(e) = crate::tray::set_menu_language(&app, lang) {
        tracing::warn!(error = %e, "cannot update tray menu language");
    }
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_title(crate::i18n::text(lang).settings_window);
    }
    Ok(())
}

#[tauri::command]
pub fn list_labels(st: State<AppState>) -> CmdResult<Vec<TaskLabel>> {
    Ok(st.store.lock().map_err(poisoned)?.list_labels()?)
}

fn is_hex_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

pub fn update_label_impl(st: &AppState, id: i64, name: &str, color: &str) -> CmdResult<()> {
    if !is_hex_color(color) {
        return Err(CmdError::new("invalid", "Pick a label color"));
    }
    st.store
        .lock()
        .map_err(poisoned)?
        .update_label(id, name, color)?;
    Ok(())
}

#[tauri::command]
pub fn update_label(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    name: String,
    color: String,
) -> CmdResult<()> {
    update_label_impl(&st, id, &name, &color)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_label(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_label(id)?;
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
pub fn list_range(
    st: State<AppState>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> CmdResult<Vec<Occurrence>> {
    Ok(st.store.lock().map_err(poisoned)?.list_range(start, end)?)
}

#[tauri::command]
pub fn list_tasks(st: State<AppState>, filter: TaskFilter) -> CmdResult<Vec<Task>> {
    Ok(st
        .store
        .lock()
        .map_err(poisoned)?
        .list_tasks(filter, Utc::now(), local_tz())?)
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
pub fn upsert_calendar(
    app: AppHandle,
    st: State<AppState>,
    input: CalendarInput,
) -> CmdResult<i64> {
    let id = st.store.lock().map_err(poisoned)?.upsert_calendar(&input)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn delete_calendar(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    move_to: i64,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .delete_calendar(id, move_to)?;
    changed(&app);
    Ok(())
}

/// Validates and applies settings. `register(new, old)` swaps the global
/// hotkey; if it fails nothing is changed.
pub fn set_settings_impl(
    st: &AppState,
    new: Settings,
    register: impl FnOnce(&str, &str) -> std::result::Result<(), String>,
) -> CmdResult<Settings> {
    if new
        .hotkey
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .is_err()
    {
        return Err(CmdError::new(
            "hotkey_invalid",
            format!("“{}” is not a valid shortcut", new.hotkey),
        ));
    }
    if !(5..=24 * 60).contains(&new.default_event_minutes) {
        return Err(CmdError::new(
            "invalid",
            "Default event length must be between 5 minutes and 24 hours",
        ));
    }
    let cals = st.store.lock().map_err(poisoned)?.calendars()?;
    if !cals.iter().any(|c| c.id == new.default_calendar_id) {
        return Err(CmdError::new("invalid", "Default calendar does not exist"));
    }
    let old = st.settings.read().map_err(poisoned)?.clone();
    if old.hotkey != new.hotkey {
        register(&new.hotkey, &old.hotkey).map_err(|e| {
            CmdError::new("hotkey_unavailable", format!("Shortcut unavailable: {e}"))
        })?;
    }
    if let Some(path) = &st.settings_path {
        new.save(path)
            .map_err(|e| CmdError::new("io", e.to_string()))?;
    }
    *st.settings.write().map_err(poisoned)? = new.clone();
    Ok(new)
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    st: State<AppState>,
    settings: Settings,
) -> CmdResult<Settings> {
    let launch_changed =
        st.settings.read().map_err(poisoned)?.launch_at_login != settings.launch_at_login;
    let saved = set_settings_impl(&st, settings, |new, old| {
        crate::tray::register_hotkey(&app, new, Some(old))
    })?;
    if launch_changed {
        use tauri_plugin_autostart::ManagerExt;
        let al = app.autolaunch();
        let r = if saved.launch_at_login {
            al.enable()
        } else {
            al.disable()
        };
        if let Err(e) = r {
            tracing::warn!(error = %e, "autostart toggle failed");
            return Err(CmdError::new("autostart", e.to_string()));
        }
    }
    changed(&app);
    Ok(saved)
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
        let r = create_from_text_impl(&st, "Lunch with Sam tomorrow at 1", now(), false).unwrap();
        assert_eq!(r.kind, ItemKindDb::Event);
        let e = st.store.lock().unwrap().get_event(r.id).unwrap().input;
        assert_eq!(e.title, "Lunch");
        assert_eq!(
            e.calendar_id,
            st.settings.read().unwrap().default_calendar_id
        );
        assert_eq!(e.attendees, vec!["Sam"]);
    }

    #[test]
    fn update_label_validates() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "pay rent #bills friday", now(), true).unwrap();
        let t = st.store.lock().unwrap().get_task(r.id).unwrap();
        let names: Vec<&str> = t.labels.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["bills"], "integration: #tags become labels");
        let id = t.labels[0].id;
        assert_eq!(
            update_label_impl(&st, id, "home", "red").unwrap_err().code,
            "invalid"
        );
        assert_eq!(
            update_label_impl(&st, id, "", "#123456").unwrap_err().code,
            "invalid"
        );
        update_label_impl(&st, id, "home", "#123456").unwrap();
        let l = st.store.lock().unwrap().list_labels().unwrap();
        assert_eq!(
            (l[0].name.as_str(), l[0].color.as_str()),
            ("home", "#123456")
        );
    }

    #[test]
    fn as_task_creates_task_without_prefix() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "call mom tomorrow 9am", now(), true).unwrap();
        assert_eq!(r.kind, ItemKindDb::Task);
        let t = st.store.lock().unwrap().get_task(r.id).unwrap();
        assert_eq!(t.title, "call mom");
        assert!(t.due_has_time);
        let p = parse_preview_impl(&st, "buy milk", now(), true).unwrap();
        assert_eq!(p.kind, taskcraft_parse::ItemKind::Task);
        assert_eq!(p.start, None);
    }

    #[test]
    fn creates_task_from_prefix() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "todo buy milk friday", now(), false).unwrap();
        assert_eq!(r.kind, ItemKindDb::Task);
        assert_eq!(
            st.store.lock().unwrap().get_task(r.id).unwrap().title,
            "buy milk"
        );
    }

    #[test]
    fn slash_calendar_overrides_default() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "Standup tomorrow 9am /work", now(), false).unwrap();
        let store = st.store.lock().unwrap();
        let work = store
            .calendars()
            .unwrap()
            .into_iter()
            .find(|c| c.name == "Work")
            .unwrap()
            .id;
        assert_eq!(store.get_event(r.id).unwrap().input.calendar_id, work);
    }

    #[test]
    fn set_settings_rejects_bad_hotkey() {
        let st = AppState::in_memory();
        let before = st.settings.read().unwrap().clone();
        let mut bad = before.clone();
        bad.hotkey = "Nope+X".into();
        bad.default_event_minutes = 30;
        let err = set_settings_impl(&st, bad, |_, _| Ok(())).unwrap_err();
        assert_eq!(err.code, "hotkey_invalid");
        assert_eq!(*st.settings.read().unwrap(), before);
    }

    #[test]
    fn set_settings_keeps_old_hotkey_when_registration_fails() {
        let st = AppState::in_memory();
        let before = st.settings.read().unwrap().clone();
        let mut new = before.clone();
        new.hotkey = "Control+Alt+K".into();
        let err = set_settings_impl(&st, new, |_, _| Err("taken".into())).unwrap_err();
        assert_eq!(err.code, "hotkey_unavailable");
        assert_eq!(*st.settings.read().unwrap(), before);
    }

    #[test]
    fn set_settings_applies_valid_changes() {
        let st = AppState::in_memory();
        let mut new = st.settings.read().unwrap().clone();
        new.hotkey = "Control+Alt+K".into();
        new.default_event_minutes = 45;
        let mut registered = None;
        let saved = set_settings_impl(&st, new.clone(), |n, o| {
            registered = Some((n.to_string(), o.to_string()));
            Ok(())
        })
        .unwrap();
        assert_eq!(saved, new);
        assert_eq!(
            registered,
            Some(("Control+Alt+K".into(), "Alt+Super+Space".into()))
        );
        assert_eq!(st.settings.read().unwrap().default_event_minutes, 45);
    }

    #[test]
    fn set_settings_rejects_unknown_default_calendar() {
        let st = AppState::in_memory();
        let mut new = st.settings.read().unwrap().clone();
        new.default_calendar_id = 999;
        assert_eq!(
            set_settings_impl(&st, new, |_, _| Ok(())).unwrap_err().code,
            "invalid"
        );
    }

    #[test]
    fn empty_text_is_rejected() {
        let st = AppState::in_memory();
        let err = create_from_text_impl(&st, "   ", now(), false).unwrap_err();
        assert_eq!(err.code, "empty");
    }
}
