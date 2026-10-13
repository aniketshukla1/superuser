use super::super::*;

/// Replace an oversized result with an abstract plus the path it was saved to.
/// A thin wrapper so both provider loops and the subagent path share one rule.
pub(in crate::commands) fn offloaded(root: &std::path::Path, label: &str, out: String) -> String {
    crate::core::artifacts::offload(root, label, &out).unwrap_or(out)
}

pub(in crate::commands) async fn run_subagents(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    depth: u8,
    args: &serde_json::Value,
) -> String {
    use crate::core::subagent::SubagentStore;

    // One level only: `spawn_subagent` is on every subagent's blocklist, so this is
    // belt-and-braces against a call that reached here another way.
    if depth > 0 {
        return "subagents can't spawn further subagents — do this step yourself.".into();
    }

    let tasks = parse_subagent_tasks(args);
    if tasks.is_empty() {
        return "error: describe at least one subtask in `tasks`".into();
    }
    let store = SubagentStore::load(&state.data_dir);
    if let Some((_, bad)) = tasks.iter().find(|(_, ty)| store.get(ty).is_none()) {
        return format!("error: unknown subagent type '{bad}'. Available: {}", store.names().join(", "));
    }
    let over = tasks.len().saturating_sub(MAX_PARALLEL_SUBAGENTS);
    let tasks: Vec<_> = tasks.into_iter().take(MAX_PARALLEL_SUBAGENTS).collect();

    let deadline = tokio::time::Instant::now() + SUBAGENT_BUDGET;
    let runs = tasks.iter().enumerate().map(|(i, (task, ty))| {
        let sub_session = format!("{}-sub{}", exec.session_id, i + 1);
        async move {
            let fut = Box::pin(run_agent_inner(
                app,
                state,
                exec.persona.clone(),
                task.clone(),
                Some(sub_session),
                None,
                None,
                depth + 1,
                None,
                Some(ty.clone()),
                None,
            ));
            match tokio::time::timeout_at(deadline, fut).await {
                Ok(Ok(out)) => out.answer,
                Ok(Err(e)) => format!("failed: {e}"),
                Err(_) => "ran out of time and was stopped — its work is incomplete, not empty.".to_string(),
            }
        }
    });

    let results = futures_util::future::join_all(runs).await;
    let mut out = String::new();
    for ((task, ty), answer) in tasks.iter().zip(results) {
        // A subagent runs in its own folder, so anything it wrote is invisible to
        // the parent. Its *answer* is the handoff — and a long one would otherwise
        // land whole in the parent's context, which is what delegating was meant to
        // avoid. Offload it into the parent's outputs/ so the path it comes back as
        // is one the parent can actually read.
        let answer = offloaded(&exec.root, &format!("{ty}-{task}"), answer);
        out.push_str(&format!("## [{ty}] {}\n\n{answer}\n\n", task.chars().take(120).collect::<String>()));
    }
    if over > 0 {
        out.push_str(&format!("({over} further task(s) were not started — at most {MAX_PARALLEL_SUBAGENTS} run at once. Call again for the rest.)\n"));
    }
    out
}
