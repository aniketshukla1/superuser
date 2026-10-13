
/// The assistant (`personas_builtin/perf.md`).
pub const ASSISTANT: &str = "perf";

/// The weekly health report routine.
pub const ROUTINE_TITLE: &str = "Weekly health report";

pub const ROUTINE_CRON: &str = "30 9 * * 1";

pub const ROUTINE_INSTRUCTIONS: &str = "Write this week's health report. Call perf_report for the last 7 days against the 7 \
     before, then report what moved — clear shifts first, each with its numbers and its link — and name what held steady. \
     Keep facts and hypotheses apart, and don't change anything.";

/// Series listed per group in one answer; the rest are counted.
pub(super) const SHOWN: usize = 12;

/// Series written to a chart's CSV: a chart of forty lines is a chart of none.
pub(super) const CHARTED: usize = 8;

/// About how many points a range asks for.
pub(super) const POINTS: i64 = 240;

/// Signals a health report runs.
pub const MAX_SIGNALS: usize = 20;
