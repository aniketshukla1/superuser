use super::super::*;

// An `always_allowed_tools` entry is either a bare tool name (allows the tool against
// any argument) or "tool target" (one space; tool names never contain spaces) binding
// the allowance to one exact target (a channel, a recipient, …). Rules live on the task
// so revocation is per-automation and deleting the task takes them along.

pub fn rule_entry(tool: &str, target: Option<&str>) -> String {
    match target {
        Some(t) if !t.is_empty() => format!("{tool} {t}"),
        _ => tool.to_string(),
    }
}

pub fn rule_parts(entry: &str) -> (String, Option<String>) {
    let entry = entry.trim();
    match entry.split_once(' ') {
        Some((tool, target)) => {
            let t = target.trim();
            (tool.to_string(), if t.is_empty() { None } else { Some(t.to_string()) })
        }
        None => (entry.to_string(), None),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub kind: String, // "cron" | "once"
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub fire_at: Option<String>, // ISO datetime for one-time
    #[serde(default = "local_tz")]
    pub timezone: String,
}

impl Schedule {
    /// Best-effort human label ("Every day at ~7:10 PM"); falls back to raw cron.
    pub fn human(&self) -> String {
        if self.kind == "once" {
            let at = self.fire_at.clone().unwrap_or_default();
            // A pinned time (see `pin_fire_at`) reads as the clock time it was written in, with
            // the zone it was written for — not as an offset a person has to do sums with.
            if let Ok(d) = chrono::DateTime::parse_from_rfc3339(at.trim()) {
                let zone = match zone_of(&self.timezone) {
                    Some(Zone::Local) | None => format!(" ({})", d.format("%:z")),
                    Some(Zone::Fixed(0)) => " (UTC)".into(),
                    Some(_) => format!(" ({})", self.timezone.trim()),
                };
                return format!("Once at {}{zone}", d.format("%Y-%m-%d %H:%M"));
            }
            return format!("Once at {at}");
        }
        let cron = self.cron.clone().unwrap_or_default();
        let parts: Vec<&str> = cron.split_whitespace().collect();
        if parts.len() != 5 {
            return if cron.is_empty() { "?".into() } else { cron };
        }
        let (minute, hour, dom, _month, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);
        // Said where it is not the machine's own clock, because "9am" in another zone is
        // a different time and a label that hides that is worse than no label.
        let zone = match zone_of(&self.timezone) {
            Some(Zone::Local) | None => String::new(),
            Some(Zone::Fixed(0)) => " (UTC)".into(),
            Some(Zone::Fixed(_)) | Some(Zone::Named(_)) => format!(" ({})", self.timezone.trim()),
        };
        // The repeating shapes people write, in the words they would say.
        if let Some(step) = minute.strip_prefix("*/").and_then(|n| n.parse::<u32>().ok()) {
            if hour == "*" && dom == "*" && dow == "*" && step > 0 && 60 % step == 0 {
                return format!("Every {step} minutes{zone}");
            }
        }
        if let (Ok(m), Some(step)) = (minute.parse::<u32>(), hour.strip_prefix("*/").and_then(|n| n.parse::<u32>().ok())) {
            if dom == "*" && dow == "*" && step > 0 {
                return if m == 0 {
                    format!("Every {step} hours, on the hour{zone}")
                } else {
                    format!("Every {step} hours at :{m:02}{zone}")
                };
            }
        }
        let (h, m) = match (hour.parse::<u32>(), minute.parse::<u32>()) {
            (Ok(h), Ok(m)) => (h, m),
            _ => return format!("{cron}{zone}"), // a shape with no plain words for it
        };
        let t = human_time(h, m);
        if dom == "*" && dow == "*" {
            return format!("Every day at ~{t}{zone}");
        }
        if dom == "*" && dow != "*" {
            // Standard cron day-of-week: 0 and 7 are both Sunday.
            let name = |d: usize| DOW[(d + 6) % 7];
            if let Ok(d) = dow.parse::<usize>() {
                return format!("Every {} at ~{t}{zone}", name(d % 7));
            }
            // `1-5` and `1,3,5` — spell out the days rather than showing raw cron.
            if parses_field(dow) {
                let days: Vec<&str> = (0..7).filter(|&i| field_matches(dow, i, true, (0, 7))).map(|i| name(i as usize)).collect();
                if days.len() == 5 && field_matches(dow, 1, true, (0, 7)) && field_matches(dow, 5, true, (0, 7)) && !field_matches(dow, 0, true, (0, 7)) {
                    return format!("Every weekday at ~{t}{zone}");
                }
                if !days.is_empty() {
                    return format!("Every {} at ~{t}{zone}", days.join(", "));
                }
            }
        }
        if dow == "*" && dom.chars().all(|c| c.is_ascii_digit()) {
            return format!("Monthly on day {dom} at ~{t}{zone}");
        }
        format!("{cron}{zone}")
    }
}

pub(in crate::core::automation) fn human_time(hour: u32, minute: u32) -> String {
    let ampm = if hour < 12 { "AM" } else { "PM" };
    let h12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{h12}:{minute:02} {ampm}")
}

/// The next fire time strictly after `after` (epoch seconds), or None if the schedule
/// has no future fire. Supports daily / weekly (dow, Monday=0)
/// / monthly (day-of-month) cron patterns and one-time (`once`) schedules, evaluated in
/// the machine's local time. Complex cron (ranges/steps) and non-local timezones aren't
/// computed here yet (returns None) — a scheduler concern to extend.
/// Does a cron field match a value? Handles every form people actually write: `*`, a
/// number (`3`), a list (`1,3,5`), a range (`1-5`), a step (`*/20`, `9-17/2`), and
/// combinations of those.
///
/// `wrap` is set for day-of-week, where 0 and 7 both mean Sunday. Day-of-month must
/// NOT wrap — mod-7 there would make `1` match the 8th, 15th and 22nd as well.
///
/// A field that cannot be read matches nothing, and every caller checks
/// [`parses_field`] first, so an unreadable expression is refused rather than quietly
/// meaning "never".
pub fn field_matches(field: &str, value: u32, wrap: bool, span: (u32, u32)) -> bool {
    let eq = |d: u32| d == value || (wrap && d % 7 == value % 7);
    field.split(',').any(|part| {
        let part = part.trim();
        let (base, step) = match part.split_once('/') {
            Some((b, s)) => match s.trim().parse::<u32>() {
                Ok(n) if n > 0 => (b.trim(), n),
                _ => return false,
            },
            None => (part, 1),
        };
        let (from, to) = if base == "*" {
            span
        } else {
            match base.split_once('-') {
                Some((a, b)) => match (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
                    (Ok(a), Ok(b)) if a <= b => (a, b),
                    _ => return false,
                },
                None => match base.parse::<u32>() {
                    // A bare number with a step means "from here on", which is what
                    // `5/10` means in cron and what someone writing it expects.
                    Ok(n) if step > 1 => (n, span.1),
                    Ok(n) => (n, n),
                    Err(_) => return false,
                },
            }
        };
        (from..=to).step_by(step as usize).any(eq)
    })
}

/// True if every element of a field is something [`field_matches`] can read.
pub fn parses_field(field: &str) -> bool {
    !field.is_empty()
        && field.split(',').all(|part| {
            let part = part.trim();
            let (base, step_ok) = match part.split_once('/') {
                Some((b, s)) => (b.trim(), s.trim().parse::<u32>().is_ok_and(|n| n > 0)),
                None => (part, true),
            };
            step_ok
                && (base == "*"
                    || match base.split_once('-') {
                        Some((a, b)) => a.trim().parse::<u32>().is_ok() && b.trim().parse::<u32>().is_ok(),
                        None => base.parse::<u32>().is_ok(),
                    })
        })
}
