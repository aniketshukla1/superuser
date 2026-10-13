use super::super::*;

/// A tool's answer: what the model reads, and the files it kept (marked read, so a chart may
/// cite them).
#[derive(Debug)]
pub struct Out {
    pub text: String,
    pub files: Vec<String>,
}

pub(in crate::core::perf) fn arg<'a>(args: &'a Value, key: &str) -> &'a str {
    args[key].as_str().map(str::trim).unwrap_or("")
}

pub(in crate::core::perf) fn link_line(conn: &Conn, query: &str, end: f64, span: i64) -> String {
    conn.graph_link(query, end, span).map(|l| format!("\nOpen it in Prometheus: {l}")).unwrap_or_default()
}

pub(in crate::core::perf) fn warn_line(w: &[String]) -> String {
    if w.is_empty() { String::new() } else { format!("\nPrometheus warned: {}", w.join("; ")) }
}

/// The tools the Product Performance assistant — or any assistant granted `metrics` — calls.
pub async fn run(conn: &Conn, name: &str, args: &Value, signals: &str, root: &Path, now: DateTime<Utc>) -> Result<Out, String> {
    let text = |t: String| Out { text: t, files: vec![] };
    match name {
        "metrics_find" => find(conn, arg(args, "text"), now).await.map(text),
        "metrics_query" => {
            let q = arg(args, "query");
            if q.is_empty() {
                return Err("give the PromQL `query` to run".into());
            }
            let at = match arg(args, "at") {
                "" => now,
                a => parse_when(a, now)?,
            };
            let (answer, w) = conn.instant(q, at.timestamp() as f64).await?;
            Ok(text(instant_text(conn, q, at, answer, &w)))
        }
        "metrics_range" => {
            let q = arg(args, "query");
            if q.is_empty() {
                return Err("give the PromQL `query` to run".into());
            }
            let start = parse_when(if arg(args, "since").is_empty() { "1h" } else { arg(args, "since") }, now)?;
            let end = match arg(args, "until") {
                "" => now,
                u => parse_when(u, now)?,
            };
            if end <= start {
                return Err("`until` has to come after `since`".into());
            }
            let span = (end - start).num_seconds();
            let step = match arg(args, "step") {
                "" => step_for(span),
                s => parse_duration(s)?.max(span / 11_000 + 1),
            };
            let (answer, w) = conn.range(q, start.timestamp() as f64, end.timestamp() as f64, step).await?;
            range_text(conn, q, start, end, step, answer, &w, root, now)
        }
        "metrics_compare" => {
            let q = arg(args, "query");
            if q.is_empty() {
                return Err("give the PromQL `query` to compare".into());
            }
            if arg(args, "change_at").is_empty() {
                return Err("give `change_at` — when the release or change happened, e.g. 2026-09-23 14:00".into());
            }
            let change = parse_when(arg(args, "change_at"), now)?;
            let window = match arg(args, "window") {
                "" => 3_600,
                w => parse_duration(w)?.min(14 * 86_400),
            };
            let start = change - chrono::Duration::seconds(window);
            let end = (change + chrono::Duration::seconds(window)).min(now);
            if (end - change).num_seconds() < 120 {
                return Err(format!("{} was just now — there's nothing after it to compare yet", local(change)));
            }
            let step = step_for((end - start).num_seconds());
            let (answer, w) = conn.range(q, start.timestamp() as f64, end.timestamp() as f64, step).await?;
            compare_text(conn, q, start, change, end, step, answer, &w, root, now)
        }
        "metrics_alerts" => Ok(text(alerts_text(&conn.alerts().await?, now))),
        "perf_report" => report(conn, signals, if arg(args, "window").is_empty() { "7d" } else { arg(args, "window") }, now).await.map(text),
        other => Err(format!("{other} isn't a metrics tool")),
    }
}

