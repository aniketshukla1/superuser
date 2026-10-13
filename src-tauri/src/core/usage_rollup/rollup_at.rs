use super::*;

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Window {
    pub input: u64,
    pub output: u64,
    pub tokens: u64,
    /// Assistant turns in this window that reported what they cost.
    pub counted: u64,
    /// Assistant turns in this window that reported nothing. The total is a floor while
    /// this is above zero, and the UI is expected to say so rather than round it away.
    pub uncounted: u64,
    /// Unix seconds when this window rolls over.
    pub resets_at: f64,
}

impl Window {
    /// True when every turn in the window reported its cost, so the total is the total.
    pub fn complete(&self) -> bool {
        self.uncounted == 0
    }
}

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Rollup {
    pub today: Window,
    pub week: Window,
    /// Echoed back so the client is not guessing what "now" the boundaries came from.
    pub now: f64,
}

/// Local midnight tonight — when "today" turns over.
pub(super) fn next_midnight(now: DateTime<Local>) -> f64 {
    let start = (now + Duration::days(1))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|d| Local.from_local_datetime(&d).earliest());
    match start {
        Some(t) => t.timestamp() as f64,
        // A DST spring-forward can delete a local midnight. Falling back to "24h from the
        // start of today" keeps the window monotonic instead of returning nothing.
        None => (now.timestamp() + 86_400) as f64,
    }
}

/// The coming Monday 00:00 local — when "this week" turns over.
pub(super) fn next_week_start(now: DateTime<Local>) -> f64 {
    let days_ahead = 7 - now.weekday().num_days_from_monday() as i64;
    let target = now + Duration::days(days_ahead);
    match target
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|d| Local.from_local_datetime(&d).earliest())
    {
        Some(t) => t.timestamp() as f64,
        None => (now.timestamp() + days_ahead * 86_400) as f64,
    }
}

/// The start of the current week — Monday 00:00 local.
pub(super) fn week_start(now: DateTime<Local>) -> f64 {
    let back = now.weekday().num_days_from_monday() as i64;
    let target = now - Duration::days(back);
    match target
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|d| Local.from_local_datetime(&d).earliest())
    {
        Some(t) => t.timestamp() as f64,
        None => (now.timestamp() - back * 86_400 - now.num_seconds_from_midnight() as i64) as f64,
    }
}

pub(super) fn day_start(now: DateTime<Local>) -> f64 {
    match now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|d| Local.from_local_datetime(&d).earliest())
    {
        Some(t) => t.timestamp() as f64,
        None => now.timestamp() as f64 - now.num_seconds_from_midnight() as f64,
    }
}

/// Sum every assistant turn in the two windows.
///
/// Takes the sessions rather than reaching for a store so the boundaries and the
/// missing-usage accounting can be tested without one.
pub fn rollup_at(sessions: &[Session], now_unix: f64) -> Rollup {
    let now = Local
        .timestamp_opt(now_unix as i64, 0)
        .earliest()
        .unwrap_or_else(Local::now);
    let (d0, w0) = (day_start(now), week_start(now));
    let mut out = Rollup {
        now: now_unix,
        today: Window { resets_at: next_midnight(now), ..Default::default() },
        week: Window { resets_at: next_week_start(now), ..Default::default() },
    };
    for s in sessions {
        for m in &s.messages {
            // User turns cost nothing on their own; their tokens are billed as the
            // input of the assistant turn that answered them, and counting both would
            // be counting the same text twice.
            if m.role != "assistant" || m.at <= 0.0 || m.at > now_unix {
                continue;
            }
            for (w, since) in [(&mut out.today, d0), (&mut out.week, w0)] {
                if m.at < since {
                    continue;
                }
                match &m.usage {
                    Some(u) => {
                        w.input += u.input;
                        w.output += u.output;
                        w.tokens += u.total();
                        w.counted += 1;
                    }
                    None => w.uncounted += 1,
                }
            }
        }
    }
    out
}
