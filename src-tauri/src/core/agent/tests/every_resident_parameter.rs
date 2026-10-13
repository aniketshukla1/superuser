use super::*;

    #[test]
    pub(super) fn every_resident_parameter_is_documented_and_the_bill_is_stated() {
        // TOOLS-RESEARCH-2026 §2. The catalogue was documented backwards: the fifteen
        // tools paid for on *every* request carried 4 described parameters out of 24,
        // while the long tail nobody reaches was at 54%. That was chronology — the
        // core loop was written when the schemas were four obvious keys — and this
        // test is what stops it happening again.
        //
        // Resident only. The same rule across all 53 would be the blanket
        // augmentation the source paper measures at +67.46% steps and a regression in
        // one case in six; the resident set is where the evidence is.
        let mut undocumented: Vec<String> = Vec::new();
        let mut resident_cost = 0usize;
        for (_, name) in every_tool() {
            if !is_resident(&name) {
                continue;
            }
            let Some(spec) = catalog().iter().flat_map(|c| (c.build)()).find(|t| t.name == name) else { continue };
            resident_cost += serde_json::to_string(&spec.schema).map(|s| s.len()).unwrap_or(0) / 4;
            let Some(props) = spec.schema["properties"].as_object() else { continue };
            for (key, v) in props {
                if v["description"].as_str().unwrap_or("").trim().is_empty() {
                    undocumented.push(format!("{name}.{key}"));
                }
            }
        }
        assert!(
            undocumented.is_empty(),
            "resident tools are on the wire every single turn, so an undocumented parameter there is \
             paid for unconditionally and read constantly: {undocumented:?}"
        );
        // Not an assertion about taste — a ceiling. These schemas are the only tokens
        // in the catalogue nothing can defer, so the number belongs in a test rather
        // than in someone's memory of what it used to be.
        assert!(
            resident_cost < 1_600,
            "the resident schemas now cost ~{resident_cost} tokens on every request; \
             the descriptions are meant to be short"
        );
        eprintln!("resident schema cost: ~{resident_cost} tokens/request");
    }

    /// An assistant that can touch files can touch documents.
    ///
    /// The bug this pins: spreadsheets shipped as their own capability, granted to
    /// exactly one built-in persona. Every other assistant — and every one the user
    /// creates — silently could not make or edit a workbook, and the
    /// create-assistant API accepts only six tool ids, so it could not have been
    /// granted there even deliberately.
    /// Showing a chart and keeping one are different asks, and the SCHEMA is where
    /// the model learns that. `path` was required, so a chart could not be drawn
    /// without also writing a file — one session here collected eleven nobody had
    /// asked for. The executor is tested separately; this pins the half the model
    /// reads.
    #[test]
    pub(super) fn a_chart_can_be_drawn_without_being_saved() {
        let chart = catalog()
            .iter()
            .flat_map(|c| (c.build)())
            .find(|t| t.name == "chart")
            .expect("the chart tool");
        let required: Vec<String> = chart.schema["required"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        assert!(!required.contains(&"path".to_string()), "saving must be opt-in: {required:?}");
        // The two that ARE required stay required: a chart with no spec draws nothing,
        // and one with no source is a picture of numbers nobody can trace.
        assert!(required.contains(&"spec".to_string()));
        assert!(required.contains(&"source".to_string()));
        // And the description has to say when to pass a path, or the model will guess.
        assert!(
            chart.description.contains("ONLY if the user asked to save"),
            "the model needs the rule, not just the absence of a requirement"
        );
    }

    #[test]
    pub(super) fn how_big_is_the_resident_tool_schema() {
        // HARNESS-PLAN §2.13 says 10.2k tokens, always, whether the turn needs
        // `sheet_write` or not. Measured here rather than quoted, because every
        // reduction Phase 6 claims is measured against this number.
        let ctx = AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: true,
            has_todo: true,
        };
        // A workbench's capabilities — the widest set the app ships.
        let caps: Vec<String> = ["files", "search", "shell", "git", "todo", "web", "toolchain", "board", "subagent", "knowledge", "recall_memory", "connectors", "messaging", "schedule", "computer", "plan", "ask"]
            .iter().map(|s| s.to_string()).collect();
        let specs = expand(&caps, &ctx);
        let wire: Vec<serde_json::Value> = specs
            .iter()
            .map(|t| serde_json::json!({"name": t.name, "description": t.description, "input_schema": t.schema}))
            .collect();
        let json = serde_json::to_string(&wire).unwrap();
        // chars/4 is the estimator `context.rs` uses; good enough to size a budget.
        eprintln!(
            "RESIDENT TOOL SCHEMA: {} tools, {} chars, ~{} tokens",
            specs.len(),
            json.len(),
            json.len() / 4
        );
        let mut by_cap: std::collections::BTreeMap<&str, Vec<(&str, usize)>> = Default::default();
        for t in &specs {
            let bytes = serde_json::to_string(&serde_json::json!({"name": t.name, "description": t.description, "input_schema": t.schema})).unwrap().len();
            by_cap.entry(&t.capability).or_default().push((&t.name, bytes));
        }
        for (cap, tools) in &by_cap {
            let total: usize = tools.iter().map(|(_, b)| b).sum();
            eprintln!("  {:14} ~{:5} tok  {}", cap, total / 4, tools.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(" "));
        }
        assert!(!specs.is_empty());
    }

    #[test]
    pub(super) fn browser_open_is_the_name_the_panel_watches_for() {
        // The Browser tab surfaces itself by matching this exact string in the
        // Activity feed (`BROWSER_OPEN_TOOL` in WorkspacePanel.tsx). Renaming the
        // tool without renaming it there would not fail to compile on either side —
        // the tab would just quietly stop opening, which is the failure this whole
        // plan keeps running into.
        let ctx = AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: false,
            has_todo: false,
        };
        let names: Vec<String> = expand(&["web".to_string()], &ctx).into_iter().map(|t| t.name).collect();
        assert!(
            names.contains(&"browser_open".to_string()),
            "the web capability should offer browser_open; it has {names:?}"
        );

        // And it must be `web` SPECIFICALLY, because the GUI decides whether to show
        // the Browser button from that capability alone (`personaScope::showsBrowser`).
        // Both facts have to stay together: the panel watches for this tool name, and
        // the button that opens the panel is granted by this capability. When they were
        // allowed to differ — the button hung off `shell` — a knowledge assistant could
        // load a page into a panel the user had no way to open.
        let shell_only: Vec<String> =
            expand(&["shell".to_string()], &ctx).into_iter().map(|t| t.name).collect();
        assert!(
            !shell_only.contains(&"browser_open".to_string()),
            "browser_open must come from `web`, not `shell` — the GUI gates the Browser \
             button on `web` and would hide it from every web-capable assistant otherwise"
        );
    }

    #[test]
    pub(super) fn every_persona_with_file_access_can_work_in_documents() {
        let ctx = AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: false,
            has_todo: false,
        };
        // The default toolset a newly created assistant is given.
        for persona in [
            vec!["recall_memory", "web", "files", "todo", "connectors"],
            vec!["files"],
            vec!["files", "search", "shell", "git", "todo"],
        ] {
            let ids: Vec<String> = persona.iter().map(|s| s.to_string()).collect();
            let names: Vec<String> = expand(&ids, &ctx).into_iter().map(|t| t.name).collect();
            // Editing a deck, not just replacing one — the same reasoning as the
            // sheet tools: a persona that can touch files must be able to change a
            // presentation without rebuilding it, or the only way to "edit" one is
            // to destroy its design.
            for want in
                ["sheet_read", "sheet_new", "sheet_write", "slides_read", "slides_edit", "doc_read", "doc_edit"]
            {
                assert!(names.contains(&want.to_string()), "{persona:?} cannot {want}");
            }
        }
    }
