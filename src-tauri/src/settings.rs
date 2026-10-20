//! User settings persisted as JSON next to the database.

use chrono::Weekday;
use serde::{Deserialize, Serialize};
use std::path::Path;
use taskcraft_parse::DateOrder;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub hotkey: String,
    pub launch_at_login: bool,
    pub default_calendar_id: i64,
    pub default_event_minutes: u32,
    pub week_start: Weekday,
    pub date_order: DateOrder,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkey: "Alt+Super+Space".into(),
            launch_at_login: false,
            default_calendar_id: 1,
            default_event_minutes: 60,
            week_start: Weekday::Mon,
            date_order: detect_date_order(),
        }
    }
}

/// Month-first only for US-style regions; everyone else gets day-first.
pub fn date_order_for_locale(locale: &str) -> DateOrder {
    let region = locale.split(['@', '.']).next().unwrap_or("");
    match region.rsplit(['_', '-']).next() {
        Some("US" | "PH" | "FM" | "MH" | "PW") => DateOrder::MonthDay,
        _ => DateOrder::DayMonth,
    }
}

fn detect_date_order() -> DateOrder {
    let out = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            date_order_for_locale(String::from_utf8_lossy(&o.stdout).trim())
        }
        _ => DateOrder::DayMonth,
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?,
        )?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_date_order() {
        assert_eq!(date_order_for_locale("it_IT"), DateOrder::DayMonth);
        assert_eq!(date_order_for_locale("en_US"), DateOrder::MonthDay);
        assert_eq!(
            date_order_for_locale("en_GB@calendar=gregorian"),
            DateOrder::DayMonth
        );
    }
}
