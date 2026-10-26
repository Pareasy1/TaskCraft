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
            StoreError::Duplicate => "duplicate",
            StoreError::LastPlan => "last_plan",
            StoreError::LastBucket => "last_bucket",
            StoreError::Rule(r) => r.code(),
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
    let store = st.store.lock().map_err(poisoned)?;
    let calendars = store.calendars()?.into_iter().map(|c| c.name).collect();
    let plans = store.list_plans()?.into_iter().map(|p| p.name).collect();
    let templates = store
        .list_templates()?
        .into_iter()
        .map(|t| t.name)
        .collect();
    Ok(ParseOptions {
        default_event_minutes: settings.default_event_minutes,
        calendars,
        date_order: settings.date_order,
        plans,
        templates,
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
    let plans = store.list_plans()?;
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
        &plans,
    );
    let r = match item {
        NewItem::Event(e) => ItemRef {
            kind: ItemKindDb::Event,
            id: store.insert_event(&e)?,
        },
        NewItem::Task(t) => {
            let template = match &p.template {
                Some(name) => store.template_by_name(name)?,
                None => None,
            };
            let id = match template {
                Some(tpl) => store.create_from_template(tpl.id, &t)?,
                None => store.insert_task(&t)?,
            };
            ItemRef {
                kind: ItemKindDb::Task,
                id,
            }
        }
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
pub fn list_subtasks(st: State<AppState>, parent_id: i64) -> CmdResult<Vec<Task>> {
    Ok(st
        .store
        .lock()
        .map_err(poisoned)?
        .list_subtasks(parent_id)?)
}

#[tauri::command]
pub fn reorder_subtasks(
    app: AppHandle,
    st: State<AppState>,
    parent_id: i64,
    ids: Vec<i64>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .reorder_subtasks(parent_id, &ids)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_task_parent(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    parent_id: Option<i64>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .set_task_parent(task_id, parent_id)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn complete_with_subtasks(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .complete_with_subtasks(id, Utc::now())?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_checklist(st: State<AppState>, task_id: i64) -> CmdResult<Vec<ChecklistItem>> {
    Ok(st.store.lock().map_err(poisoned)?.list_checklist(task_id)?)
}

#[tauri::command]
pub fn add_checklist_item(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    text: String,
) -> CmdResult<i64> {
    let id = st
        .store
        .lock()
        .map_err(poisoned)?
        .add_checklist_item(task_id, &text)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_checklist_item(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    text: Option<String>,
    done: Option<bool>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .update_checklist_item(id, text.as_deref(), done)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_checklist(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    ids: Vec<i64>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .reorder_checklist(task_id, &ids)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_checklist_item(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .delete_checklist_item(id)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn checklist_item_to_subtask(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<i64> {
    let sub = st
        .store
        .lock()
        .map_err(poisoned)?
        .checklist_item_to_subtask(id)?;
    changed(&app);
    Ok(sub)
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

/// Renames the untouched seed plan/bucket to the language's names (once).
/// Returns whether anything was renamed.
pub fn set_ui_language_impl(st: &AppState, lang: crate::i18n::Lang) -> CmdResult<bool> {
    *st.lang.write().map_err(poisoned)? = lang;
    let text = crate::i18n::text(lang);
    Ok(st
        .store
        .lock()
        .map_err(poisoned)?
        .localize_seed(text.seed_plan, text.seed_bucket)?)
}

/// The UI reports the language it picked from the Mac's settings; the
/// menu-bar menu and notifications follow it.
#[tauri::command]
pub fn set_ui_language(app: AppHandle, st: State<AppState>, lang: String) -> CmdResult<()> {
    let lang = crate::i18n::Lang::from_code(&lang);
    match set_ui_language_impl(&st, lang) {
        Ok(true) => changed(&app),
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e.message, "cannot localize seed plan"),
    }
    if let Err(e) = crate::tray::set_menu_language(&app, lang) {
        tracing::warn!(error = %e, "cannot update tray menu language");
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

fn check_color(c: &str) -> CmdResult<()> {
    if is_hex_color(c) {
        Ok(())
    } else {
        Err(CmdError::new("invalid", "Pick a plan color"))
    }
}

#[tauri::command]
pub fn list_plans(st: State<AppState>) -> CmdResult<Vec<Plan>> {
    Ok(st.store.lock().map_err(poisoned)?.list_plans()?)
}

#[tauri::command]
pub fn create_plan(
    app: AppHandle,
    st: State<AppState>,
    name: String,
    color: String,
) -> CmdResult<i64> {
    check_color(&color)?;
    let id = st
        .store
        .lock()
        .map_err(poisoned)?
        .create_plan(&name, &color)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_plan(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    name: Option<String>,
    color: Option<String>,
) -> CmdResult<()> {
    if let Some(c) = &color {
        check_color(c)?;
    }
    st.store
        .lock()
        .map_err(poisoned)?
        .update_plan(id, name.as_deref(), color.as_deref())?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_plans(app: AppHandle, st: State<AppState>, ids: Vec<i64>) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.reorder_plans(&ids)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_plan(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    mode: DeleteMode,
) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_plan(id, mode)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_buckets(st: State<AppState>, plan_id: i64) -> CmdResult<Vec<Bucket>> {
    Ok(st.store.lock().map_err(poisoned)?.list_buckets(plan_id)?)
}

#[tauri::command]
pub fn create_bucket(
    app: AppHandle,
    st: State<AppState>,
    plan_id: i64,
    name: String,
) -> CmdResult<i64> {
    let id = st
        .store
        .lock()
        .map_err(poisoned)?
        .create_bucket(plan_id, &name)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn rename_bucket(app: AppHandle, st: State<AppState>, id: i64, name: String) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .rename_bucket(id, &name)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_buckets(
    app: AppHandle,
    st: State<AppState>,
    plan_id: i64,
    ids: Vec<i64>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .reorder_buckets(plan_id, &ids)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_bucket(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    mode: DeleteMode,
) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_bucket(id, mode)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn move_task(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    bucket_id: i64,
    before: Option<i64>,
    after: Option<i64>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .move_task(task_id, bucket_id, before, after)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_default_plan(st: State<AppState>) -> CmdResult<i64> {
    Ok(st.store.lock().map_err(poisoned)?.default_plan_id()?)
}

#[tauri::command]
pub fn set_default_plan(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.set_default_plan(id)?;
    changed(&app);
    Ok(())
}

// ---------- attachments ----------

/// What `open_attachment` hands to the system opener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenTarget {
    Path(std::path::PathBuf),
    Url(String),
}

/// Drops the `\\?\` verbatim prefix `canonicalize` adds on Windows, which
/// ShellExecute and many apps handle badly. Unchanged elsewhere.
pub fn native_path(p: &std::path::Path) -> std::path::PathBuf {
    dunce::simplified(p).to_path_buf()
}

fn is_http(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://")
}

/// Resolves an attachment to a stored file inside `<data>/attachments`
/// (missing → `not_found`, escaping → `invalid`) or an http(s) link.
pub fn open_target(st: &AppState, id: i64) -> CmdResult<OpenTarget> {
    let store = st.store.lock().map_err(poisoned)?;
    let a = store.get_attachment(id)?;
    match a.kind {
        AttachmentKind::File => Ok(OpenTarget::Path(native_path(
            &store.attachment_path(id, &st.data_dir)?,
        ))),
        AttachmentKind::Link => match a.url {
            Some(u) if is_http(&u) => Ok(OpenTarget::Url(u)),
            _ => Err(CmdError::new(
                "attachment_url",
                "Only web links can be opened",
            )),
        },
    }
}

/// Resolves the target first, then calls `open`; nothing is opened when the
/// target cannot be resolved.
pub fn open_attachment_impl(
    st: &AppState,
    id: i64,
    open: impl FnOnce(OpenTarget) -> std::result::Result<(), String>,
) -> CmdResult<()> {
    let target = open_target(st, id)?;
    open(target).map_err(|e| {
        tracing::warn!(error = %e, id, "cannot open attachment");
        CmdError::new("open", e)
    })
}

/// Stored file to show in Finder/Explorer; links have none (`invalid`).
pub fn reveal_target(st: &AppState, id: i64) -> CmdResult<std::path::PathBuf> {
    match open_target(st, id)? {
        OpenTarget::Path(p) => Ok(p),
        OpenTarget::Url(_) => Err(CmdError::new("invalid", "A link has no file to show")),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileError {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddFilesResult {
    pub added: Vec<Attachment>,
    pub errors: Vec<FileError>,
}

/// Adds each file; a failing file is reported by name and does not stop
/// the others. The store is locked per file, so other commands wait for at
/// most one copy.
pub fn add_files_impl(st: &AppState, task_id: i64, paths: &[std::path::PathBuf]) -> AddFilesResult {
    let mut out = AddFilesResult::default();
    for p in paths {
        let r = st
            .store
            .lock()
            .map_err(poisoned)
            .and_then(|s| Ok(s.add_file_attachment(task_id, p, &st.data_dir)?));
        match r {
            Ok(a) => out.added.push(a),
            Err(e) => out.errors.push(FileError {
                name: display_name(p),
                code: e.code,
            }),
        }
    }
    out
}

fn display_name(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_attachments(st: State<AppState>, task_id: i64) -> CmdResult<Vec<Attachment>> {
    Ok(st
        .store
        .lock()
        .map_err(poisoned)?
        .list_attachments(task_id)?)
}

/// Opens the multi-select file dialog, then adds the picked files. Async so
/// the dialog never blocks the main thread, and the store is only locked
/// after the dialog has closed.
#[tauri::command]
pub async fn pick_and_add_files(app: AppHandle, task_id: i64) -> CmdResult<AddFilesResult> {
    use tauri_plugin_dialog::DialogExt;
    let builder = app.dialog().file();
    let picked = tauri::async_runtime::spawn_blocking(move || builder.blocking_pick_files())
        .await
        .map_err(|e| CmdError::new("internal", e.to_string()))?;
    let Some(picked) = picked else {
        return Ok(AddFilesResult::default());
    };
    let mut paths = Vec::new();
    let mut bad = Vec::new();
    for f in picked {
        let label = f.to_string();
        match f.into_path() {
            Ok(p) => paths.push(p),
            Err(_) => bad.push(FileError {
                name: label,
                code: "not_found".into(),
            }),
        }
    }
    // Copying can take a while: keep it off the async workers.
    let handle = app.clone();
    let mut r = tauri::async_runtime::spawn_blocking(move || {
        add_files_impl(&handle.state::<AppState>(), task_id, &paths)
    })
    .await
    .map_err(|e| CmdError::new("internal", e.to_string()))?;
    r.errors.extend(bad);
    if !r.added.is_empty() {
        changed(&app);
    }
    Ok(r)
}

#[tauri::command]
pub fn add_link_attachment(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    url: String,
    name: Option<String>,
) -> CmdResult<Attachment> {
    let a =
        st.store
            .lock()
            .map_err(poisoned)?
            .add_link_attachment(task_id, &url, name.as_deref())?;
    changed(&app);
    Ok(a)
}

#[tauri::command]
pub fn rename_attachment(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    name: String,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .rename_attachment(id, &name)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_attachments(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    ids: Vec<i64>,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .reorder_attachments(task_id, &ids)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn remove_attachment(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<Attachment> {
    let a = st
        .store
        .lock()
        .map_err(poisoned)?
        .remove_attachment(id, Utc::now())?;
    changed(&app);
    Ok(a)
}

#[tauri::command]
pub fn restore_file_attachment(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    name: String,
    stored: String,
    size: i64,
) -> CmdResult<Attachment> {
    let a = st
        .store
        .lock()
        .map_err(poisoned)?
        .restore_file_attachment(task_id, &name, &stored, size)?;
    changed(&app);
    Ok(a)
}

#[tauri::command]
pub fn open_attachment(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    open_attachment_impl(&st, id, |t| {
        let r = match t {
            // The free function takes a Path, so non-UTF-8 names survive.
            OpenTarget::Path(p) => tauri_plugin_opener::open_path(p, None::<&str>),
            OpenTarget::Url(u) => app.opener().open_url(u, None::<&str>),
        };
        r.map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub fn reveal_attachment(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let p = reveal_target(&st, id)?;
    app.opener().reveal_item_in_dir(p).map_err(|e| {
        tracing::warn!(error = %e, id, "cannot reveal attachment");
        CmdError::new("open", e.to_string())
    })
}

// ---------- duplicate & templates ----------

/// Duplicates with the title suffix of the current UI language.
pub fn duplicate_task_impl(st: &AppState, id: i64) -> CmdResult<DuplicateResult> {
    let lang = *st.lang.read().map_err(poisoned)?;
    let suffix = crate::i18n::text(lang).duplicate_suffix;
    let r = st
        .store
        .lock()
        .map_err(poisoned)?
        .duplicate_task(id, suffix, &st.data_dir)?;
    if !r.skipped.is_empty() {
        tracing::warn!(
            id,
            skipped = r.skipped.len(),
            "duplicate skipped missing files"
        );
    }
    Ok(r)
}

/// Async so the copy runs off the main thread: duplicating large
/// attachments must not freeze the window.
#[tauri::command]
pub async fn duplicate_task(app: AppHandle, id: i64) -> CmdResult<DuplicateResult> {
    let handle = app.clone();
    let r = tauri::async_runtime::spawn_blocking(move || {
        duplicate_task_impl(&handle.state::<AppState>(), id)
    })
    .await
    .map_err(|e| CmdError::new("internal", e.to_string()))??;
    changed(&app);
    Ok(r)
}

#[tauri::command]
pub fn save_template(
    app: AppHandle,
    st: State<AppState>,
    task_id: i64,
    name: String,
    replace: bool,
) -> CmdResult<i64> {
    let id = st
        .store
        .lock()
        .map_err(poisoned)?
        .save_template(task_id, &name, replace)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn list_templates(st: State<AppState>) -> CmdResult<Vec<TemplateInfo>> {
    Ok(st.store.lock().map_err(poisoned)?.list_templates()?)
}

#[tauri::command]
pub fn rename_template(
    app: AppHandle,
    st: State<AppState>,
    id: i64,
    name: String,
) -> CmdResult<()> {
    st.store
        .lock()
        .map_err(poisoned)?
        .rename_template(id, &name)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_template(app: AppHandle, st: State<AppState>, id: i64) -> CmdResult<()> {
    st.store.lock().map_err(poisoned)?.delete_template(id)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn create_from_template(
    app: AppHandle,
    st: State<AppState>,
    template_id: i64,
    input: TaskInput,
) -> CmdResult<i64> {
    let id = st
        .store
        .lock()
        .map_err(poisoned)?
        .create_from_template(template_id, &input)?;
    changed(&app);
    Ok(id)
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
    fn quick_entry_routes_slash_plan() {
        let st = AppState::in_memory();
        let pid = st
            .store
            .lock()
            .unwrap()
            .create_plan("3D print", "#112233")
            .unwrap();
        let r = create_from_text_impl(&st, "ship Rafael /3dprint", now(), true).unwrap();
        let t = st.store.lock().unwrap().get_task(r.id).unwrap();
        assert_eq!(t.plan_id, pid);
        assert_eq!(t.title, "ship Rafael");
    }

    #[test]
    fn quick_entry_slash_matching_calendar_and_plan_sets_both() {
        let st = AppState::in_memory();
        let pid = st
            .store
            .lock()
            .unwrap()
            .create_plan("Work", "#112233")
            .unwrap();
        let r = create_from_text_impl(&st, "pay invoice /work", now(), true).unwrap();
        let s = st.store.lock().unwrap();
        let t = s.get_task(r.id).unwrap();
        let work = s
            .calendars()
            .unwrap()
            .into_iter()
            .find(|c| c.name == "Work")
            .unwrap();
        assert_eq!(t.calendar_id, work.id);
        assert_eq!(t.plan_id, pid);
        assert_eq!(t.title, "pay invoice");
    }

    #[test]
    fn quick_entry_without_plan_uses_default() {
        let st = AppState::in_memory();
        let b = st
            .store
            .lock()
            .unwrap()
            .create_plan("B", "#112233")
            .unwrap();
        st.store.lock().unwrap().set_default_plan(b).unwrap();
        let r = create_from_text_impl(&st, "buy milk", now(), true).unwrap();
        assert_eq!(st.store.lock().unwrap().get_task(r.id).unwrap().plan_id, b);
    }

    #[test]
    fn quick_entry_after_default_deleted() {
        let st = AppState::in_memory();
        let (a, b) = {
            let s = st.store.lock().unwrap();
            let a = s.default_plan_id().unwrap();
            let b = s.create_plan("B", "#112233").unwrap();
            s.set_default_plan(b).unwrap();
            s.delete_plan(b, DeleteMode::MoveTo(a)).unwrap();
            (a, b)
        };
        assert_ne!(a, b);
        let r = create_from_text_impl(&st, "buy milk", now(), true).unwrap();
        assert_eq!(st.store.lock().unwrap().get_task(r.id).unwrap().plan_id, a);
    }

    #[test]
    fn store_errors_have_codes() {
        assert_eq!(CmdError::from(StoreError::Duplicate).code, "duplicate");
        assert_eq!(CmdError::from(StoreError::LastPlan).code, "last_plan");
        assert_eq!(CmdError::from(StoreError::LastBucket).code, "last_bucket");
        assert_eq!(
            CmdError::from(StoreError::Rule(taskcraft_store::Rule::StartAfterDue)).code,
            "start_after_due"
        );
    }

    #[test]
    fn quick_entry_sets_priority() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "pay rent !urgente", now(), true).unwrap();
        let t = st.store.lock().unwrap().get_task(r.id).unwrap();
        assert_eq!(t.priority, Priority::Urgent);
        assert_eq!(t.title, "pay rent");
    }

    #[test]
    fn rule_errors_have_codes() {
        use taskcraft_store::Rule;
        for (r, c) in [
            (Rule::ParentIsSubtask, "parent_is_subtask"),
            (Rule::HasSubtasks, "has_subtasks"),
            (Rule::RepeatingSubtask, "repeating_subtask"),
            (Rule::StartAfterDue, "start_after_due"),
            (Rule::ChecklistText, "checklist_text"),
            (Rule::AttachmentTooLarge, "attachment_too_large"),
            (Rule::AttachmentUrl, "attachment_url"),
            (Rule::TemplateName, "template_name"),
        ] {
            assert_eq!(CmdError::from(StoreError::Rule(r)).code, c);
        }
    }

    #[test]
    fn ui_language_localizes_seed_once() {
        use crate::i18n::Lang;
        let st = AppState::in_memory();
        assert!(set_ui_language_impl(&st, Lang::It).unwrap());
        {
            let s = st.store.lock().unwrap();
            let plan = s.list_plans().unwrap().remove(0);
            assert_eq!(plan.name, "Le mie attività");
            assert_eq!(s.list_buckets(plan.id).unwrap()[0].name, "Da fare");
        }
        assert!(!set_ui_language_impl(&st, Lang::En).unwrap());
        let plan = st.store.lock().unwrap().list_plans().unwrap().remove(0);
        assert_eq!(plan.name, "Le mie attività");
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
            Some((
                "Control+Alt+K".into(),
                crate::settings::default_hotkey_for(std::env::consts::OS).into()
            ))
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

    fn task(st: &AppState, title: &str, parent: Option<i64>) -> i64 {
        st.store
            .lock()
            .unwrap()
            .insert_task(&TaskInput {
                calendar_id: 1,
                title: title.into(),
                tz: "Europe/Rome".into(),
                parent_id: parent,
                ..Default::default()
            })
            .unwrap()
    }

    #[test]
    fn quick_entry_uses_template() {
        let st = AppState::in_memory();
        let src = task(&st, "Ship", None);
        task(&st, "pack", Some(src));
        task(&st, "label", Some(src));
        st.store
            .lock()
            .unwrap()
            .save_template(src, "name", false)
            .unwrap();
        let r = create_from_text_impl(&st, "ship Rafael friday +name", now(), true).unwrap();
        let s = st.store.lock().unwrap();
        let t = s.get_task(r.id).unwrap();
        assert_eq!(t.title, "ship Rafael");
        assert!(t.due.is_some(), "friday still sets the due date");
        let subs: Vec<String> = s
            .list_subtasks(r.id)
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(subs, ["pack", "label"]);
    }

    #[test]
    fn quick_entry_unknown_template_stays_in_title() {
        let st = AppState::in_memory();
        let r = create_from_text_impl(&st, "ship +nope", now(), true).unwrap();
        let s = st.store.lock().unwrap();
        assert_eq!(s.get_task(r.id).unwrap().title, "ship +nope");
        assert!(s.list_subtasks(r.id).unwrap().is_empty());
    }

    #[test]
    fn duplicate_suffix_follows_language() {
        use crate::i18n::Lang;
        let st = AppState::in_memory();
        let id = task(&st, "Report", None);
        *st.lang.write().unwrap() = Lang::It;
        let r = duplicate_task_impl(&st, id).unwrap();
        assert!(r.skipped.is_empty());
        let s = st.store.lock().unwrap();
        assert_eq!(s.get_task(r.id).unwrap().title, "Report (copia)");
        drop(s);
        *st.lang.write().unwrap() = Lang::En;
        let r = duplicate_task_impl(&st, id).unwrap();
        let s = st.store.lock().unwrap();
        assert_eq!(s.get_task(r.id).unwrap().title, "Report (copy)");
    }

    #[test]
    fn open_missing_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let st = AppState::in_memory_at(tmp.path());
        let id = task(&st, "T", None);
        let src = tmp.path().join("notes.txt");
        std::fs::write(&src, b"hi").unwrap();
        let a = st
            .store
            .lock()
            .unwrap()
            .add_file_attachment(id, &src, tmp.path())
            .unwrap();
        std::fs::remove_file(tmp.path().join(a.stored.as_deref().unwrap())).unwrap();
        let mut called = false;
        let err = open_attachment_impl(&st, a.id, |_| {
            called = true;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(err.code, "not_found");
        assert!(!called, "the opener must not run for a missing file");
    }

    #[test]
    fn open_resolves_files_and_links() {
        let tmp = tempfile::tempdir().unwrap();
        let st = AppState::in_memory_at(tmp.path());
        let id = task(&st, "T", None);
        let src = tmp.path().join("a.txt");
        std::fs::write(&src, b"hi").unwrap();
        let (f, l) = {
            let s = st.store.lock().unwrap();
            (
                s.add_file_attachment(id, &src, tmp.path()).unwrap(),
                s.add_link_attachment(id, "https://example.com/x", None)
                    .unwrap(),
            )
        };
        let mut seen = None;
        open_attachment_impl(&st, f.id, |t| {
            seen = Some(t);
            Ok(())
        })
        .unwrap();
        match seen.take() {
            Some(OpenTarget::Path(p)) => assert!(p.ends_with(f.stored.as_deref().unwrap())),
            other => panic!("expected a path, got {other:?}"),
        }
        open_attachment_impl(&st, l.id, |t| {
            seen = Some(t);
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, Some(OpenTarget::Url("https://example.com/x".into())));
        let err = open_attachment_impl(&st, f.id, |_| Err("boom".into())).unwrap_err();
        assert_eq!(err.code, "open");
        assert_eq!(reveal_target(&st, l.id).unwrap_err().code, "invalid");
        assert!(reveal_target(&st, f.id).is_ok());
    }

    #[test]
    fn add_files_reports_per_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let st = AppState::in_memory_at(tmp.path());
        let id = task(&st, "T", None);
        let ok = tmp.path().join("ok.txt");
        std::fs::write(&ok, b"hi").unwrap();
        let r = add_files_impl(&st, id, &[ok, tmp.path().join("gone.pdf")]);
        assert_eq!(r.added.len(), 1);
        assert_eq!(r.added[0].name, "ok.txt");
        assert_eq!(
            r.errors,
            [FileError {
                name: "gone.pdf".into(),
                code: "not_found".into()
            }]
        );
    }

    #[test]
    fn native_path_strips_verbatim_prefix_only_on_windows() {
        let p = std::path::Path::new("/Users/me/Library/TaskCraft/attachments/1/1-a.txt");
        assert_eq!(native_path(p), p);
        #[cfg(windows)]
        assert_eq!(
            native_path(std::path::Path::new(r"\\?\C:\data\a.txt")),
            std::path::Path::new(r"C:\data\a.txt")
        );
    }

    #[test]
    fn rule_codes_for_attachments() {
        use taskcraft_store::Rule;
        assert_eq!(
            CmdError::from(StoreError::Rule(Rule::AttachmentTooLarge)).code,
            "attachment_too_large"
        );
        assert_eq!(
            CmdError::from(StoreError::Rule(Rule::AttachmentUrl)).code,
            "attachment_url"
        );
        assert_eq!(
            CmdError::from(StoreError::Rule(Rule::TemplateName)).code,
            "template_name"
        );
    }

    #[test]
    fn empty_text_is_rejected() {
        let st = AppState::in_memory();
        let err = create_from_text_impl(&st, "   ", now(), false).unwrap_err();
        assert_eq!(err.code, "empty");
    }
}
