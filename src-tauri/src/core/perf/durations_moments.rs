//! durations and moments

use super::*;

/// A duration as Prometheus writes one — `30s`, `15m`, `1h30m`, `2d`, `1w` — or in words
/// (`2 hours`), in seconds.
pub fn parse_duration(s: &str) -> Result<i64, String> {
    let bad = || format!("“{}” isn't a duration — write it like 30m, 2h or 7d", s.trim());
    let mut t = s.trim().to_ascii_lowercase();
    for (words, unit) in [
        (&["weeks", "week", "wks", "wk"][..], "w"),
        (&["days", "day"], "d"),
        (&["hours", "hour", "hrs", "hr"], "h"),
        (&["minutes", "minute", "mins", "min"], "m"),
        (&["seconds", "second", "secs", "sec"], "s"),
    ] {
        for w in words {
            t = t.replace(w, unit);
        }
    }
    let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let (mut total, mut num, mut any) = (0i64, String::new(), false);
    for c in t.chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        let unit = match c {
            's' => 1,
            'm' => 60,
            'h' => 3_600,
            'd' => 86_400,
            'w' => 604_800,
            'y' => 31_536_000,
            _ => return Err(bad()),
        };
        let n: i64 = num.parse().map_err(|_| bad())?;
        total = total.saturating_add(n.saturating_mul(unit));
        num.clear();
        any = true;
    }
    if !num.is_empty() || !any {
        return Err(bad());
    }
    if total <= 0 {
        return Err("a duration has to be longer than zero".into());
    }
    Ok(total)
}

/// Seconds the way Prometheus writes them: `90m`, `2h`, `7d`.
pub fn prom_duration(secs: i64) -> String {
    for (unit, n) in [("w", 604_800), ("d", 86_400), ("h", 3_600), ("m", 60)] {
        if secs >= n && secs % n == 0 {
            return format!("{}{unit}", secs / n);
        }
    }
    format!("{secs}s")
}

/// A moment, as a person or a model writes one: `now`; a duration ago (`90m`, `3h ago`, `2d`);
/// RFC 3339; `2026-09-23 14:00` in this machine's time zone; `yesterday 14:00`; or a unix
/// timestamp. Never in the future: there is nothing there to measure.
pub fn parse_when(s: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    let bad = || {
        format!("“{t}” isn't a time this reads — use 2026-09-23 14:00 (this computer's time zone), RFC 3339, or a duration ago like 3h")
    };
    if lower.is_empty() {
        return Err(bad());
    }
    let at = if lower == "now" {
        now
    } else if let Ok(d) = parse_duration(lower.trim_end_matches("ago").trim()) {
        now - chrono::Duration::seconds(d)
    } else if let Ok(d) = DateTime::parse_from_rfc3339(t) {
        d.with_timezone(&Utc)
    } else if let Some(secs) = t.parse::<f64>().ok().filter(|v| *v > 1e9 && *v < 1e11) {
        Utc.timestamp_opt(secs as i64, 0).single().ok_or_else(bad)?
    } else {
        let local_now = now.with_timezone(&Local);
        let (day, time) = match lower.split_once(' ') {
            Some(("today", rest)) => (Some(local_now.date_naive()), rest),
            Some(("yesterday", rest)) => (local_now.date_naive().pred_opt(), rest),
            _ => (None, ""),
        };
        let naive = match day {
            Some(d) => NaiveTime::parse_from_str(time.trim(), "%H:%M").ok().map(|tm| d.and_time(tm)),
            None => ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M"]
                .iter()
                .find_map(|f| NaiveDateTime::parse_from_str(t, f).ok())
                .or_else(|| NaiveDate::parse_from_str(t, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0))),
        }
        .ok_or_else(bad)?;
        Local.from_local_datetime(&naive).earliest().ok_or_else(bad)?.with_timezone(&Utc)
    };
    if at > now + chrono::Duration::seconds(60) {
        return Err(format!("{} is in the future — there's nothing there to measure yet", local(at)));
    }
    Ok(at)
}

/// A moment in this machine's time zone, as the answers say it.
pub(super) fn local(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%Y-%m-%d %H:%M %Z").to_string()
}

/// The step for `span` seconds: about 240 points, never finer than 15 s, on a round number.
pub fn step_for(span: i64) -> i64 {
    const NICE: &[i64] = &[15, 30, 60, 120, 300, 600, 900, 1_800, 3_600, 7_200, 10_800, 21_600, 43_200, 86_400];
    let want = (span / POINTS).max(15);
    NICE.iter().copied().find(|&n| n >= want).unwrap_or(86_400)
}
