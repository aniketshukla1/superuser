use super::super::*;

/// The arithmetic, split out so it can be tested with dates somebody chose. Through the
/// handler it would need a Tauri AppHandle for its state, and a streak is exactly the kind
/// of figure where an off-by-one survives every test that only checks invariants.
pub(in crate::core::api_server) fn compute_stats(
    sessions: &[&crate::core::sessions::Session],
    now: chrono::DateTime<chrono::Local>,
) -> Value {
    use chrono::{Local, TimeZone, Timelike};

    let mut per_day: std::collections::BTreeMap<String, u64> = Default::default();
    let mut per_hour = [0u64; 24];
    // Named rather than a 6-tuple: this grew cache counters for HARNESS-PLAN Phase 0,
    // and `e.4 += …` beside `e.5 += …` is how the wrong one gets incremented.
    #[derive(Default)]
    struct ModelRoll {
        sessions: u64,
        messages: u64,
        usage: crate::core::sessions::TokenUsage,
    }
    let mut per_model: std::collections::HashMap<String, ModelRoll> = Default::default();
    let mut per_agent: std::collections::HashMap<String, u64> = Default::default();
    let mut messages: u64 = 0;
    let mut tokens: u64 = 0;
    let mut priced: u64 = 0; // messages that actually carried a figure

    for s in sessions {
        let count = s.messages.len() as u64;
        messages += count;
        let mut roll = crate::core::sessions::TokenUsage::default();
        for m in &s.messages {
            if let Some(u) = &m.usage {
                tokens += u.total();
                priced += 1;
                roll.input += u.input;
                roll.output += u.output;
                roll.cache_read += u.cache_read;
                roll.cache_write += u.cache_write;
            }
        }
        // created_at is 0 on sessions written before it was recorded; updated_at is the
        // fallback, and a session with neither contributes no day rather than a day in 1970.
        let stamp = if s.created_at > 0.0 { s.created_at } else { s.updated_at };
        if stamp > 0.0 {
            if let Some(t) = Local.timestamp_opt(stamp as i64, 0).single() {
                *per_day.entry(t.format("%Y-%m-%d").to_string()).or_default() += 1;
                per_hour[t.hour() as usize] += 1;
            }
        }
        if !s.model.is_empty() {
            let e = per_model.entry(s.model.clone()).or_default();
            e.sessions += 1;
            e.messages += count;
            e.usage.input += roll.input;
            e.usage.output += roll.output;
            e.usage.cache_read += roll.cache_read;
            e.usage.cache_write += roll.cache_write;
        }
        if !s.agent.is_empty() {
            *per_agent.entry(s.agent.clone()).or_default() += 1;
        }
    }

    // Streaks over CONSECUTIVE CALENDAR DAYS, walking the day keys rather than counting
    // entries — two sessions on one day are one day of work.
    let day_keys: Vec<chrono::NaiveDate> = per_day
        .keys()
        .filter_map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();
    let mut longest = 0u64;
    let mut run = 0u64;
    let mut prev: Option<chrono::NaiveDate> = None;
    for d in &day_keys {
        run = match prev {
            Some(p) if (*d - p).num_days() == 1 => run + 1,
            _ => 1,
        };
        longest = longest.max(run);
        prev = Some(*d);
    }
    // Current streak counts back from today, and from yesterday too — a streak should not
    // read as broken at 00:01 before you have had a chance to work.
    let today = now.date_naive();
    let have: std::collections::HashSet<_> = day_keys.iter().copied().collect();
    let mut current = 0u64;
    let mut cursor = if have.contains(&today) {
        today
    } else {
        today - chrono::Duration::days(1)
    };
    while have.contains(&cursor) {
        current += 1;
        cursor -= chrono::Duration::days(1);
    }

    let peak_hour = per_hour
        .iter()
        .enumerate()
        .max_by_key(|(_, n)| **n)
        .filter(|(_, n)| **n > 0)
        .map(|(h, _)| h as u64);

    let mut models: Vec<Value> = per_model
        .iter()
        .map(|(m, r)| {
            let u = &r.usage;
            json!({
                "model": m,
                "sessions": r.sessions,
                "messages": r.messages,
                "input": u.input,
                "output": u.output,
                "cache_read": u.cache_read,
                "cache_write": u.cache_write,
                "cache_hit_rate": u.cache_hit_rate(),
                // Absent, not zero, when we have no rate for this model id — the UI
                // must be able to say "unknown" rather than claim it was free.
                "cost_usd": crate::core::pricing::cost_of(m, u),
                "tokens": u.total(),
            })
        })
        .collect();
    // By tokens when any are known, else by sessions — ordering a token breakdown by
    // session count puts the biggest bar somewhere other than the top.
    models.sort_by(|a, b| {
        let key = |v: &Value| (v["tokens"].as_u64().unwrap_or(0), v["sessions"].as_u64().unwrap_or(0));
        key(b).cmp(&key(a))
    });
    // Spend across every model we have a rate for. Models we do not recognise
    // contribute nothing here and appear in `models[]` with a null `cost_usd`, so a
    // partial total can never be read as a complete one — the same reason
    // `priced_messages` sits beside `tokens`. `None` when we recognised nothing at all.
    let known_costs: Vec<f64> = models.iter().filter_map(|m| m["cost_usd"].as_f64()).collect();
    let total_cost: Option<f64> = (!known_costs.is_empty()).then(|| known_costs.iter().sum());

    let mut agents: Vec<Value> = per_agent
        .iter()
        .map(|(a, n)| json!({ "agent": a, "sessions": n }))
        .collect();
    agents.sort_by(|a, b| b["sessions"].as_u64().cmp(&a["sessions"].as_u64()));

    json!({
        "sessions": sessions.len(),
        "messages": messages,
        "tokens": tokens,
        // How many messages the total is actually built from. Providers report usage
        // inconsistently and everything written before usage existed has none, so a bare
        // total would quietly present a partial count as a complete one.
        "priced_messages": priced,
        "cost_usd": total_cost,
        "active_days": per_day.len(),
        "current_streak": current,
        "longest_streak": longest,
        "peak_hour": peak_hour,
        "favorite_model": models.first().map(|m| m["model"].clone()),
        "favorite_agent": agents.first().map(|a| a["agent"].clone()),
        "models": models,
        "agents": agents,
        "days": per_day.iter().map(|(d, n)| json!({ "date": d, "count": n })).collect::<Vec<_>>(),
    })
}

pub(in crate::core::api_server) async fn session_messages(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    let msgs: Vec<crate::core::sessions::Message> = store(&ctx)
        .get(&id)
        .map(|s| s.messages.iter().map(|m| m.without_wire()).collect())
        .unwrap_or_default();
    // Wrapped, like sessions/artifacts — the GUI reads `.messages`.
    Json(json!({ "messages": msgs }))
}

pub(in crate::core::api_server) async fn session_patch(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let st = store(&ctx);
    let mut ok = false;
    if let Some(title) = body["title"].as_str() {
        ok |= st.rename(&id, title);
    }
    let pinned = body["pinned"].as_bool();
    let archived = body["archived"].as_bool();
    if pinned.is_some() || archived.is_some() {
        ok |= st.set_flags(&id, pinned, archived);
    }
    Json(json!({ "ok": ok }))
}

pub(in crate::core::api_server) async fn session_delete(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    Json(json!({ "ok": store(&ctx).delete(&id) }))
}
