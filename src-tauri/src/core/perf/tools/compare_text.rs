use super::super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::core::perf) fn compare_text(
    conn: &Conn,
    q: &str,
    start: DateTime<Utc>,
    change: DateTime<Utc>,
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
    let (before_len, after_len) = ((change - start).num_seconds(), (end - change).num_seconds());
    let mut out = format!(
        "Compared `{q}` in the {} before {} and the {} after it (a point every {}). {total} series.",
        prom_duration(before_len),
        local(change),
        prom_duration(after_len),
        prom_duration(step)
    );
    if series.is_empty() {
        out.push_str(" Nothing matched in that time — metrics_find shows what exists.");
        return Ok(Out { text: out, files: vec![] });
    }
    if total > series.len() {
        out.push_str(&format!(" Only the first {} were read; narrow the query with a label to see the rest.", series.len()));
    }
    let cut = change.timestamp() as f64;
    let labels: Vec<&BTreeMap<String, String>> = series.iter().map(|s| &s.labels).collect();
    let short = prom::short_names(&labels);
    let (mut moved, mut appeared, mut vanished): (Vec<(usize, Shift)>, Vec<(usize, Stats)>, Vec<(usize, Stats)>) = (vec![], vec![], vec![]);
    for (i, s) in series.iter().enumerate() {
        let before: Vec<f64> = s.points.iter().filter(|p| p.0 < cut).map(|p| p.1).collect();
        let after: Vec<f64> = s.points.iter().filter(|p| p.0 >= cut).map(|p| p.1).collect();
        match (stats(&before), stats(&after)) {
            (Some(_), Some(_)) => moved.extend(shift(&before, &after).map(|sh| (i, sh))),
            (None, Some(a)) => appeared.push((i, a)),
            (Some(b), None) => vanished.push((i, b)),
            (None, None) => {}
        }
    }
    moved.sort_by(|a, b| {
        a.1.verdict.cmp(&b.1.verdict).then(
            b.1.pct.map_or(f64::INFINITY, f64::abs).partial_cmp(&a.1.pct.map_or(f64::INFINITY, f64::abs)).unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    let line = |i: usize, sh: &Shift| {
        let mut l = format!(
            "{} — {}: {} → {} (spread before ±{}",
            short[i],
            fmt_move(sh),
            fmt_num(sh.before.mean),
            fmt_num(sh.after.mean),
            fmt_num(sh.before.sd)
        );
        if sh.verdict != Verdict::TooLittle {
            l.push_str(&format!(
                "; {}% of the points after are {} anything before",
                fmt_num(sh.beyond * 100.0),
                if sh.change >= 0.0 { "above" } else { "below" }
            ));
        }
        l.push(')');
        l
    };
    for (verdict, title) in [(Verdict::Clear, "Clear shifts"), (Verdict::Possible, "Possible shifts")] {
        let group: Vec<&(usize, Shift)> = moved.iter().filter(|m| m.1.verdict == verdict).collect();
        if group.is_empty() {
            continue;
        }
        out.push_str(&format!("\n\n{title} ({}):", group.len()));
        for (n, (i, sh)) in group.iter().enumerate().take(SHOWN) {
            out.push_str(&format!("\n{}. {}", n + 1, line(*i, sh)));
        }
        if group.len() > SHOWN {
            out.push_str(&format!("\n…and {} more.", group.len() - SHOWN));
        }
    }
    let steady: Vec<&(usize, Shift)> = moved.iter().filter(|m| m.1.verdict == Verdict::Steady).collect();
    if steady.len() == 1 {
        out.push_str(&format!("\n\nWithin its usual variation (1): {}.", line(steady[0].0, &steady[0].1)));
    } else if !steady.is_empty() {
        let names: Vec<&str> = steady.iter().take(SHOWN).map(|(i, _)| short[*i].as_str()).collect();
        out.push_str(&format!(
            "\n\nWithin their usual variation ({}): {}{}. The largest of these: {}.",
            steady.len(),
            names.join(", "),
            if steady.len() > SHOWN { format!(" and {} more", steady.len() - SHOWN) } else { String::new() },
            line(steady[0].0, &steady[0].1)
        ));
    }
    let little: Vec<&str> = moved.iter().filter(|m| m.1.verdict == Verdict::TooLittle).map(|(i, _)| short[*i].as_str()).collect();
    if !little.is_empty() {
        out.push_str(&format!("\n\nToo few points to judge ({}): {}.", little.len(), little.iter().take(SHOWN).copied().collect::<Vec<_>>().join(", ")));
    }
    for (group, title) in [(&appeared, "New after the change"), (&vanished, "Gone after the change")] {
        if !group.is_empty() {
            let items: Vec<String> = group.iter().take(SHOWN).map(|(i, st)| format!("{} (average {})", short[*i], fmt_num(st.mean))).collect();
            out.push_str(&format!("\n\n{title} ({}): {}.", group.len(), items.join(", ")));
        }
    }
    if moved.iter().all(|m| m.1.verdict >= Verdict::Steady) && appeared.is_empty() && vanished.is_empty() {
        out.push_str("\n\nNothing here moved beyond its usual variation. The cause may be in a label this query sums away, or in another metric.");
    }
    let link = link_line(conn, q, end.timestamp() as f64, (end - start).num_seconds());
    if !link.is_empty() {
        out.push('\n');
        out.push_str(&link);
    }
    // The chart: the series that moved most, then the rest, up to eight.
    let order: Vec<usize> = moved.iter().map(|m| m.0).chain(appeared.iter().map(|a| a.0)).chain(vanished.iter().map(|v| v.0)).collect();
    let columns: Vec<(String, &Series)> = if series.len() == 1 {
        vec![("value".to_string(), &series[0])]
    } else {
        order.iter().take(CHARTED).map(|&i| (short[i].clone(), &series[i])).collect()
    };
    let mut files = vec![];
    if let Some(rel) = keep_csv(root, "compare", q, now, &to_csv(&columns)) {
        let note = format!("The change was at {}.", change.with_timezone(&Local).format("%H:%M"));
        out.push_str(&chart_hint(&rel, &columns.iter().map(|c| c.0.clone()).collect::<Vec<_>>(), &note));
        files.push(rel);
    }
    out.push_str(&warn_line(w));
    Ok(Out { text: out, files })
}

pub(in crate::core::perf) fn alerts_text(alerts: &[prom::Alert], now: DateTime<Utc>) -> String {
    if alerts.is_empty() {
        return "No alerts are firing or pending.".into();
    }
    let firing = alerts.iter().filter(|a| a.state == "firing").count();
    let mut out = format!("{firing} firing, {} pending:\n", alerts.len() - firing);
    for a in alerts.iter().take(40) {
        let since = DateTime::parse_from_rfc3339(&a.active_at)
            .ok()
            .map(|t| {
                let mins = (now - t.with_timezone(&Utc)).num_minutes().max(0);
                let ago = if mins < 120 { format!("{mins} min") } else if mins < 2_880 { format!("{} h", mins / 60) } else { format!("{} days", mins / 1_440) };
                format!(" since {} ({ago})", local(t.with_timezone(&Utc)))
            })
            .unwrap_or_default();
        let sev = if a.severity.is_empty() { String::new() } else { format!("[{}] ", a.severity) };
        let what = if a.summary.is_empty() { String::new() } else { format!(" — {}", a.summary) };
        let on: Vec<String> = a.labels.iter().filter(|(k, _)| !k.starts_with("__")).map(|(k, v)| format!("{k}=\"{v}\"")).collect();
        out.push_str(&format!(
            "- {sev}{} — {}{since}{what}{}\n",
            a.name,
            a.state,
            if on.is_empty() { String::new() } else { format!(" — {}", on.join(", ")) }
        ));
    }
    if alerts.len() > 40 {
        out.push_str(&format!("…and {} more.\n", alerts.len() - 40));
    }
    out.push_str("Read-only: nothing here silences or changes an alert.");
    out
}
