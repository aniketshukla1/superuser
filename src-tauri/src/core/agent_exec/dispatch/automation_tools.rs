use super::*;

pub(super) async fn automation_tools(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    match name {
        "watch_for" => {
            use crate::core::subconscious::{Subconscious, Watch};
            let question = args["question"].as_str().unwrap_or("").trim().to_string();
            if question.is_empty() {
                return "error: `question` must say what to watch for".into();
            }
            let sources: Vec<String> = args["sources"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            // Fail closed on intent: a watch the model didn't think about is a watch
            // that might act, and `writes` is what decides whether answering it can
            // happen unattended or has to end at the Inbox.
            let writes = args["writes"].as_bool().unwrap_or(true);
            // Recorded by the gateway, before this ran — see `core::gateway`.
            let created = Subconscious::load(&ctx.data_dir).create(Watch::new(question, sources.clone(), writes));
            format!(
                "Watching: \"{}\" — checked against {} whenever they change, not on a clock, so quiet \
                 periods cost nothing.{} The user can see it and turn it off under Automations ▸ Watches.",
                created.question,
                if sources.is_empty() { "everything in memory".to_string() } else { sources.join(", ") },
                if writes {
                    " Anything it wants to do outside this machine will be parked for approval."
                } else {
                    ""
                },
            )
        }
        "schedule_task" => {
            use crate::core::automation::{next_run_after, AutomationStore, Schedule, ScheduledTask};
            let title = args["title"].as_str().unwrap_or("").trim().to_string();
            let instructions = args["instructions"].as_str().unwrap_or("").trim().to_string();
            if instructions.is_empty() {
                return "error: `instructions` must say what to do when it fires".into();
            }
            let cron = args["cron"].as_str().unwrap_or("").trim().to_string();
            let at = args["at"].as_str().unwrap_or("").trim().to_string();
            // The clock the person spoke in. Left out, it is this machine's own.
            let timezone = args["timezone"].as_str().unwrap_or("").trim();
            let timezone = if timezone.is_empty() { "local".to_string() } else { timezone.to_string() };
            let schedule = if !cron.is_empty() {
                Schedule { kind: "cron".into(), cron: Some(cron), fire_at: None, timezone }
            } else if !at.is_empty() {
                // Pinned to the zone it was said in, so it fires at that time there (F-6).
                let at = crate::core::automation::pin_fire_at(&at, &timezone);
                Schedule { kind: "once".into(), cron: None, fire_at: Some(at), timezone }
            } else {
                return "error: give either `cron` (repeating) or `at` (one-off)".into();
            };
            // Validated against *now*, not the epoch: a one-off in the past passes an
            // epoch check and then either fires instantly or never. Measured — a model
            // with no date in its context invented a 2023 timestamp and it was accepted.
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
            if next_run_after(&schedule, now).is_none() {
                return format!(
                    "error: that schedule has no next run. Today is {}. For a one-off, `at` must be in the future; \
                     for a repeat, use five cron fields like `0 9 * * 1-5` (9am Mon–Fri).",
                    chrono::Local::now().format("%A %-d %B %Y, %H:%M"),
                );
            }
            let mut task = ScheduledTask::new(
                if title.is_empty() { instructions.chars().take(50).collect() } else { title },
                instructions,
                schedule,
                ctx.root.to_string_lossy().into_owned(),
                Some(ctx.persona.clone()),
                None,
            );
            // Bound to the conversation it was asked for in, so each run's answer
            // comes back here and the user can follow up on it in context.
            //
            // Checked against the store rather than by naming convention: an
            // unattended run, a subagent and the stateless HTTP endpoint all invent
            // session ids that no conversation will ever open, and pointing a task at
            // one of those means its answers go nowhere with nothing to show for it.
            if !ctx.unattended && crate::core::sessions::SessionStore::new(&ctx.data_dir).get(&ctx.session_id).is_some() {
                task.origin_session = Some(ctx.session_id.clone());
            }
            let created = match AutomationStore::load(&ctx.data_dir).create_checked(task) {
                Ok(t) => t,
                // The floor, the cap and an unreadable schedule all arrive here as
                // sentences the model can act on rather than pass along.
                Err(problem) => return format!("Not scheduled — {problem}"),
            };
            format!(
                "Scheduled \"{}\" — {}. First run: {}. The user can change or cancel it under Automations.",
                created.title,
                created.human_schedule(),
                created
                    .next_run
                    .and_then(|t| chrono::DateTime::from_timestamp(t as i64, 0))
                    .map(|d| d.with_timezone(&chrono::Local).format("%a %d %b, %H:%M").to_string())
                    .unwrap_or_else(|| "unknown".into()),
            )
        }
        "list_automations" | "update_automation" | "delete_automation" => automations(ctx, name, args),
        _ => unreachable!("routed here only for its own tools"),
    }
}
