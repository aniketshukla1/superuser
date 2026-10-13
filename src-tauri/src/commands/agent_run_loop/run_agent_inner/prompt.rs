use super::*;

/// The system prompt's identity half: role, standing instructions, memory, what the
/// user is looking at, the gates, skills, connectors and who it may delegate to.
/// Returns the stable text and the volatile tail.
#[allow(clippy::too_many_arguments)]
pub(super) fn identity_prompt(
    state: &AppState,
    settings: &crate::core::settings::Settings,
    sub: &Option<crate::core::subagent::SubagentType>,
    manifest: &crate::core::personas::PersonaManifest,
    sid: &String,
    depth: u8,
    history: &Option<Vec<HistMsg>>,
    message: &String,
    specs: &mut Vec<crate::core::agent::ToolSpec>,
    skills_text: &str,
    mcp: &McpBundle,
    persona: &String,
    underpowered: bool,
) -> (String, String) {
    let mut system = match &sub {
        Some(t) => t.instructions.clone(),
        None => manifest.system_prompt.clone(),
    };
    // The user's standing instructions, straight after the role they sit beside — with
    // the sentence that says which wins. Given to subagents too: "never push to main"
    // is exactly the kind of rule a delegate doing the work needs to have heard, and
    // the precedence sentence keeps a type's own instructions in charge of what it does.
    // Empty when the user wrote none, so nothing changes for anyone who hasn't.
    system.push_str(&settings.standing_block());
    // Core memory: what past conversations established, distilled by Deep Dream.
    // Costs nothing until there is something to say.
    system.push_str(&crate::core::journal::Journal::new(&state.memory.vault_path).system_block());
    // How the user wants to be worked with. The assistant loop had none of this —
    // the profile only ever reached the old one-shot chat path, so the surface the
    // product is actually used through was the one that didn't know your name.
    //
    // Frozen per session: the learner writes to the profile while a conversation is
    // still going, and re-rendering per turn would rewrite the system prompt under
    // the provider's prompt cache on every turn to reflect something that can
    // perfectly well wait for the next one.
    //
    // Not given to subagents, for the same reason they don't get the persona: a
    // delegate is producing an intermediate result for the agent that called it,
    // not talking to the user, so how the user likes their prose is not its
    // business — and its instructions are meant to be exactly its type's.
    if sub.is_none() {
        system.push_str(&crate::core::profile::frozen_block(&state.data_dir, &sid));
    }
    // What the user is looking at. An assistant beside them could see the screen;
    // without this, "change that to fifteen" has no referent and the only options
    // are to ask which file or to guess. Not given to subagents: a delegate is
    // producing a result for the agent that called it, not looking at the panel.
    //
    // VOLATILE, and therefore last. Caching is a prefix match: one byte changes and
    // everything after it is invalidated. This block, the SuperContext brief and the
    // clock all used to sit AHEAD of the gates, the skills, AGENTS.md and the tool
    // descriptions — so the stable 5k of the prompt was re-read on every request
    // because a document had been opened, or because a minute had passed
    // (HARNESS-PLAN §2.8). They are collected here and appended after everything
    // stable instead.
    let mut volatile = String::new();
    if sub.is_none() {
        volatile.push_str(&crate::core::opendocs::prompt_block(&crate::core::opendocs::get(&sid)));
    }
    // SuperContext: sweep memory before the model sees turn 1, so a new thread
    // doesn't start cold. Turn 1 only — by turn 2 the conversation is its own
    // context, and paying for the sweep on every turn would buy nothing.
    //
    // Not for subagents: a delegate is given a task, not the user's vault.
    //
    // It never taints, and that isn't an oversight to be fixed later — it is the
    // design. Only internal-origin memory is inlined; external memory is *named*
    // (source, count, dates, all of it our own metadata) and never quoted. See
    // `core::supercontext` for why the obvious version of this feature would have
    // disabled Step 1.
    if sub.is_none() && depth == 0 && settings.supercontext_enabled && history.as_ref().map(|h| h.is_empty()).unwrap_or(true) {
        let brief = crate::core::supercontext::sweep(&state.memory, &message);
        // Derived from THIS turn's message, so it differs on every request — the
        // single worst thing to have sitting in front of a cache breakpoint.
        volatile.push_str(&crate::core::supercontext::render(&brief));
    }
    // The gates, stated before they have to fire. A refusal mid-run costs a step and
    // arrives with less context than the choice did; saying the rule up front is the
    // cheap half of the same policy. Keyed to the tools this run actually has — see
    // `core::gates::prompt_block`.
    system.push_str(&crate::core::gates::prompt_block(
        specs.iter().any(|s| s.name == "ask_user"),
        specs.iter().any(|s| s.name == "chart"),
        specs.iter().any(|s| s.name == "web_fetch"),
        specs.iter().any(|s| s.name == "read_file"),
        specs.iter().any(|s| s.name == "recall_memory"),
    ));
    // A type with an explicit tool allowlist doesn't get skills — its list is meant
    // to be exactly what it can do. Types that inherit everything keep them.
    if !skills_text.is_empty() && sub.as_ref().map(|t| t.tools.is_none()).unwrap_or(true) {
        system.push_str("\n\n");
        system.push_str(&skills_text);
    } else if sub.is_some() {
        specs.retain(|s| s.name != "load_skill");
    }
    if !mcp.names.is_empty() {
        system.push_str(&mcp.holdings_block(specs.iter().any(|s| matches!(s.name.as_str(), "web_fetch" | "browser_open" | "browser_navigate"))));
    }
    // How its hands work — only when it has them. Stable text, so it sits with the
    // stable blocks ahead of the volatile tail.
    if specs.iter().any(|s| crate::core::agent::is_browser_tool(&s.name)) {
        system.push_str(crate::core::agentbrowser::GUIDANCE);
    }
    // The subagent menu, listed only for a run that can actually delegate — the
    // types are user-extensible, so a static tool description can't name them.
    if sub.is_none() && specs.iter().any(|s| s.name == "spawn_subagent") {
        // Narrowed to the assistant's declared roster, if it has one. A lead staffs
        // a team it was designed around; a generalist sees everything installed.
        let roster = crate::core::personas::PersonaRegistry::load(&state.data_dir)
            .manifest(&persona)
            .map(|m| m.subagents.clone())
            .unwrap_or_default();
        let store = crate::core::subagent::SubagentStore::load(&state.data_dir);
        let menu = store.catalog_text_for(&roster);
        if menu.trim().is_empty() {
            // A roster that names only uninstalled types leaves nothing to delegate
            // to. Say so, rather than shipping an empty heading the model reads as
            // "delegation is available" and then fails at.
            system.push_str(
                "\n\n## Subagent types\n\nNone of the worker types this assistant staffs are installed, \
                 so you cannot delegate right now. Do the work yourself or tell the user what is missing.",
            );
        } else {
            system.push_str(&format!(
                "\n\n## Subagent types\n\nWhen you delegate with spawn_subagent, pick the type that fits:\n{menu}"
            ));
        }
    }
    // A view the model lays out itself, when the person has turned it on. Off by default:
    // sealed or not, "a model wrote the page you are looking at" is a thing to agree to.
    if sub.is_none() && settings.generative_ui {
        specs.push(crate::core::agent::ui_component_spec());
        system.push_str(crate::core::genui::GUIDANCE);
    }
    // Handing work to another assistant — offered only to a run that has somebody to ask
    // and is not already as far from the person as a chain goes. A model shown a tool
    // whose every call would be refused reaches for it and then tells the person it tried,
    // which reads as the app being broken rather than as the cap doing its job.
    if sub.is_none() {
        let reg = crate::core::personas::PersonaRegistry::load(&state.data_dir);
        let roster = crate::core::coworker::may_ask(&reg, &persona);
        if !roster.is_empty() && crate::core::coworker::depth_of(&sid) < crate::core::coworker::MAX_DEPTH {
            specs.push(crate::core::agent::ask_coworker_spec());
            system.push_str(&crate::core::coworker::roster_block(&roster));
        }
    }
    if underpowered {
        system.push_str(
            "\n\n## The model running you\n\nThis assistant's work depends on judgement a small model does not \
             reliably have, and you are running on a local one. Do the work anyway — but say so in your answer, \
             once, plainly: which conclusions you are less sure of, and that a more capable model would judge them \
             better. Do not hedge every sentence, and do not skip the work. A confident report from a model that \
             could not really judge is worse than a report that names its own limits.",
        );
    }
    (system, volatile)
}
