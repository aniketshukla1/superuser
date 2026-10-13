use super::super::*;

pub(in crate::core::perf) fn instant_text(conn: &Conn, q: &str, at: DateTime<Utc>, answer: Answer, w: &[String]) -> String {
    let head = format!("`{q}` at {}", local(at));
    let body = match answer {
        Answer::Scalar(_, v) => format!("{head}: {}", fmt_num(v)),
        Answer::Text(_, s) => format!("{head}: “{s}”"),
        Answer::Series(mut series, total) => {
            if series.is_empty() {
                format!("{head}: no series. The metric may not exist, or nothing matched the labels — metrics_find shows what exists.")
            } else {
                series.sort_by(|a, b| {
                    let v = |s: &Series| s.points.first().map(|p| p.1).filter(|x| !x.is_nan()).unwrap_or(f64::NEG_INFINITY);
                    v(b).partial_cmp(&v(a)).unwrap_or(std::cmp::Ordering::Equal)
                });
                let value = |s: &Series| s.points.first().map(|p| p.1).unwrap_or(f64::NAN);
                let (zero, rest): (Vec<&Series>, Vec<&Series>) = series.iter().partition(|s| value(s) == 0.0);
                let mut out = format!("{head}: {total} series, highest first:\n");
                for s in rest.iter().take(SHOWN * 2) {
                    out.push_str(&format!("- {} = {}\n", prom::fmt_labels(&s.labels), fmt_num(value(s))));
                }
                if rest.len() > SHOWN * 2 {
                    out.push_str(&format!("…and {} more.\n", rest.len() - SHOWN * 2));
                }
                // A `by (handler)` over a server lists every route it has; the idle ones are
                // one fact, not forty lines.
                if !zero.is_empty() {
                    let names: Vec<String> = zero.iter().take(6).map(|s| prom::fmt_labels(&s.labels)).collect();
                    out.push_str(&format!(
                        "{} at 0: {}{}\n",
                        zero.len(),
                        names.join(", "),
                        if zero.len() > 6 { format!(" and {} more", zero.len() - 6) } else { String::new() }
                    ));
                }
                if total > series.len() {
                    out.push_str(&format!("Only the first {} of {total} were read.\n", series.len()));
                }
                out.trim_end().to_string()
            }
        }
    };
    format!("{body}{}{}", link_line(conn, q, at.timestamp() as f64, 3_600), warn_line(w))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::core::perf) fn range_text(
    conn: &Conn,
    q: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    step: i64,
    answer: Answer,
    w: &[String],
    root: &Path,
    now: DateTime<Utc>,
) -> Result<Out, String> {
    let Answer::Series(series, total) = answer else {
        return Err("a range query answers with series; this one didn't".into());
    };
    let head = format!("`{q}` from {} to {} (a point every {})", local(start), local(end), prom_duration(step));
    if series.is_empty() {
        return Ok(Out { text: format!("{head}: no series — nothing matched in that time."), files: vec![] });
    }
    let mut rows: Vec<(&Series, Stats)> =
        series.iter().filter_map(|s| stats(&s.points.iter().map(|p| p.1).collect::<Vec<_>>()).map(|st| (s, st))).collect();
    rows.sort_by(|a, b| b.1.mean.partial_cmp(&a.1.mean).unwrap_or(std::cmp::Ordering::Equal));
    let labels: Vec<&BTreeMap<String, String>> = rows.iter().map(|(s, _)| &s.labels).collect();
    let short = prom::short_names(&labels);
    let mut out = format!("{head}: {total} series{}:\n", if rows.len() > SHOWN * 2 { format!(", the {} highest on average", SHOWN * 2) } else { String::new() });
    for (i, (_, st)) in rows.iter().enumerate().take(SHOWN * 2) {
        out.push_str(&format!(
            "- {} — average {}, min {}, max {}, last {}\n",
            short[i],
            fmt_num(st.mean),
            fmt_num(st.min),
            fmt_num(st.max),
            fmt_num(st.last)
        ));
    }
    let span = (end - start).num_seconds();
    out.push_str(link_line(conn, q, end.timestamp() as f64, span).trim_start_matches('\n'));
    let columns: Vec<(String, &Series)> = if rows.len() == 1 {
        vec![("value".to_string(), rows[0].0)]
    } else {
        rows.iter().take(CHARTED).enumerate().map(|(i, (s, _))| (short[i].clone(), *s)).collect()
    };
    let mut files = vec![];
    if let Some(rel) = keep_csv(root, "range", q, now, &to_csv(&columns)) {
        out.push_str(&chart_hint(&rel, &columns.iter().map(|c| c.0.clone()).collect::<Vec<_>>(), ""));
        files.push(rel);
    }
    out.push_str(&warn_line(w));
    Ok(Out { text: out.trim_end().to_string(), files })
}
