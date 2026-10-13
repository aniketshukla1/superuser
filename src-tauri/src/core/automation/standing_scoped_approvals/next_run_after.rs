use super::super::*;

/// Why this schedule cannot be used, or `None` if it can.
///
/// The floor is checked twice: a deterministic scan from a fixed
/// date, so whether a schedule is acceptable never depends on when it was asked about,
/// and then the next two firings from now, which catches the shapes a fixed scan cannot
/// see. Both must pass.
pub fn schedule_problem(schedule: &Schedule) -> Option<String> {
    if schedule.kind == "once" {
        return match schedule.fire_at.as_deref().and_then(parse_iso_local) {
            Some(_) => None,
            None => Some("That date and time couldn't be read. Use an ISO datetime, like 2026-03-01T09:00.".into()),
        };
    }
    let Some(cron) = schedule.cron.as_deref().filter(|c| !c.trim().is_empty()) else {
        return Some("A repeating schedule needs a cron expression, like `0 9 * * 1-5`.".into());
    };
    if zone_of(&schedule.timezone).is_none() {
        return Some(format!(
            "“{}” isn't a timezone this machine knows. Use a name like Europe/Madrid, UTC, an offset like +02:00, or leave \
             it out for this machine's own clock.",
            schedule.timezone
        ));
    }
    if cron_fields(cron).is_none() {
        return Some(format!("`{cron}` isn't a cron expression this can read. Five fields: minute hour day-of-month month day-of-week."));
    }
    let too_often = |from: f64| -> bool {
        let mut at = from;
        let mut seen = 0;
        while seen < 200 {
            let Some(next) = next_run_after(schedule, at) else { return false };
            if next - at < FLOOR_SECS && seen > 0 {
                return true;
            }
            // The first gap is measured from `from`, which is not a firing; start
            // counting once we are standing on one.
            at = next;
            seen += 1;
        }
        false
    };
    // 2024-01-01, so the answer to "is this schedule allowed" is the same on any day.
    let fixed = 1_704_067_200.0;
    if too_often(fixed) || too_often(now()) {
        return Some(format!(
            "That fires more often than every {} minutes, which is too often for an automation. Space it out.",
            (FLOOR_SECS / 60.0) as u32
        ));
    }
    if next_run_after(schedule, now()).is_none() {
        return Some("That schedule has no next run — check the day and month fields.".into());
    }
    None
}

pub fn next_run_after(schedule: &Schedule, after: f64) -> Option<f64> {
    use chrono::{Datelike, Duration, FixedOffset, Local, TimeZone};
    if schedule.kind == "once" {
        let ts = parse_iso_local(schedule.fire_at.as_deref()?)?;
        return (ts > after).then_some(ts);
    }
    let zone = zone_of(&schedule.timezone)?;
    let calendar = calendar_zone(&zone);
    if matches!(zone, Zone::Named(_)) && calendar.is_none() {
        return None;
    }
    let [minute, hour, dom, month, dow] = cron_fields(schedule.cron.as_deref()?)?;
    // The date walk happens in the schedule's own clock, so "09:00" means nine there.
    let local_date = |ts: f64| -> Option<chrono::NaiveDate> {
        match (&calendar, &zone) {
            (Some(tz), _) => date_in(tz, ts),
            (None, Zone::Fixed(off)) => FixedOffset::east_opt(*off)?.timestamp_opt(ts as i64, 0).single().map(|d| d.date_naive()),
            (None, _) => Local.timestamp_opt(ts as i64, 0).single().map(|d| d.date_naive()),
        }
    };
    let stamp = |date: chrono::NaiveDate, h: u32, m: u32| -> Option<f64> {
        let ndt = date.and_hms_opt(h, m, 0)?;
        match (&calendar, &zone) {
            (Some(tz), _) => instant_in(tz, &ndt),
            (None, Zone::Fixed(off)) => FixedOffset::east_opt(*off)?.from_local_datetime(&ndt).single().map(|d| d.timestamp() as f64),
            // A machine whose zone the database can't read: `chrono`'s reading, taking the first
            // of a repeated time. A skipped time still resolves to nothing on this path.
            (None, _) => Local.from_local_datetime(&ndt).earliest().map(|d| d.timestamp() as f64),
        }
    };
    let start = local_date(after)?;
    for i in 0..366 {
        let date = start + Duration::days(i);
        if !field_matches(month, date.month(), false, (1, 12)) {
            continue;
        }
        let day_matches = if dom == "*" && dow == "*" {
            true
        } else if dom == "*" {
            field_matches(dow, date.weekday().num_days_from_sunday(), true, (0, 7))
        } else if dow == "*" {
            field_matches(dom, date.day(), false, (1, 31))
        } else {
            // Real cron ORs the two when both are set; matching neither would make
            // `0 9 1 * 1` mean "never", which is never what someone meant.
            field_matches(dom, date.day(), false, (1, 31))
                || field_matches(dow, date.weekday().num_days_from_sunday(), true, (0, 7))
        };
        if !day_matches {
            continue;
        }
        for h in 0..24 {
            if !field_matches(hour, h, false, (0, 23)) {
                continue;
            }
            for m in 0..60 {
                if !field_matches(minute, m, false, (0, 59)) {
                    continue;
                }
                if let Some(ts) = stamp(date, h, m) {
                    if ts > after {
                        return Some(ts);
                    }
                }
            }
        }
    }
    None
}

