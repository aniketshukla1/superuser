use super::*;

    /// Where the pass-rate history lives. Beside the scoreboard it describes.
    pub(super) fn history_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../HARNESS-HISTORY.json")
    }

    /// The history, if one has been recorded.
    pub(super) fn history() -> Option<History> {
        let raw = std::fs::read_to_string(history_path()).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// Whether this task is in the run, per `HARNESS_EVAL_TASKS`.
    ///
    /// Unset means everything, so a plain run is still the full sweep — a filter that
    /// silently narrowed the default would let a partial result be mistaken for a
    /// complete one, and the report has no way to tell the difference.
    pub(super) fn task_selected(id: &str) -> bool {
        // An explicit list always wins: it is someone naming what they want.
        if let Ok(v) = std::env::var("HARNESS_EVAL_TASKS") {
            if !v.trim().is_empty() {
                return v.split(',').any(|t| t.trim() == id);
            }
        }
        // `HARNESS_EVAL_BAND=1` narrows to the informative band (§6c). Opt-in, and
        // it stays opt-in: a default that silently scored a subset would make a
        // partial board look like a full one, and every table in HARNESS-SCOREBOARD
        // would stop meaning what its header says.
        let band_on = std::env::var("HARNESS_EVAL_BAND").map(|v| v == "1" || v == "true").unwrap_or(false);
        if !band_on {
            return true;
        }
        match history() {
            Some(h) => h.informative(id),
            None => {
                // No history is not a reason to run a quarter of the suite.
                eprintln!("HARNESS_EVAL_BAND is set but no history has been recorded yet — running the full suite");
                true
            }
        }
    }

    pub(super) fn sanitise(s: &str) -> String {
        s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
    }
