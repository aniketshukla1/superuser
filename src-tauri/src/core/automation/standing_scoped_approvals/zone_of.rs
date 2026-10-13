use super::super::*;

/// The five fields, if this is a cron expression at all.
pub fn cron_fields(cron: &str) -> Option<[&str; 5]> {
    let p: Vec<&str> = cron.split_whitespace().collect();
    if p.len() != 5 {
        return None;
    }
    let spans = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
    for (f, span) in p.iter().zip(spans) {
        let _ = span;
        if !parses_field(f) {
            return None;
        }
    }
    Some([p[0], p[1], p[2], p[3], p[4]])
}

/// A schedule's clock: the machine's own, a fixed offset from UTC, or a named zone.
///
/// A named zone (`Europe/Madrid`) is read from the system's zone database, so "09:00" stays
/// nine o'clock there across daylight saving. A name the database doesn't have is refused
/// when the schedule is made rather than evaluated in the wrong hours — a routine that fires
/// at the wrong time looks like a routine that works.
#[derive(Debug, Clone, PartialEq)]
pub enum Zone {
    Local,
    /// Seconds east of UTC.
    Fixed(i32),
    /// A zone database name, as written — checked to exist when it was read.
    Named(String),
}

/// The zone a schedule names, or `None` when it names one this build cannot evaluate.
pub fn zone_of(name: &str) -> Option<Zone> {
    let n = name.trim();
    if n.is_empty() || n.eq_ignore_ascii_case("local") {
        return Some(Zone::Local);
    }
    if n.eq_ignore_ascii_case("utc") || n.eq_ignore_ascii_case("z") || n.eq_ignore_ascii_case("gmt") {
        return Some(Zone::Fixed(0));
    }
    // `Europe/Madrid`, `America/New_York` — from the system's zone database.
    if named_zone(n).is_some() {
        return Some(Zone::Named(n.to_string()));
    }
    // `+02:00`, `-0500`, `UTC+1`.
    let body = n.strip_prefix("UTC").or_else(|| n.strip_prefix("utc")).unwrap_or(n);
    let (sign, rest) = match body.chars().next()? {
        '+' => (1, &body[1..]),
        '-' => (-1, &body[1..]),
        _ => return None,
    };
    let (h, m) = match rest.split_once(':') {
        Some((h, m)) => (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?),
        None if rest.len() == 4 => (rest[..2].parse::<i32>().ok()?, rest[2..].parse::<i32>().ok()?),
        None => (rest.parse::<i32>().ok()?, 0),
    };
    (h <= 14 && m < 60).then(|| Zone::Fixed(sign * (h * 3600 + m * 60)))
}

/// The zone database's view of a schedule's clock: the named zone, or this machine's own. `None`
/// for a fixed offset, which has no daylight saving to get wrong — and for a machine whose zone
/// the database can't read, which falls back to `chrono`'s reading of it.
///
/// Local time goes through the database too. Through `chrono` alone, a
/// time the clock skips or repeats resolved to nothing, so a 02:30 job didn't run on the
/// spring-forward day and a 01:30 job didn't run on the fall-back day.
pub(in crate::core::automation) fn calendar_zone(zone: &Zone) -> Option<jiff::tz::TimeZone> {
    match zone {
        Zone::Named(name) => named_zone(name),
        Zone::Local => jiff::tz::TimeZone::try_system().ok(),
        Zone::Fixed(_) => None,
    }
}

/// The moment a wall-clock time names in `tz`, with daylight saving as cron has it: a time the
/// clock skips happens once the clock has jumped (02:30 is 03:30), and a time it repeats happens
/// the first time.
pub(in crate::core::automation) fn instant_in(tz: &jiff::tz::TimeZone, at: &chrono::NaiveDateTime) -> Option<f64> {
    use chrono::{Datelike, Timelike};
    let civil = jiff::civil::date(i16::try_from(at.year()).ok()?, i8::try_from(at.month()).ok()?, i8::try_from(at.day()).ok()?).at(
        i8::try_from(at.hour()).ok()?,
        i8::try_from(at.minute()).ok()?,
        i8::try_from(at.second()).ok()?,
        0,
    );
    civil.to_zoned(tz.clone()).ok().map(|z| z.timestamp().as_second() as f64)
}