/// A new one-time schedule's `fire_at`, pinned to the zone it was written in.
///
/// A time without an offset is read on this machine's clock whatever the schedule's zone says, so
/// "once at 09:00, Europe/Madrid" used to fire at nine here. A new one-time schedule in a named zone
/// or an offset is stored with that zone's offset at that moment — `2026-03-01T09:00` in Madrid
/// becomes `2026-03-01T09:00:00+01:00` — which reads the same on any clock. One already saved
/// without an offset is left alone: changing how it is read would move a moment somebody already
/// planned around. So this runs where a schedule is made, never where one is read.
///
/// Unchanged when there is nothing to pin: the time carries an offset already, the zone is this
/// machine's own, or either can't be read — the schedule check then says why.
pub fn pin_fire_at(fire_at: &str, timezone: &str) -> String {
    use chrono::{FixedOffset, NaiveDateTime, TimeZone};
    let at = fire_at.trim();
    if chrono::DateTime::parse_from_rfc3339(at).is_ok() {
        return at.to_string();
    }
    let Some(zone) = zone_of(timezone) else { return at.to_string() };
    let Some(naive) = ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(at, f).ok())
    else {
        return at.to_string();
    };
    let pinned = match &zone {
        Zone::Local => None,
        Zone::Fixed(off) => FixedOffset::east_opt(*off).and_then(|o| o.from_local_datetime(&naive).single()),
        Zone::Named(_) => calendar_zone(&zone).and_then(|tz| {
            // The zone's offset at the moment itself — in a skipped hour, the moment after the
            // jump, so the stored text names the clock time it will actually fire at.
            let ts = instant_in(&tz, &naive)? as i64;
            let offset = jiff::Timestamp::from_second(ts).ok()?.to_zoned(tz).offset().seconds();
            FixedOffset::east_opt(offset)?.timestamp_opt(ts, 0).single()
        }),
    };
    pinned.map(|d| d.to_rfc3339()).unwrap_or_else(|| at.to_string())
}

pub(in crate::core::automation) fn parse_iso_local(s: &str) -> Option<f64> {
    parse_iso_in(s, calendar_zone(&Zone::Local).as_ref())
}

/// A one-time schedule's moment. A time with an offset is that moment; one without is read on
/// `tz`'s clock — this machine's — with daylight saving as the cron path has it, so a time the
/// clock skips or repeats still names a moment rather than failing to read (F-5).
pub(in crate::core::automation) fn parse_iso_in(s: &str, tz: Option<&jiff::tz::TimeZone>) -> Option<f64> {
    use chrono::{Local, NaiveDateTime, TimeZone};
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(d.timestamp() as f64);
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(nd) = NaiveDateTime::parse_from_str(s, fmt) {
            let at = match tz {
                Some(tz) => instant_in(tz, &nd),
                None => Local.from_local_datetime(&nd).earliest().map(|d| d.timestamp() as f64),
            };
            if at.is_some() {
                return at;
            }
        }
    }
    None
}
