use super::*;

/// The work board. Every write records the acting run's session id as the actor —
/// the platform assigns it (`abc`, `abc-sub1`, `abc-sub2`), so a model can neither
/// choose nor forge it, which is what makes the no-self-review rule mean anything.
pub(super) fn dispatch_board(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> Option<String> {
    use crate::core::board::{Board, ItemState};

    if !name.starts_with("board_") {
        return None;
    }
    let b = Board::open(&ctx.data_dir, &ctx.root);
    let actor = ctx.session_id.as_str();
    let id = || args["id"].as_str().unwrap_or("").trim().to_string();
    let said = |r: Result<(), String>, ok: String| Some(match r {
        Ok(()) => ok,
        Err(e) => format!("error: {e}"),
    });

    match name {
        "board_add" => Some(
            match b.add(
                actor,
                args["title"].as_str().unwrap_or(""),
                args["detail"].as_str().unwrap_or(""),
                args["acceptance"].as_str().unwrap_or(""),
            ) {
                Ok(id) => format!("Added `{id}` to the board."),
                Err(e) => format!("error: {e}"),
            },
        ),
        "board_list" => {
            let filter = args["state"].as_str().and_then(ItemState::parse);
            Some(b.render(filter))
        }
        "board_assign" => {
            let to = args["to"].as_str().unwrap_or("").trim().to_string();
            if to.is_empty() {
                return Some("error: say who is taking it".into());
            }
            said(b.assign(actor, &id(), &to), format!("`{}` assigned to {to}.", id()))
        }
        "board_move" => {
            let Some(state) = args["state"].as_str().and_then(ItemState::parse) else {
                return Some("error: state must be open | in_progress | blocked | review | done | canceled".into());
            };
            said(b.move_to(actor, &id(), state), format!("`{}` is now {}.", id(), state.label()))
        }
        "board_note" => said(
            b.note(actor, &id(), args["text"].as_str().unwrap_or("")),
            format!("Noted on `{}`.", id()),
        ),
        "board_review" => {
            let pass = args["pass"].as_bool().unwrap_or(false);
            said(
                b.review(actor, &id(), pass, args["evidence"].as_str().unwrap_or("")),
                format!("Recorded: `{}` {}.", id(), if pass { "passes" } else { "does not pass" }),
            )
        }
        _ => None,
    }
}

/// The automations a person already has: listing them, changing one, cancelling one.
///
/// Every reply is in words — the schedule spelled out, when it last ran, what happened —
/// because these are read by a model that has to tell a person about them, and `0 9 * * 1-5`
/// is not something to read aloud.
pub(super) fn automations(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    use crate::core::automation::{AutomationStore, Schedule};

    let mut store = AutomationStore::load(&ctx.data_dir);
    let when = |ts: Option<f64>| {
        ts.and_then(|t| chrono::DateTime::from_timestamp(t as i64, 0))
            .map(|d| d.with_timezone(&chrono::Local).format("%a %-d %b, %H:%M").to_string())
            .unwrap_or_else(|| "—".into())
    };
    match name {
        "list_automations" => {
            let tasks = store.list();
            if tasks.is_empty() {
                return "There are no automations set up.".into();
            }
            let mut out = String::new();
            for t in tasks {
                out.push_str(&format!(
                    "- **{}** — {} · {} · last ran {} ({}) · next {} · id `{}`\n",
                    t.title,
                    t.human_schedule(),
                    if t.enabled { "on" } else { "switched off" },
                    when(t.last_run),
                    t.last_status.clone().unwrap_or_else(|| "never".into()),
                    if t.enabled { when(t.next_run) } else { "when it is switched back on".into() },
                    t.id
                ));
            }
            out
        }
        "update_automation" => {
            let id = args["id"].as_str().unwrap_or("").trim().to_string();
            let Some(before) = store.get(&id).cloned() else {
                return format!("error: there's no automation with the id `{id}`. Call list_automations to see them.");
            };
            let mut task = before.clone();
            let mut changed: Vec<String> = Vec::new();
            if let Some(title) = args["title"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                task.title = title.to_string();
                changed.push("its name".into());
            }
            if let Some(instructions) = args["instructions"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                task.instructions = instructions.to_string();
                changed.push("what it does".into());
            }
            if let Some(cron) = args["cron"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                task.schedule = Schedule {
                    kind: "cron".into(),
                    cron: Some(cron.to_string()),
                    fire_at: None,
                    timezone: args["timezone"].as_str().unwrap_or(&before.schedule.timezone).trim().to_string(),
                };
                changed.push("when it runs".into());
            }
            if let Some(on) = args["enabled"].as_bool() {
                task.enabled = on;
                changed.push(if on { "switched it on".into() } else { "switched it off".to_string() });
            }
            if changed.is_empty() {
                return "error: say what to change — the name, what it does, when it runs, or whether it's on.".into();
            }
            if let Some(problem) = crate::core::automation::schedule_problem(&task.schedule) {
                return format!("Not changed — {problem}");
            }
            if task.enabled && !before.enabled {
                if let Err(e) = store.set_enabled(&id, true) {
                    return format!("Not changed — {e}");
                }
            }
            match store.replace(&id, task) {
                Ok(after) => format!(
                    "Changed {} — “{}”, {}. Next run {}.",
                    changed.join(" and "),
                    after.title,
                    after.human_schedule(),
                    if after.enabled { when(after.next_run) } else { "when it is switched back on".into() }
                ),
                Err(e) => format!("error: {e}"),
            }
        }
        _ => {
            let id = args["id"].as_str().unwrap_or("").trim().to_string();
            let Some(task) = store.get(&id).cloned() else {
                return format!("error: there's no automation with the id `{id}`. Call list_automations to see them.");
            };
            if !args["confirm"].as_bool().unwrap_or(false) {
                return format!(
                    "Not deleted. Deleting “{}” also removes its run history, and switching it off keeps both. \
                     If the person really means delete, call this again with confirm: true.",
                    task.title
                );
            }
            if store.delete(&id) {
                format!("Deleted “{}” and its run history.", task.title)
            } else {
                format!("error: couldn't delete `{id}`.")
            }
        }
    }
}
