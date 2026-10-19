//! Recurrence expansion. The `rrule` column holds an RRULE body, optionally
//! followed by a line `EXDATE:<utc>,<utc>…` listing removed occurrences.

use crate::model::{Result, StoreError};
use crate::time::local_to_utc;
use chrono::{DateTime, Duration, NaiveTime, Utc};
use chrono_tz::Tz;

pub(crate) struct RuleText {
    pub rule: String,
    pub exdates: Vec<DateTime<Utc>>,
}

pub(crate) fn split(col: &str) -> RuleText {
    let mut lines = col.lines();
    let rule = lines.next().unwrap_or_default().trim().to_string();
    let exdates = lines
        .filter_map(|l| l.strip_prefix("EXDATE:"))
        .flat_map(|l| l.split(','))
        .filter_map(|s| parse_basic_utc(s.trim()))
        .collect();
    RuleText { rule, exdates }
}

pub(crate) fn join(r: &RuleText) -> String {
    if r.exdates.is_empty() {
        return r.rule.clone();
    }
    let ex: Vec<String> = r
        .exdates
        .iter()
        .map(|d| d.format("%Y%m%dT%H%M%SZ").to_string())
        .collect();
    format!("{}\nEXDATE:{}", r.rule, ex.join(","))
}

fn parse_basic_utc(s: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|n| n.and_utc())
}

/// `UNTIL=YYYYMMDD` (a local date from the parser) → end of that local day in UTC,
/// which RFC 5545 requires when DTSTART has a TZID.
fn normalize_until(rule: &str, tz: Tz) -> String {
    rule.split(';')
        .map(|part| match part.strip_prefix("UNTIL=") {
            Some(v) if v.len() == 8 => match chrono::NaiveDate::parse_from_str(v, "%Y%m%d") {
                Ok(d) => {
                    let end = local_to_utc(
                        d.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap_or_default()),
                        tz,
                    );
                    format!("UNTIL={}", end.format("%Y%m%dT%H%M%SZ"))
                }
                Err(_) => part.to_string(),
            },
            _ => part.to_string(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Start instants of occurrences whose interval `[start, start+dur)` overlaps
/// `[from, to)`. Wall-clock time is kept across DST via DTSTART;TZID.
pub(crate) fn expand(
    col: &str,
    dtstart: DateTime<Utc>,
    tz: Tz,
    dur: Duration,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>> {
    let r = split(col);
    let local = dtstart.with_timezone(&tz).naive_local();
    let mut text = format!(
        "DTSTART;TZID={}:{}\nRRULE:{}",
        tz.name(),
        local.format("%Y%m%dT%H%M%S"),
        normalize_until(&r.rule, tz)
    );
    if !r.exdates.is_empty() {
        let ex: Vec<String> = r
            .exdates
            .iter()
            .map(|d| d.format("%Y%m%dT%H%M%SZ").to_string())
            .collect();
        text.push_str(&format!("\nEXDATE:{}", ex.join(",")));
    }
    let set: rrule::RRuleSet = text
        .parse()
        .map_err(|e: rrule::RRuleError| StoreError::Invalid(e.to_string()))?;
    let after = (from - dur).with_timezone(&rrule::Tz::UTC);
    let before = to.with_timezone(&rrule::Tz::UTC);
    let res = set.after(after).before(before).all(5000);
    Ok(res
        .dates
        .into_iter()
        .map(|d| d.with_timezone(&Utc))
        .filter(|s| *s < to && *s + dur > from)
        .collect())
}

/// First occurrence strictly after `after` (searching up to ~2 years ahead).
pub(crate) fn next_after(
    col: &str,
    dtstart: DateTime<Utc>,
    tz: Tz,
    after: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>> {
    let from = after + Duration::seconds(1);
    let occ = expand(
        col,
        dtstart,
        tz,
        Duration::seconds(1),
        from,
        from + Duration::days(800),
    )?;
    Ok(occ.into_iter().find(|s| *s > after))
}
