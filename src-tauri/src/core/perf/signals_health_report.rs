//! signals and the health report

use super::*;

/// A signal to watch, from Settings: a name and its PromQL.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub name: String,
    pub query: String,
}

/// Settings' "signals to watch" — `Name: PromQL`, one a line; `#` starts a comment — and what's
/// wrong with any line that isn't one.
pub fn signals(text: &str) -> (Vec<Signal>, Vec<String>) {
    let (mut out, mut problems) = (Vec::new(), Vec::new());
    for (n, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        match l.split_once(": ") {
            Some((name, query)) if !name.trim().is_empty() && !query.trim().is_empty() && name.trim().chars().count() <= 60 => {
                if out.len() == MAX_SIGNALS {
                    problems.push(format!("only the first {MAX_SIGNALS} signals are read"));
                    break;
                }
                out.push(Signal { name: name.trim().to_string(), query: query.trim().to_string() });
            }
            _ => problems.push(format!(
                "line {} isn't a signal — write it as a name, a colon and a space, then the query: “Checkout p95 latency: histogram_quantile(…)”",
                n + 1
            )),
        }
    }
    (out, problems)
}

/// The health report: each signal over the last `window` against the one before, the numbers
/// the code's, the reading the model's.
pub(super) async fn report(conn: &Conn, signal_text: &str, window: &str, now: DateTime<Utc>) -> Result<String, String> {
    let (list, problems) = signals(signal_text);
    if list.is_empty() {
        return Err("no signals are set. The person lists them in Settings ▸ General ▸ Product Performance — a name and a PromQL query a line.".into());
    }
    let window = parse_duration(window)?.min(30 * 86_400);
    let (start, cut) = (now - chrono::Duration::seconds(2 * window), now - chrono::Duration::seconds(window));
    let step = step_for(2 * window);
    let mut out = format!(
        "Health report: the {} to {} against the {} before. {} signal{}.",
        prom_duration(window),
        local(now),
        prom_duration(window),
        list.len(),
        if list.len() == 1 { "" } else { "s" }
    );
    let mut groups: BTreeMap<Verdict, Vec<String>> = BTreeMap::new();
    let mut failed = Vec::new();
    for sig in &list {
        match conn.range(&sig.query, start.timestamp() as f64, now.timestamp() as f64, step).await {
            Ok((Answer::Series(series, _), _)) if !series.is_empty() => {
                let labels: Vec<&BTreeMap<String, String>> = series.iter().map(|s| &s.labels).collect();
                let short = prom::short_names(&labels);
                let mut shifts: Vec<(usize, Shift)> = series
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| {
                        let b: Vec<f64> = s.points.iter().filter(|p| p.0 < cut.timestamp() as f64).map(|p| p.1).collect();
                        let a: Vec<f64> = s.points.iter().filter(|p| p.0 >= cut.timestamp() as f64).map(|p| p.1).collect();
                        shift(&b, &a).map(|sh| (i, sh))
                    })
                    .collect();
                shifts.sort_by(|a, b| a.1.verdict.cmp(&b.1.verdict));
                let link = conn.graph_link(&sig.query, now.timestamp() as f64, 2 * window).map(|l| format!(" — {l}")).unwrap_or_default();
                let has = |from: f64, to: f64| series.iter().any(|s| s.points.iter().any(|p| p.0 >= from && p.0 < to && p.1.is_finite()));
                let (had_before, has_now) = (has(start.timestamp() as f64, cut.timestamp() as f64), has(cut.timestamp() as f64, f64::INFINITY));
                match shifts.first() {
                    None if has_now && !had_before => groups.entry(Verdict::TooLittle).or_default().push(format!("{} — new: no data before {}{link}", sig.name, local(cut))),
                    None if had_before && !has_now => failed.push(format!("{} — gone: no data since {}", sig.name, local(cut))),
                    Some((i, sh)) => {
                        let which = if series.len() > 1 { format!(" ({})", short[*i]) } else { String::new() };
                        let others = shifts.iter().filter(|s| s.1.verdict == sh.verdict).count().saturating_sub(1);
                        let also = if others > 0 { format!(", and {others} more of its series like it") } else { String::new() };
                        groups.entry(sh.verdict).or_default().push(format!(
                            "{}{which} {}: {} → {}{also}{link}",
                            sig.name,
                            fmt_move(sh),
                            fmt_num(sh.before.mean),
                            fmt_num(sh.after.mean)
                        ));
                    }
                    None => failed.push(format!("{} — no data in one of the two windows", sig.name)),
                }
            }
            Ok(_) => failed.push(format!("{} — the query matched nothing", sig.name)),
            Err(e) => failed.push(format!("{} — {e}", sig.name)),
        }
    }
    for (verdict, title) in [(Verdict::Clear, "Clear shifts"), (Verdict::Possible, "Possible shifts"), (Verdict::Steady, "Steady"), (Verdict::TooLittle, "New, or too few points to judge")] {
        if let Some(lines) = groups.get(&verdict) {
            out.push_str(&format!("\n\n{title}:"));
            for l in lines {
                out.push_str(&format!("\n- {l}"));
            }
        }
    }
    if !failed.is_empty() {
        out.push_str("\n\nCouldn't read:");
        for f in failed {
            out.push_str(&format!("\n- {f}"));
        }
    }
    for p in problems {
        out.push_str(&format!("\n(Settings: {p}.)"));
    }
    Ok(out)
}
