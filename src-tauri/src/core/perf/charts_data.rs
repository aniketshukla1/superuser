//! charts' data

use super::*;

/// One field of CSV; a header a spreadsheet would run as a formula is written as text (label
/// values are other people's words).
pub(super) fn csv_field(value: &str) -> String {
    let v = if value.starts_with(['=', '+', '-', '@']) { format!("'{value}") } else { value.to_string() };
    if v.contains([',', '"', '\n', '\r']) || v != v.trim() {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

/// A value for the CSV: up to six significant figures, and nothing for NaN or ±Inf, which a
/// chart can't place.
pub(super) fn csv_num(x: f64) -> String {
    if !x.is_finite() {
        return String::new();
    }
    let s = format!("{x:.6}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

/// Series as a CSV `chart` can read: one row a moment, one column a series, the moment in this
/// machine's time zone.
pub fn to_csv(columns: &[(String, &Series)]) -> String {
    let mut times: Vec<i64> = columns.iter().flat_map(|(_, s)| s.points.iter().map(|p| p.0 as i64)).collect();
    times.sort_unstable();
    times.dedup();
    let long = times.first().zip(times.last()).is_some_and(|(a, b)| b - a > 86_400);
    let at: Vec<BTreeMap<i64, f64>> = columns.iter().map(|(_, s)| s.points.iter().map(|p| (p.0 as i64, p.1)).collect()).collect();
    let mut out = String::from("time");
    for (name, _) in columns {
        out.push(',');
        out.push_str(&csv_field(name));
    }
    out.push('\n');
    for t in times {
        let when = Local.timestamp_opt(t, 0).single().map(|d| d.format(if long { "%m-%d %H:%M" } else { "%H:%M" }).to_string()).unwrap_or_default();
        out.push_str(&when);
        for m in &at {
            out.push(',');
            out.push_str(&m.get(&t).map(|v| csv_num(*v)).unwrap_or_default());
        }
        out.push('\n');
    }
    out
}

/// Keep `csv` in the conversation's folder under `metrics/`, returning the path as a chart
/// cites it. `None` when the folder can't be written, which costs the chart and nothing else.
pub(super) fn keep_csv(root: &Path, kind: &str, query: &str, now: DateTime<Utc>, csv: &str) -> Option<String> {
    let slug: String = query
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|w| w.len() > 2 && !matches!(*w, "sum" | "by" | "rate" | "avg" | "max" | "min" | "irate" | "increase"))
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect();
    let rel = format!("metrics/{kind}-{}{}.csv", if slug.is_empty() { String::new() } else { format!("{slug}-") }, now.format("%H%M%S"));
    std::fs::create_dir_all(root.join("metrics")).ok()?;
    std::fs::write(root.join(&rel), csv).ok()?;
    Some(rel)
}

pub(super) fn chart_hint(rel: &str, columns: &[String], note: &str) -> String {
    let series: Vec<Value> = columns.iter().map(|c| serde_json::json!({ "field": c })).collect();
    let spec = serde_json::json!({ "type": "line", "title": "", "note": note, "x": { "field": "time" }, "series": series });
    format!(
        "\nThe series are kept in {rel}. To show them, call `chart` with `source: \"{rel}\"` and no `data` — it reads the rows \
         itself — and a spec like {spec} (give it a title, and say in `note` what it shows)."
    )
}