/// The calendar date a moment falls on in `tz`.
pub(in crate::core::automation) fn date_in(tz: &jiff::tz::TimeZone, ts: f64) -> Option<chrono::NaiveDate> {
    let z = jiff::Timestamp::from_second(ts as i64).ok()?.to_zoned(tz.clone());
    chrono::NaiveDate::from_ymd_opt(i32::from(z.year()), u32::try_from(z.month()).ok()?, u32::try_from(z.day()).ok()?)
}

/// The zone as the calendar code reads it: a named zone resolved, everything else as it is.
pub(in crate::core::automation) fn named_zone(name: &str) -> Option<jiff::tz::TimeZone> {
    // Names only: an offset or a bare word the database happens to hold ("EST") is not what
    // anyone setting "9am in Madrid" wrote, and the parser above already covers offsets.
    if !name.contains('/') || name.len() > 64 || name.contains("..") {
        return None;
    }
    jiff::tz::TimeZone::get(name).ok()
}

/// What a firing is told, ahead of the instruction.
///
/// Without it the instruction reads as a question about the automation instead of as the
/// work: a routine that was supposed to append a line to a page once read its
/// own instruction as "what routines do I have?", listed them, did nothing, and finished
/// green. The frame says which turn this is and what to do in it.
pub const FIRING_FRAME: &str = "One of the user's automations is firing right now, on its schedule, and this turn is that firing. \
Carry out the instruction below in this turn: do the work now, then say what happened. Don't create, list or change any \
automation unless the instruction itself asks you to.";

/// The same frame for a run someone started with Run now. The instruction is the same, for the
/// same reason — a manual run of "every morning, summarise my inbox" is just as likely to be read
/// as a request to schedule something — and only the first sentence differs, because "on its
/// schedule" would be a false thing to tell a model about a run a person just started.
pub const MANUAL_FRAME: &str = "One of the user's automations is running right now because they pressed Run now, and this turn is that run. \
Carry out the instruction below in this turn: do the work now, then say what happened. Don't create, list or change any \
automation unless the instruction itself asks you to.";

/// An automation's instruction as the model is given it on a firing or a manual run.
pub fn framed(instructions: &str, manual: bool) -> String {
    format!("{}\n\n{}", if manual { MANUAL_FRAME } else { FIRING_FRAME }, instructions)
}

/// The instruction without the frame, for anything that shows a firing to a person.
pub fn without_frame(message: &str) -> &str {
    [FIRING_FRAME, MANUAL_FRAME]
        .iter()
        .find_map(|frame| message.strip_prefix(frame))
        .map(|rest| rest.trim_start())
        .unwrap_or(message)
}

/// What a turn in `session_id` should hand the model: the message framed as a manual run when
/// it opens one, and as it was otherwise.
///
/// "Run now" in the GUI starts a run, opens its session and sends the instruction through the
/// ordinary chat socket, so this is where it can be framed without the person seeing the frame
/// in their transcript. It opens the run only while the run is still going and the message is
/// that automation's instruction: a follow-up typed into the same conversation afterwards is
/// a conversation, not a firing.
pub fn frame_for_turn(data_dir: &std::path::Path, session_id: &str, message: String) -> String {
    if !session_id.starts_with("__run__") {
        return message;
    }
    if AutomationStore::load(data_dir).opens_manual_run(session_id, &message) {
        framed(&message, true)
    } else {
        message
    }
}

/// The closest two firings a schedule is allowed to have. A routine that fires every
/// minute is not a routine, it is a loop with a person's name on it.
pub const FLOOR_SECS: f64 = 15.0 * 60.0;

/// How many enabled automations one machine may have. Past twenty, nobody knows what
/// their assistants are doing.
pub const MAX_ENABLED: usize = 20;

/// How late a firing may be before it is skipped rather than run.
///
/// The alternative is a backlog replay — thirty morning briefs at once after a laptop has
/// been shut for a month — which is worse than the miss it is trying to make up for.
pub const GRACE_SECS: f64 = 10.0 * 60.0;

/// How long a run may be in flight before it is written off.
pub const ABANDONED_SECS: f64 = 10.0 * 60.0;

/// Consecutive failures before an automation switches itself off.
pub const FATIGUE_LIMIT: u32 = 10;
