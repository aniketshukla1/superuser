use super::*;

    #[test]
    pub(super) fn how_much_a_tier_actually_saves_on_the_widest_persona() {
        // The companion to `agent::how_big_is_the_resident_tool_schema`, which
        // measures the catalogue this narrows. Printed rather than asserted tightly,
        // so the numbers in HARNESS-BASELINE are re-measured every run instead of
        // being quoted from a commit message.
        let ctx = crate::core::agent::AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: true,
            has_todo: true,
        };
        let caps: Vec<String> = [
            "files", "search", "shell", "git", "todo", "web", "toolchain", "board",
            "subagent", "knowledge", "recall_memory", "connectors", "messaging",
            "schedule", "computer", "plan", "ask",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let specs = crate::core::agent::expand(&caps, &ctx);
        let whole: usize = specs.iter().map(schema_tokens).sum();
        for tier in [Tier::Small, Tier::Mid, Tier::Frontier] {
            let shape = LoopShape::for_model(&profile(tier, ToolSupport::Sequential), &Settings::default());
            let (sent, held) = shape.split_tools(&specs);
            let cost: usize = sent.iter().map(schema_tokens).sum();
            let find = if held.is_empty() { 0 } else { schema_tokens(&find_tools_spec(&held)) };
            eprintln!(
                "TIER {tier:?}: {} of {} tools, ~{} of ~{} tokens (+{} for find_tools) — {}% cut",
                sent.len(),
                specs.len(),
                cost,
                whole,
                find,
                100 - ((cost + find) * 100 / whole.max(1))
            );
            // Whatever the budget, the model can always read, write and ask.
            let names: Vec<&str> = sent.iter().map(|t| t.name.as_str()).collect();
            for must in ["read_file", "write_file", "list_dir", "run_shell", "ask_user"] {
                assert!(names.contains(&must), "{tier:?} dropped {must}");
            }
        }
    }
