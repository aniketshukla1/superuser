use super::*;

impl LoopShape {
    /// What this model gets.
    pub fn for_model(profile: &ModelProfile, settings: &crate::core::settings::Settings) -> Self {
        let steps = settings.agent_max_steps as usize;
        if shaping_disabled() {
            return Self::unshaped(settings);
        }
        let shape = Self::for_tier(profile, settings, steps);
        // Narrowing only, and after the tier has had its say, so the override cannot
        // hand a model *more* than its tier allows.
        match (eval_tool_budget(), shape.tool_budget) {
            (Some(want), None) => LoopShape { tool_budget: Some(want), ..shape },
            (Some(want), Some(have)) => LoopShape { tool_budget: Some(want.min(have)), ..shape },
            (None, _) => shape,
        }
    }

    pub(in crate::core::loopshape) fn for_tier(profile: &ModelProfile, settings: &crate::core::settings::Settings, steps: usize) -> Self {
        match profile.tier {
            // **No tool budget, deliberately, and this is not an oversight.**
            //
            // Small started at a 1,400-token budget — 13 tools instead of 37 — on the
            // reasoning that a weak model reading a long catalogue picks worse. The
            // scoreboard graded that and it was wrong: `llama3.2:3b` scored 6/19
            // narrowed against 10/19 with the full list, stable across three ablations
            // (with and without `find_tools`, with and without the skills menu), against
            // a measured run-to-run variance of ±1–2 tasks.
            //
            // The four tasks it cost need **no withheld tool** — one of them calls no
            // tool at all — so this is not "the tool it needed was taken away", and no
            // mechanism here explains it beyond "small models are brittle to changes in
            // the schema set in ways that are not about availability".
            //
            // So Small is now *less* restricted than Mid. That reads backwards and is
            // what the evidence says: the tier is not a dial for how much to withhold,
            // it is a record of what each class of model was measured to tolerate. The
            // narrowing machinery stays — `split_tools` still serves Mid — and is
            // available to Small again the day a Small model is measured to benefit.
            // None has been. See HARNESS-BASELINE, "Phase 5, graded".
            Tier::Small => LoopShape {
                tool_budget: None,
                // Kept. This was never implicated: median steps on the failing tasks
                // was 2, nowhere near the cap, and a small model still in flight after
                // a dozen steps is looping rather than converging. The user's setting
                // remains the ceiling — this only ever lowers it.
                max_steps: steps.min(12).max(4),
                // Kept on, matching the arm that scored 10/19. Ablation A4 showed the
                // menu makes no difference either way on this tier, so the tie goes to
                // the condition that was actually measured good.
                skills_in_prompt: true,
                parallel_tools: profile.tools == ToolSupport::Parallel,
            },
            // **No tool budget either, and for a reason Small's arm below does not
            // cover.** Small was un-narrowed because narrowing made it score *worse*.
            // Mid is un-narrowed because narrowing costs a round trip, and on a local
            // model a round trip is the whole complaint.
            //
            // Withholding tools does not remove them from the turn; it defers them.
            // The model spends its first step calling `find_tools` to ask for the list
            // back, and that step is a complete model call — prefill of the entire
            // prompt, then generation. Measured on the owner's machine, 2026-09-05:
            // ~30 seconds, producing no work. Three consecutive turns in the run
            // journals open with exactly that, at +27.8s, +35.6s and +46.4s.
            //
            // So the trade the budget was making is not "fewer tokens for a slightly
            // worse pick". It is "3,000 tokens saved on step one, ~6,500 tokens and a
            // full round trip spent on step two" — strictly worse on both axes it was
            // introduced to improve. The honest way to spend fewer tokens on tools is
            // to offer fewer tools, which is a decision about the persona's capability
            // list, not something to discover at runtime and then undo.
            //
            // `split_tools` stays and stays tested: it is correct machinery, and the
            // day a tier is *measured* to benefit from deferral it is one line away.
            // None has been.
            Tier::Mid => LoopShape {
                tool_budget: None,
                max_steps: steps,
                skills_in_prompt: true,
                parallel_tools: profile.tools == ToolSupport::Parallel,
            },
            Tier::Frontier => LoopShape {
                // Nothing withheld: on Anthropic the tool-search deferral already
                // shrinks what enters the context, and elsewhere a frontier model can
                // read the whole catalogue without picking worse for it.
                tool_budget: None,
                max_steps: steps,
                skills_in_prompt: true,
                parallel_tools: profile.tools == ToolSupport::Parallel,
            },
        }
    }

    /// The loop with no shaping at all — the control condition for the scoreboard.
    ///
    /// Named and public so it can be asserted against directly: a test that had to set
    /// an environment variable to reach this would be racing every other test in the
    /// process.
    pub fn unshaped(settings: &crate::core::settings::Settings) -> Self {
        LoopShape {
            tool_budget: None,
            max_steps: settings.agent_max_steps as usize,
            skills_in_prompt: true,
            // Pre-Phase-5 the parallel group was gated on risk class alone, never on
            // what the model said it could do. Faithful means faithful.
            parallel_tools: true,
        }
    }

    /// Split the persona's tools into what is sent now and what is held back.
    ///
    /// Resident tools first, in `RESIDENT`'s own order, then everything else in the
    /// order capability expansion produced — which `tools_claude` sorts by name on the
    /// way out, so the cache prefix is stable either way.
    ///
    /// A tool is never dropped for being unaffordable while a later, cheaper one is
    /// kept: the walk stops at the first tool that does not fit. Filling the gap with
    /// whatever happens to be small would make the sent set depend on schema length
    /// rather than on usefulness.
    /// Tools that go on the wire whatever the query looks like.
    ///
    /// Smaller than `agent::RESIDENT` on purpose. §4's argument is that a *fixed*
    /// subset is not a small catalogue, it is a different one — so the fixed part
    /// here is only what a turn cannot proceed without: the two ways to touch a
    /// file, the way to run anything, and the escape hatch that can fetch the rest.
    pub const ALWAYS_SENT: &'static [&'static str] =
        &["read_file", "write_file", "run_shell", "find_tools"];
}
