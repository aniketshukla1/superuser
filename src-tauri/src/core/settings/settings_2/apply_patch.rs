use super::*;

impl Settings {
    /// Apply a partial JSON patch (only the keys present are changed).
    pub fn apply_patch(&mut self, patch: &serde_json::Value) {
        if let Some(v) = patch.get("local_only").and_then(|v| v.as_bool()) {
            self.local_only = v;
        }
        if let Some(v) = patch.get("ollama_url").and_then(|v| v.as_str()) {
            self.ollama_url = v.to_string();
        }
        if let Some(v) = patch.get("ollama_model").and_then(|v| v.as_str()) {
            self.ollama_model = v.to_string();
        }
        if let Some(v) = patch.get("small_model").and_then(|v| v.as_str()) {
            self.small_model = v.to_string();
        }
        // Only a model that can actually embed. A chat model here does not fail
        // loudly — Ollama answers `501 This server does not support embeddings`,
        // semantic recall falls back to keyword matching, and everything keeps
        // working slightly worse forever. That is exactly what happened on the
        // author's machine, undetected for months.
        //
        // Refused rather than clamped, unlike `sync_interval_min` below: there is no
        // "nearest sane value" for a model name. The test is `ollama::is_embed_model`,
        // which is a name heuristic and so can in principle refuse an embedding model
        // named unlike any of them — a settings.json edit still gets through, and is
        // the documented way out.
        if let Some(v) = patch.get("embed_model").and_then(|v| v.as_str()) {
            if crate::core::ollama::is_embed_model(v) {
                self.embed_model = v.to_string();
            }
        }
        if let Some(v) = patch.get("cloud_enabled").and_then(|v| v.as_bool()) {
            self.cloud_enabled = v;
        }
        if let Some(v) = patch.get("cloud_model").and_then(|v| v.as_str()) {
            self.cloud_model = v.to_string();
        }
        if let Some(v) = patch.get("sync_interval_min").and_then(|v| v.as_u64()) {
            // A zero interval isn't "as often as possible", it's a background loop
            // with no sleep in it. Clamp rather than reject, so a bad value from
            // the API degrades to the slowest sane setting instead of a spin.
            self.sync_interval_min = v.clamp(1, 24 * 60) as u32;
        }
        if let Some(v) = patch.get("searxng_url").and_then(|v| v.as_str()) {
            self.searxng_url = v.trim().trim_end_matches('/').to_string();
        }
        if let Some(v) = patch.get("num_ctx").and_then(|v| v.as_u64()) {
            self.num_ctx = (v as u32).clamp(1024, 131072);
        }
        if let Some(v) = patch.get("max_tokens").and_then(|v| v.as_u64()) {
            self.max_tokens = (v as u32).clamp(256, 32768);
        }
        if let Some(v) = patch.get("telegram_chat_id").and_then(|v| v.as_str()) {
            self.telegram_chat_id = v.trim().to_string();
        }
        if let Some(v) = patch.get("telegram_allowed").and_then(|v| v.as_str()) {
            self.telegram_allowed = v.trim().to_string();
        }
        if let Some(v) = patch.get("telegram_inbound").and_then(|v| v.as_bool()) {
            self.telegram_inbound = v;
        }
        if let Some(v) = patch.get("email_inbound").and_then(|v| v.as_bool()) {
            self.email_inbound = v;
        }
        if let Some(v) = patch.get("email_allowed").and_then(|v| v.as_str()) {
            self.email_allowed = v.trim().to_string();
        }
        if let Some(v) = patch.get("expense_intake").and_then(|v| v.as_bool()) {
            self.expense_intake = v;
        }
        if let Some(v) = patch.get("expense_policy").and_then(|v| v.as_str()) {
            self.expense_policy = v.trim().chars().take(4_000).collect();
        }
        if let Some(v) = patch.get("cos_priorities").and_then(|v| v.as_str()) {
            self.cos_priorities = v.trim().chars().take(4_000).collect();
        }
        if let Some(v) = patch.get("perf_signals").and_then(|v| v.as_str()) {
            self.perf_signals = v.trim().chars().take(8_000).collect();
        }
        if let Some(v) = patch.get("repro_staging").and_then(|v| v.as_str()) {
            self.repro_staging = v.trim().chars().take(500).collect();
        }
        if let Some(v) = patch.get("agent_max_context_tokens").and_then(|v| v.as_u64()) {
            self.agent_max_context_tokens = (v as u32).clamp(4000, 400_000);
        }
        if let Some(v) = patch.get("agent_max_context_turns").and_then(|v| v.as_u64()) {
            self.agent_max_context_turns = (v as u32).clamp(2, 200);
        }
        if let Some(v) = patch.get("agent_max_steps").and_then(|v| v.as_u64()) {
            self.agent_max_steps = (v as u32).clamp(1, 100);
        }
        if let Some(v) = patch.get("plan_tool_enabled").and_then(|v| v.as_bool()) {
            self.plan_tool_enabled = v;
        }
        if let Some(v) = patch.get("archive_after_days").and_then(|v| v.as_i64()) {
            self.archive_after_days = v.max(0);
        }
        if let Some(v) = patch.get("deep_dream_enabled").and_then(|v| v.as_bool()) {
            self.deep_dream_enabled = v;
        }
        if let Some(v) = patch.get("self_evolution_enabled").and_then(|v| v.as_bool()) {
            self.self_evolution_enabled = v;
        }
        if let Some(v) = patch.get("self_evolution_idle_minutes").and_then(|v| v.as_u64()) {
            // At least a minute: a zero idle window would review a conversation the
            // user is still in the middle of.
            self.self_evolution_idle_minutes = (v as u32).clamp(1, 1440);
        }
        if let Some(v) = patch.get("self_evolution_min_turns").and_then(|v| v.as_u64()) {
            self.self_evolution_min_turns = (v as u32).clamp(2, 100);
        }
        if let Some(v) = patch.get("profile_learning_enabled").and_then(|v| v.as_bool()) {
            self.profile_learning_enabled = v;
        }
        if let Some(v) = patch.get("supercontext_enabled").and_then(|v| v.as_bool()) {
            self.supercontext_enabled = v;
        }
        if let Some(v) = patch.get("standing_instructions").and_then(|v| v.as_str()) {
            self.standing_instructions = v.trim().chars().take(STANDING_INSTRUCTIONS_MAX).collect();
        }
        // 0 switches a watchdog off. Anything else is floored at ten seconds: a limit of
        // one would end every turn before a cloud model's first token, and the setting
        // that did that would look like the provider failing.
        let stall = |v: u64| if v == 0 { 0 } else { v.clamp(10, 3600) as u32 };
        if let Some(v) = patch.get("stall_seconds").and_then(|v| v.as_u64()) {
            self.stall_seconds = stall(v);
        }
        if let Some(v) = patch.get("stall_seconds_local").and_then(|v| v.as_u64()) {
            self.stall_seconds_local = stall(v);
        }
        if let Some(v) = patch.get("audit_retention_days").and_then(|v| v.as_u64()) {
            self.audit_retention_days = v.min(3650) as u32;
        }
        for (key, slot) in [
            ("generative_ui", &mut self.generative_ui),
            ("browser_enabled", &mut self.browser_enabled),
            ("browser_allow_loopback", &mut self.browser_allow_loopback),
            ("browser_headed", &mut self.browser_headed),
        ] {
            if let Some(v) = patch.get(key).and_then(|v| v.as_bool()) {
                *slot = v;
            }
        }
    }
}
