use super::*;

mod seed_brain;
use seed_brain::*;

    pub(super) async fn run_one(
        http: &crate::core::egress::Client,
        tok: &str,
        settings: &Value,
        model: &str,
        task: &Task,
        sid: &str,
        marker: &str,
    ) -> TurnRecord {
        let mut rec = TurnRecord::default();
        rec.marker = marker.to_string();
        let Some(dir) = work_dir(settings, sid) else {
            rec.error = Some("no data dir in /v1/settings".into());
            return rec;
        };
        // A fresh folder per task, so one task's leftovers cannot make another pass.
        let _ = std::fs::remove_dir_all(&dir);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            rec.error = Some(format!("fixture dir: {e}"));
            return rec;
        }
        for (name, body) in task.fixture {
            let body = body.replace(MARKER_SLOT, marker);
            // Filler last: it is large, and expanding it before the marker swap would
            // mean scanning ~56k of log lines for a token that is never in them.
            let body = if body.contains(FILLER_SLOT) { body.replace(FILLER_SLOT, &filler()) } else { body };
            if let Err(e) = std::fs::write(dir.join(name), body) {
                rec.error = Some(format!("fixture {name}: {e}"));
                return rec;
            }
        }

        // The brain half — see `seed_brain`.
        if let Err(e) = seed_brain(settings, task, marker) {
            rec.error = Some(e);
            return rec;
        }

        // Boundaries and standing instructions, under the vault's rule and for the
        // vault's reason: applying either replaces the instance's own. On a scratch
        // instance every task gets its own — none, for most — so a rule written for one
        // task can never refuse the next one's work.
        let configures = task.boundaries.is_some() || !task.standing.is_empty();
        match (scratch_vault(settings).is_some(), configures) {
            (true, _) => {
                if let Err(e) = apply_config(http, tok, task, marker).await {
                    rec.error = Some(e);
                    return rec;
                }
            }
            (false, false) => {}
            (false, true) => {
                rec.error = Some(
                    "this task sets boundaries or standing instructions, so it needs a scratch instance: run the app and this driver with the same SUPERUSER_DATA_DIR".into(),
                );
                return rec;
            }
        }

        // The assistants this task needs beside the one under test, and who it may ask.
        // Installing an assistant writes into the app's own data directory and granting one
        // changes what every later turn may do, so both are scratch-only — and on scratch
        // both are applied for EVERY task, so an empty list clears what the last one set.
        let colleagues = task.scratch_only || !task.personas.is_empty() || !task.asks.is_empty();
        match (scratch_vault(settings).is_some(), colleagues) {
            (true, _) => {
                if let Err(e) = seed_personas(http, tok, settings, task, marker).await {
                    rec.error = Some(e);
                    return rec;
                }
            }
            (false, false) => {}
            (false, true) => {
                rec.error = Some(
                    "this task changes the app's own state (assistants, grants, or something parked for approval), so it needs a scratch instance: run the app and this driver with the same SUPERUSER_DATA_DIR".into(),
                );
                return rec;
            }
        }

        // The browser, under the same rule. A browser task switches it on, gives the
        // assistant one, and stands in for the "Always allow" a person would press on the
        // first card — for this conversation and the fixture host only. Every OTHER task on
        // a scratch instance gets it switched off, so its tool list is the one every earlier
        // row on the scoreboard was measured with.
        let browses = !task.site.is_empty();
        let mut prompt = task.prompt.replace(MARKER_SLOT, marker);
        let _site = match (scratch_vault(settings).is_some(), browses) {
            (true, true) => {
                let site = match serve_site(task.site, marker.to_string()).await {
                    Ok(s) => s,
                    Err(e) => {
                        rec.error = Some(format!("fixture site: {e}"));
                        return rec;
                    }
                };
                if let Err(e) = prepare_browser(http, tok, sid, &site.base).await {
                    rec.error = Some(e);
                    return rec;
                }
                prompt = prompt.replace(SITE_SLOT, &site.base);
                Some(site)
            }
            (true, false) => {
                if let Err(e) = post_ok(http, tok, "/v1/settings/computer", json!({ "browser_enabled": false })).await {
                    rec.error = Some(e);
                    return rec;
                }
                None
            }
            (false, true) => {
                rec.error = Some(
                    "this is a browser task, so it needs a scratch instance: run the app and this driver with the same SUPERUSER_DATA_DIR".into(),
                );
                return rec;
            }
            (false, false) => {
                // Not an instance the driver may change, so the browser
                // can't be switched off as it is on scratch. Read now rather than at the start of
                // the sweep: the person whose app this is may flip it while the sweep runs.
                let fresh = get_value(http, tok, "/v1/settings").await;
                let now = fresh.as_ref().unwrap_or(settings);
                let assistant = if now["browser_enabled"].as_bool().unwrap_or(false) {
                    get_value(http, tok, "/v1/personas/assistant").await
                } else {
                    None
                };
                rec.not_comparable = browser_note(now, assistant.as_ref());
                None
            }
        };

        // A second early: the session id is reused across runs, and the trail's clock
        // and this one are the same clock read a moment apart.
        let began_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
            - 1.0;
        let began = std::time::Instant::now();
        let resp = http
            .post(api("/v1/chat/completions"))
            .header("x-superuser-token", tok)
            .timeout(std::time::Duration::from_secs(deadline_secs()))
            .json(&json!({
                "model": "assistant",
                "messages": [{ "role": "user", "content": prompt }],
                "superuser": { "model": model, "session_id": sid }
            }))
            .send()
            .await;
        let body: Value = match resp {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
            Ok(r) => {
                let st = r.status();
                rec.error = Some(format!("{st}: {}", r.text().await.unwrap_or_default().chars().take(160).collect::<String>()));
                rec.seconds = began.elapsed().as_secs_f64();
                rec.files = read_tree(&dir);
                return rec;
            }
            Err(e) => {
                // The app was never reached, or not in time. Distinct from an error
                // it returned: a 6-step task on a 12B local model can legitimately
                // outlast any deadline, and scoring that as the model's failure
                // reports the driver's timeout as the model's ability.
                rec.unreachable = Some(format!("no answer: {e}"));
                return rec;
            }
        };

        rec.answer = body["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        rec.calls = body["superuser"]["trace"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| Call {
                        name: c["name"].as_str().unwrap_or_default().into(),
                        args: c["args"].as_str().unwrap_or_default().into(),
                        result: c["result"].as_str().unwrap_or_default().into(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let m = &body["superuser"]["measure"];
        rec.steps = m["steps"].as_u64().unwrap_or(0) as usize;
        rec.seconds = m["seconds"].as_f64().unwrap_or_else(|| began.elapsed().as_secs_f64());
        rec.cost_usd = m["cost_usd"].as_f64();
        rec.input_tokens = body["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
        rec.output_tokens = body["usage"]["completion_tokens"].as_u64().unwrap_or(0);
        rec.files = read_tree(&dir);
        rec.trail = trail(http, tok, sid, began_at).await;
        rec
    }
