//! Background thread that turns due alerts into macOS notifications.

use crate::commands::{local_tz, ItemRef};
use crate::state::AppState;
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Mutex;
use taskcraft_store::{DueAlert, ItemKindDb, Store, StoreError};
use tauri::{AppHandle, Listener, Manager};

const TICK: std::time::Duration = std::time::Duration::from_secs(30);

pub trait Notifier {
    fn notify(&self, title: &str, body: &str);
}

/// Last notified item, revealed when the user clicks a notification.
#[derive(Default)]
pub struct LastFired(pub Mutex<Option<(ItemRef, DateTime<Utc>)>>);

fn body(store: &Store, a: &DueAlert, tz: Tz) -> String {
    let t = |d: DateTime<Utc>| d.with_timezone(&tz).format("%H:%M").to_string();
    let mut s = match a.owner {
        ItemKindDb::Event => match store.get_event(a.owner_id) {
            Ok(e) if e.input.all_day => "All day".to_string(),
            Ok(e) => format!(
                "{} – {}",
                t(a.when),
                t(a.when + (e.input.end - e.input.start))
            ),
            Err(_) => t(a.when),
        },
        ItemKindDb::Task => format!("Due {}", t(a.when)),
    };
    if let Some(l) = &a.location {
        s.push_str(" · ");
        s.push_str(l);
    }
    s
}

/// Notifies every due alert once and marks it fired.
pub fn tick(
    store: &Store,
    n: &dyn Notifier,
    now: DateTime<Utc>,
    tz: Tz,
) -> Result<Vec<DueAlert>, StoreError> {
    let due = store.due_alerts(now)?;
    for a in &due {
        n.notify(&a.title, &body(store, a, tz));
        store.mark_fired(a.alert_id, now)?;
    }
    Ok(due)
}

struct AppNotifier<'a>(&'a AppHandle);

impl Notifier for AppNotifier<'_> {
    fn notify(&self, title: &str, body: &str) {
        use tauri_plugin_notification::NotificationExt;
        if let Err(e) = self
            .0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            tracing::warn!(error = %e, "notification failed");
        }
    }
}

/// Starts the scheduler thread. It wakes every 30 s, when data changes and
/// when the Mac wakes from sleep.
pub fn spawn(app: &AppHandle) {
    let (tx, rx) = channel::<()>();
    {
        let tx = tx.clone();
        app.listen("items-changed", move |_| {
            let _ = tx.send(());
        });
    }
    observe_wake(tx);
    let app = app.clone();
    std::thread::Builder::new()
        .name("alerts".into())
        .spawn(move || loop {
            run_once(&app);
            match rx.recv_timeout(TICK) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        })
        .map_err(|e| tracing::error!(error = %e, "cannot start alert thread"))
        .ok();
}

fn run_once(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(store) = state.store.lock() else {
        return;
    };
    match tick(&store, &AppNotifier(app), Utc::now(), local_tz()) {
        Ok(fired) => {
            if let Some(last) = fired.last() {
                tracing::info!(count = fired.len(), "alerts fired");
                let r = ItemRef {
                    kind: last.owner,
                    id: last.owner_id,
                };
                if let Ok(mut slot) = app.state::<LastFired>().0.lock() {
                    *slot = Some((r, Utc::now()));
                }
            }
        }
        Err(e) => tracing::warn!(error = %e, "alert tick failed"),
    }
}

/// Item to reveal on a notification click: the last one notified in the
/// past 10 minutes.
pub fn recent(app: &AppHandle) -> Option<ItemRef> {
    let slot = app.state::<LastFired>();
    let mut g = slot.0.lock().ok()?;
    match *g {
        Some((r, at)) if Utc::now() - at < Duration::minutes(10) => {
            *g = None;
            Some(r)
        }
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn observe_wake(tx: Sender<()>) {
    use block2::RcBlock;
    use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification};
    use objc2_foundation::NSNotification;
    use std::ptr::NonNull;

    let tx = Mutex::new(tx);
    let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
        tracing::info!("woke from sleep");
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(());
        }
    });
    unsafe {
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let token = center.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceDidWakeNotification),
            None,
            None,
            &block,
        );
        // Observe for the app's whole lifetime.
        std::mem::forget(token);
    }
}

#[cfg(not(target_os = "macos"))]
fn observe_wake(_tx: Sender<()>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use std::cell::RefCell;
    use taskcraft_store::{AlertSpecUtc, EventInput, Store};

    #[derive(Default)]
    struct Fake(RefCell<Vec<(String, String)>>);
    impl Notifier for Fake {
        fn notify(&self, title: &str, body: &str) {
            self.0.borrow_mut().push((title.into(), body.into()));
        }
    }

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn add(s: &Store, title: &str, start: &str, loc: Option<&str>) {
        let start = utc(start);
        s.insert_event(&EventInput {
            calendar_id: 1,
            title: title.into(),
            start,
            end: start + Duration::hours(1),
            tz: "Europe/Rome".into(),
            all_day: false,
            location: loc.map(str::to_string),
            attendees: vec![],
            notes: None,
            url: None,
            rrule: None,
            alerts: vec![AlertSpecUtc::Before(10)],
        })
        .unwrap();
    }

    #[test]
    fn tick_fires_and_marks() {
        let s = Store::open_in_memory().unwrap();
        add(&s, "Call", "2026-10-20T15:00:00Z", None);
        add(&s, "Dentist", "2026-10-20T15:05:00Z", Some("Via Roma 12"));
        let fake = Fake::default();
        let now = utc("2026-10-20T14:58:00Z");
        let fired = tick(&s, &fake, now, chrono_tz::Europe::Rome).unwrap();
        assert_eq!(fired.len(), 2);
        let calls = fake.0.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "Call");
        assert_eq!(calls[0].1, "17:00 – 18:00");
        assert_eq!(calls[1].1, "17:05 – 18:05 · Via Roma 12");
        assert!(s.due_alerts(now).unwrap().is_empty());
    }
}
