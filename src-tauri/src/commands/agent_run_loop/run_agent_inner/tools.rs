use super::*;

/// What this turn may call, and how it is offered to this model.
pub(super) struct Toolset {
    pub(super) shape: crate::core::loopshape::LoopShape,
    pub(super) specs: Vec<crate::core::agent::ToolSpec>,
    pub(super) held: Vec<crate::core::agent::ToolSpec>,
    pub(super) skills: crate::core::skills::SkillStore,
    pub(super) skills_text: String,
    pub(super) mcp: McpBundle,
    pub(super) sub: Option<crate::core::subagent::SubagentType>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn choose_tools(
    state: &AppState,
    agent: &crate::core::agent::Agent,
    persona: &String,
    settings: &crate::core::settings::Settings,
    profile: &crate::core::model_profile::ModelProfile,
    http: &crate::core::egress::Client,
    working_dir: &std::path::PathBuf,
    depth: u8,
    subagent_type: &Option<String>,
    message: &String,
) -> Toolset {
    // ---- Loop shape ------------------------------------------------------------
    // H6: 53 tool schemas, 24 steps and a 9k prompt went out identically to a 3B
    // local model and a frontier one. Resolved here, ahead of the tool list and the
    // prompt, because those are the two things it changes.
    let shape = crate::core::loopshape::LoopShape::for_model(&profile, &settings);

    // Tools available in this context (+ load_skill when skills exist).
    let ctx = crate::core::agent::AgentContext { workspace: Some(working_dir.clone()), roots: vec![working_dir.clone()], has_executor: true, has_todo: true };
    let mut specs = agent.tools(&ctx);
    let skills = crate::core::skills::SkillStore::load(&state.data_dir);
    // Narrowed to what this assistant was given. An assistant declaring none gets
    // everything installed, which is what they all did before and stays right for a
    // generalist — but the Security Assistant offering `ui-component` is noise, and a
    // model reading a catalogue of twenty-one reaches for the wrong one eventually.
    let allowed_skills = crate::core::personas::PersonaRegistry::load(&state.data_dir)
        .manifest(&persona)
        .map(|m| m.skills.clone())
        .unwrap_or_default();
    // …and narrowed again by tier: the catalogue is a menu of twenty-one one-line
    // descriptions, and a small model that picks the wrong one has spent a step
    // loading it. Same argument as the tool budget, same place it is decided.
    let skills_text = if shape.skills_in_prompt {
        skills.catalog_text_for(&allowed_skills)
    } else {
        String::new()
    };
    if !skills_text.is_empty() {
        specs.push(crate::core::skills::load_skill_tool());
    }
    // C4, only when the user has turned it on. See `agent::plan_tool_spec`.
    if state.settings.lock().unwrap().plan_tool_enabled {
        specs.push(crate::core::agent::plan_tool_spec());
    }

    // Connected apps (MCP). These were only ever attached to the Coder loop, so a
    // assistant asked to "file that as a Linear issue" had no way to do it — it could
    // only read what the connector had already synced into memory. Their tool defs
    // are merged into the wire format below rather than into `specs`, because they
    // arrive as provider-shaped JSON, not catalog entries.
    // Subagents are deliberately excluded: acting inside the user's accounts in the
    // main agent's name is outside the scope of one delegated subtask, and it also
    // saves re-opening every connector session per subagent.
    let mcp = if depth == 0 { gather_mcp(&http, &state.data_dir).await } else { McpBundle::default() };

    // A subagent runs under its *type's* prompt and toolset, not the persona's. It
    // inherits the model and the working folder, so its files are the same
    // deliverables — but it has no persona identity, and its allowlist is the real
    // constraint (a type with only read tools cannot modify anything, whatever its
    // task asks for).
    let sub = subagent_type.as_deref().and_then(|t| {
        crate::core::subagent::SubagentStore::load(&state.data_dir).get(t).cloned()
    });
    if let Some(t) = &sub {
        specs = crate::core::subagent::allowed_tools(t, &specs);
    }
    // The assistant's own browser: only once the user has switched it on, and never for a
    // delegate. A subagent would share its parent's browser — one page, two runs clicking
    // in it, and nobody able to tell whose click moved it. Connected apps stop at depth 0
    // for the same kind of reason.
    if !settings.browser_enabled || sub.is_some() || depth > 0 {
        specs.retain(|s| !crate::core::agent::is_browser_tool(&s.name));
    }

    // Tier narrowing, last, so it can only ever remove — never widen a subagent's
    // allowlist or a persona's capabilities. `held` is not lost: `find_tools` brings
    // any of it back mid-turn, which is what keeps this a deferral rather than a cut.
    // §4's untested arm, behind a flag. Default stays the fixed priority order, so
    // a matrix run after this is comparable to every table already in
    // HARNESS-SCOREBOARD; turning it on is how the two get compared rather than
    // swapped. `find_tools` remains the escape hatch either way.
    let retrieved = std::env::var("SUPERUSER_TOOL_RETRIEVAL")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    let (mut specs, held) = if retrieved {
        let rel = crate::core::embeddings::tool_relevance(&settings, &specs, &message).await;
        // An empty map means the embedder was unavailable. That degrades to the
        // always-sent set plus catalogue order, which is a worse ranking but still
        // a working turn — never an empty wire.
        shape.split_tools_retrieved(&specs, &rel)
    } else {
        shape.split_tools(&specs)
    };
    // `HARNESS_EVAL_NO_ESCAPE=1` keeps the narrowing and drops the escape hatch. It
    // exists to answer one question the scoreboard raised and could not settle: the
    // Small tier regressed on tasks that need *no* withheld tool at all, so either
    // the shorter list hurts or `find_tools` itself is a distractor a 3B model
    // reaches for. Two arms cannot tell those apart; three can.
    let escape_hatch = !std::env::var("HARNESS_EVAL_NO_ESCAPE").is_ok_and(|v| v == "1");
    if !held.is_empty() && escape_hatch {
        specs.push(crate::core::loopshape::find_tools_spec(&held));
    }
    // E2's arm, behind `SUPERUSER_SKILL_NARROWING=1`: which skills this message needs, and
    // therefore which tools. Offered beside the tier's own narrowing rather than instead of
    // it, because the two answer different questions — how many tools this model can hold,
    // and which of them this request is about. Fails open in every direction (see
    // `core::skillpick`), and is measured against `find_tools` before anything is adopted.
    if crate::core::skillpick::on() && sub.is_none() {
        let named = skills.names_for(&allowed_skills);
        let known: Vec<&crate::core::skills::Skill> = named.iter().filter_map(|n| skills.get(n)).collect();
        let (kept, why) = crate::core::skillpick::narrow(&settings, &specs, &known, &message).await;
        crate::trace!("turn", "skill narrowing: {} of {} tools offered ({})", kept.len(), specs.len(), why.label());
        specs = kept;
    }
    crate::trace!(
        "turn",
        "tools {} sent, {} held ({:?} tier, budget {:?})",
        specs.len(),
        held.len(),
        profile.tier,
        shape.tool_budget
    );
    Toolset { shape, specs, held, skills, skills_text, mcp, sub }
}
