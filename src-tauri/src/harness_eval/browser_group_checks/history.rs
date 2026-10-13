use super::super::*;

/// Whether a run had a browser the board's rows didn't.
///
/// On a scratch instance the driver switches the browser off for every task but a browser
/// task, so every row is measured with the same tools. On any other instance it may not change
/// the setting — that instance is somebody's app — so when the app-wide switch is on and the
/// assistant under test holds its own browser, the turn is offered nine tools the board's rows
/// never were. `settings` is `GET /v1/settings`; `assistant` is `GET /v1/personas/assistant`,
/// `None` when it couldn't be read, which with the switch on is reported rather than assumed.
pub fn browser_note(settings: &serde_json::Value, assistant: Option<&serde_json::Value>) -> Option<String> {
    if !settings["browser_enabled"].as_bool().unwrap_or(false) {
        return None;
    }
    let holds = |a: &serde_json::Value| a["tools"].as_array().is_some_and(|t| t.iter().any(|x| x == "browser"));
    match assistant {
        Some(a) if !holds(a) => None,
        Some(_) => Some(
            "ran with the browser on: this isn't a scratch instance, so the driver couldn't switch it off, and the assistant had its own"
                .into(),
        ),
        None => Some("ran with the browser switched on, and whether the assistant had its own couldn't be read".into()),
    }
}

///
/// Exists to serve `HARNESS_EVAL_BAND` (HARNESS-RESEARCH-2026 §6c): score only
/// the tasks whose historical pass rate sits in the informative band, where item
/// response theory says the discriminative information is. Applied to our own
/// board, `one_word`, `read_named_file`, `shell_reports_output` and
/// `counts_what_is_there` pass everywhere and carry no information;
/// `grep_before_reading_everything` and `does_not_invent_a_field` fail everywhere
/// and carry none either.
///
/// Written by the sweep rather than annotated on each `Task`, because a
/// hand-maintained band is a number that goes stale — which is the exact failure
/// round 1 earned a rule about. This one carries the date and the marker of the
/// run that produced it, so a stale history is visible rather than silent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    /// When this was measured, ISO-8601.
    pub measured_at: String,
    /// The run marker, so a rate can be traced to the sweep that produced it.
    pub marker: String,
    /// Task id -> pass rate over all models and runs, 0.0..=1.0.
    pub rates: std::collections::BTreeMap<String, f64>,
    /// Task id -> median seconds across every measured run.
    ///
    /// The `seconds` column has existed for a while and nothing has ever asserted
    /// on it, which is HARNESS-RESEARCH-2026 §5's complaint: a latency regression
    /// lands silently. Median rather than mean, because one 108-second step on a
    /// 12B model should not redefine the baseline for the task.
    #[serde(default)]
    pub seconds: std::collections::BTreeMap<String, f64>,
}

/// The informative band. Outside it a task is either free or hopeless, and either
/// way it separates nothing.
pub const BAND: std::ops::RangeInclusive<f64> = 0.30..=0.70;

impl History {
    /// Build it from a finished sweep's outcomes.
    pub fn from_outcomes(marker: &str, outcomes: &[Outcome]) -> Self {
        let mut ids: Vec<&str> = outcomes
            .iter()
            .filter(|o| o.verdict != Verdict::NoResult)
            .map(|o| o.task)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let mut rates = std::collections::BTreeMap::new();
        let mut seconds = std::collections::BTreeMap::new();
        for id in ids {
            let runs: Vec<&Outcome> = outcomes
                .iter()
                .filter(|o| o.task == id && o.verdict != Verdict::NoResult)
                .collect();
            if runs.is_empty() {
                continue;
            }
            let passed = runs.iter().filter(|o| o.verdict == Verdict::Pass).count();
            rates.insert(id.to_string(), passed as f64 / runs.len() as f64);
            let mut secs: Vec<f64> = runs.iter().map(|o| o.seconds).collect();
            secs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            if let Some(m) = secs.get(secs.len() / 2) {
                seconds.insert(id.to_string(), *m);
            }
        }
        Self {
            measured_at: chrono::Utc::now().to_rfc3339(),
            marker: marker.to_string(),
            rates,
            seconds,
        }
    }

    /// Tasks that got materially slower than `prev`, as `(id, was, now)`.
    ///
    /// `TOLERANCE` is wide on purpose. This machine has been measured running a
    /// build alongside a sweep, and the `seconds` column notices that; a tight
    /// threshold would spend its life reporting the laptop rather than the harness.
    /// A real latency regression from a harness change is not 20% — the thing §5
    /// proposes is worth 48%, and the thing that would undo it is comparable.
    pub fn slower_than(&self, prev: &History) -> Vec<(String, f64, f64)> {
        const TOLERANCE: f64 = 1.5;
        let mut out = Vec::new();
        for (id, now) in &self.seconds {
            let Some(was) = prev.seconds.get(id) else { continue };
            // A task that used to take under a second is noise, not a baseline.
            if *was >= 1.0 && *now > was * TOLERANCE {
                out.push((id.clone(), *was, *now));
            }
        }
        out.sort_by(|a, b| (b.2 / b.1).partial_cmp(&(a.2 / a.1)).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    /// Whether a task sits in the informative band. Unknown tasks are kept: a task
    /// with no history has not earned exclusion, and dropping it would make a new
    /// task invisible on the very run that should measure it.
    pub fn informative(&self, id: &str) -> bool {
        match self.rates.get(id) {
            Some(r) => BAND.contains(r),
            None => true,
        }
    }
}