pub(in crate::core::perf) async fn find(conn: &Conn, text: &str, now: DateTime<Utc>) -> Result<String, String> {
    if text.is_empty() {
        return Err("give `text`: words to look for in metric names, or one exact metric name".into());
    }
    let names = conn.metric_names().await?;
    if names.iter().any(|n| n == text) {
        let meta = conn.metadata(text).await.unwrap_or_default();
        let sets = conn.series(text, (now.timestamp() - 3_600) as f64, now.timestamp() as f64).await?;
        let mut values: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for set in &sets {
            for (k, v) in set {
                if k != "__name__" {
                    let e = values.entry(k).or_default();
                    if !e.contains(&v.as_str()) {
                        e.push(v);
                    }
                }
            }
        }
        let mut out = format!("{text}");
        if !meta.kind.is_empty() {
            out.push_str(&format!(" — {}{}", meta.kind, if meta.unit.is_empty() { String::new() } else { format!(", in {}", meta.unit) }));
        }
        if !meta.help.is_empty() {
            out.push_str(&format!(": {}", meta.help));
        }
        out.push_str(&format!("\n{} series in the last hour{}.", sets.len(), if sets.len() >= prom::MAX_SERIES { " or more" } else { "" }));
        if values.is_empty() {
            out.push_str(" No labels.");
        } else {
            out.push_str(" Its labels, with some of their values:\n");
            for (k, vs) in values {
                let shown: Vec<String> = vs.iter().take(6).map(|v| format!("\"{v}\"")).collect();
                let more = vs.len().saturating_sub(6);
                out.push_str(&format!("- {k}: {}{}\n", shown.join(", "), if more > 0 { format!(" and {more} more") } else { String::new() }));
            }
        }
        if meta.kind == "histogram" || text.ends_with("_bucket") {
            let base = text.trim_end_matches("_bucket");
            out.push_str(&format!(
                "\nA histogram: its 95th percentile by a label is histogram_quantile(0.95, sum by (le, <label>) (rate({base}_bucket[5m])))."
            ));
        }
        return Ok(out.trim_end().to_string());
    }
    let words: Vec<String> = text
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect();
    let mut scored: Vec<(usize, &String)> = names
        .iter()
        .filter_map(|n| {
            let l = n.to_ascii_lowercase();
            let hits = words.iter().filter(|w| l.contains(w.as_str())).count();
            (hits > 0).then_some((hits, n))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.len().cmp(&b.1.len())).then(a.1.cmp(b.1)));
    // A histogram's three series are one metric to look for.
    let mut shown: Vec<String> = Vec::new();
    for (_, n) in &scored {
        let family = n.strip_suffix("_bucket").or_else(|| n.strip_suffix("_sum")).or_else(|| n.strip_suffix("_count"));
        let is_hist = family.is_some_and(|f| names.iter().any(|m| m == &format!("{f}_bucket")));
        // A summary is its name (with a `quantile` label) beside `_sum` and `_count`.
        let is_summary = |f: &str| names.iter().any(|m| m == f) && names.iter().any(|m| m == &format!("{f}_sum"));
        let label = match family {
            Some(f) if is_hist => format!("{f}_bucket (a histogram; also {f}_sum and {f}_count)"),
            Some(f) if is_summary(f) => format!("{f} (a summary, by quantile; also {f}_sum and {f}_count)"),
            None if is_summary(n) => format!("{n} (a summary, by quantile; also {n}_sum and {n}_count)"),
            _ => n.to_string(),
        };
        if !shown.contains(&label) {
            shown.push(label);
        }
        if shown.len() >= 20 {
            break;
        }
    }
    if shown.is_empty() {
        return Ok(format!(
            "No metric names contain {}. {} metrics exist; try other words (latency, duration, seconds, requests, errors, cpu, memory).",
            words.iter().map(|w| format!("“{w}”")).collect::<Vec<_>>().join(" or "),
            names.len()
        ));
    }
    let mut out = format!("{} of {} metric names match, best first:\n", scored.len(), names.len());
    for (i, name) in shown.iter().enumerate() {
        let bare = name.split(' ').next().unwrap_or(name);
        let help = if i < 8 { conn.metadata(bare).await.ok().filter(|m| !m.help.is_empty()).map(|m| format!(" — {}", m.help)) } else { None };
        out.push_str(&format!("- {name}{}\n", help.unwrap_or_default()));
    }
    out.push_str("Give one exact name to see its labels.");
    Ok(out)
}
