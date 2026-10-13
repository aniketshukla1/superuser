use super::*;

/// One tool's record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    pub name: String,
    pub calls: usize,
    /// Calls that came back an error.
    pub failures: usize,
    /// Calls that were proposed and never came back at all — refused by a gate,
    /// parked for approval, or the turn ended under them.
    ///
    /// Kept apart from `failures` deliberately. Folding them together was the first
    /// version and it libelled `ask_user`: a blocking question the user never got
    /// round to answering is not the tool failing, it is the tool doing exactly what
    /// it says on the tin. `read_file` returning "no such path" is a different event
    /// and the whole value of this table is that the two are told apart.
    pub unfinished: usize,
    /// Distinct contexts it appeared in — task ids for a sweep, session ids for real
    /// usage. A tool called forty times in one session is a different fact from one
    /// called forty times across forty.
    pub contexts: usize,
}

impl Row {
    /// Share of calls that came back an error, 0.0–1.0. Zero calls is zero, not NaN:
    /// this number is formatted into a table and a NaN there reads as a bug.
    pub fn failure_rate(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.failures as f64 / self.calls as f64
        }
    }
}

/// Running counts, folded from either source.
#[derive(Debug, Default)]
pub struct Tally {
    pub(super) seen: BTreeMap<String, (usize, usize, usize, std::collections::BTreeSet<String>)>,
}

impl Tally {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one call that came back. `context` is the task or session it was in.
    pub fn record(&mut self, name: &str, failed: bool, context: &str) {
        let e = self.seen.entry(name.to_string()).or_default();
        e.0 += 1;
        if failed {
            e.1 += 1;
        }
        e.3.insert(context.to_string());
    }

    /// Record a call that was proposed and never came back.
    pub fn record_unfinished(&mut self, name: &str, context: &str) {
        let e = self.seen.entry(name.to_string()).or_default();
        e.0 += 1;
        e.2 += 1;
        e.3.insert(context.to_string());
    }

    /// Rows, most-called first, then by name so a report is stable run to run.
    pub fn rows(&self) -> Vec<Row> {
        let mut out: Vec<Row> = self
            .seen
            .iter()
            .map(|(name, (calls, failures, unfinished, ctx))| Row {
                name: name.clone(),
                calls: *calls,
                failures: *failures,
                unfinished: *unfinished,
                contexts: ctx.len(),
            })
            .collect();
        out.sort_by(|a, b| b.calls.cmp(&a.calls).then_with(|| a.name.cmp(&b.name)));
        out
    }

    pub fn total_calls(&self) -> usize {
        self.seen.values().map(|(c, _, _, _)| c).sum()
    }

    /// Catalogue tools that were never called here, with the capability they belong
    /// to — because "uncalled" and "unavailable" look identical in a call log, and
    /// the capability is what tells them apart.
    pub fn never_called(&self, catalogue: &[(&'static str, String)]) -> Vec<(&'static str, String)> {
        catalogue.iter().filter(|(_, n)| !self.seen.contains_key(n)).cloned().collect()
    }

    /// Tools that were called and are **not** in the catalogue — MCP, mostly. Worth
    /// surfacing rather than dropping: an MCP server the user actually leans on is
    /// the strongest argument there is about what the catalogue is missing.
    pub fn off_catalogue(&self, catalogue: &[(&'static str, String)]) -> Vec<Row> {
        let known: std::collections::HashSet<&str> = catalogue.iter().map(|(_, n)| n.as_str()).collect();
        self.rows().into_iter().filter(|r| !known.contains(r.name.as_str())).collect()
    }

    /// The table, as markdown.
    pub fn render(&self, title: &str, catalogue: &[(&'static str, String)], resident: &[&str]) -> String {
        let mut out = format!("## {title}\n\n");
        if self.seen.is_empty() {
            out.push_str("No tool calls recorded.\n");
            return out;
        }
        out.push_str("| tool | calls | errored | rate | unfinished | contexts | resident |\n|---|---|---|---|---|---|---|\n");
        for r in self.rows() {
            out.push_str(&format!(
                "| `{}` | {} | {} | {:.0}% | {} | {} | {} |\n",
                r.name,
                r.calls,
                r.failures,
                r.failure_rate() * 100.0,
                r.unfinished,
                r.contexts,
                if resident.contains(&r.name.as_str()) { "yes" } else { "" }
            ));
        }
        let never = self.never_called(catalogue);
        out.push_str(&format!(
            "\n**{} of {} catalogue tools were never called.**",
            never.len(),
            catalogue.len()
        ));
        if !never.is_empty() {
            let mut by_cap: BTreeMap<&str, Vec<String>> = BTreeMap::new();
            for (cap, name) in &never {
                by_cap.entry(cap).or_default().push(name.clone());
            }
            out.push('\n');
            for (cap, names) in by_cap {
                out.push_str(&format!("\n* `{cap}` — {}", names.join(", ")));
            }
            out.push('\n');
        }
        // The two directions that matter, spelled out rather than left to the reader.
        let resident_uncalled: Vec<&str> =
            resident.iter().copied().filter(|n| !self.seen.contains_key(*n)).collect();
        if !resident_uncalled.is_empty() {
            out.push_str(&format!(
                "\n**Resident but never called here: {}.** These are paid for on every single \
                 request, so an unused one is the most expensive kind of unused.\n",
                resident_uncalled.join(", ")
            ));
        }
        let off = self.off_catalogue(catalogue);
        if !off.is_empty() {
            out.push_str(&format!(
                "\n**Called but not in the catalogue ({}):** {}. MCP, or a tool that has been \
                 renamed out from under this list.\n",
                off.len(),
                off.iter().map(|r| format!("`{}`×{}", r.name, r.calls)).collect::<Vec<_>>().join(", ")
            ));
        }
        out
    }
}

/// Did this tool result read as a failure?
///
/// The same rule `commands::tool_status` uses for the UI, restated here so the
/// counting does not depend on a display function that is free to change its mind
/// about colours.
pub fn is_failure(result: &str) -> bool {
    let head = result.trim_start().chars().take(120).collect::<String>().to_lowercase();
    head.starts_with("error") || head.starts_with("unknown ") || head.contains("failed:")
}
