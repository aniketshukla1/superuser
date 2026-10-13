use super::*;

pub(super) fn default_sessions_peek() -> u32 {
    5
}

/// The longest standing instructions accepted. They go into every
/// turn of every assistant, so they cost tokens on all of them.
pub const STANDING_INSTRUCTIONS_MAX: usize = 4000;

pub(super) fn default_stall_seconds() -> u32 {
    crate::core::stall::DEFAULT_CLOUD_SECS
}

pub(super) fn default_stall_seconds_local() -> u32 {
    crate::core::stall::DEFAULT_LOCAL_SECS
}

pub(super) fn default_nav_layout() -> String {
    "grouped".into()
}

pub(super) fn default_sync_interval() -> u32 {
    15
}

pub(super) fn default_num_ctx() -> u32 {
    16384
}

pub(super) fn default_max_tokens() -> u32 {
    4096
}

pub(super) fn default_effort() -> String {
    crate::core::effort::DEFAULT.into()
}

pub(super) fn default_agent_max_context_tokens() -> u32 {
    48000
}

pub(super) fn default_agent_max_context_turns() -> u32 {
    20
}

pub(super) fn default_agent_max_steps() -> u32 {
    24
}

pub(super) fn tru() -> bool {
    true
}

pub(super) fn default_evolution_idle_minutes() -> u32 {
    10
}

pub(super) fn default_subconscious_interval() -> u32 {
    15
}

pub(super) fn default_evolution_min_turns() -> u32 {
    6
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            local_only: true,
            // Off. It is a prototype, and the renderer's subresource requests are outside
            // what the egress guard can see — see core/webrender.rs.
            web_render_cmd: String::new(),
            ollama_url: "http://localhost:11434".into(),
            // Match the model every "pull one" message names. The old default was
            // llama3.1, which those messages never mention and most installs don't have,
            // so a fresh app immediately fell back to an auto-picked model instead.
            ollama_model: "llama3.2".into(),
            small_model: String::new(),
            embed_model: "nomic-embed-text".into(),
            cloud_enabled: false,
            // The current flagship, at the same price as 4.8 and with a lower cache
            // minimum (512 vs 1024 tokens) — which matters directly once prompt
            // caching lands (HARNESS-PLAN §2.7).
            cloud_model: "claude-opus-5".into(),
            sync_interval_min: 15,
            searxng_url: String::new(),
            num_ctx: 16384,
            max_tokens: 4096,
            effort: crate::core::effort::DEFAULT.into(),
            mic_device: String::new(),
            mic_hold_to_record: false,
            auto_approve: false,
            auto_allow_read_only_commands: false,
            sessions_peek: default_sessions_peek(),
            context_bar: false,
            nav_layout: default_nav_layout(),
            scratch_base: String::new(),
            onboarded: false,
            surface_chat: false,
            surface_code: false,
            telegram_chat_id: String::new(),
            telegram_allowed: String::new(),
            // Off by default: turning a bot token into an inbound channel to your
            // own machine should be a deliberate act, not a side effect of pasting
            // a token to get notifications.
            telegram_inbound: false,
            email_inbound: false,
            email_allowed: String::new(),
            expense_intake: false,
            expense_policy: String::new(),
            cos_priorities: String::new(),
            repro_staging: String::new(),
            perf_signals: String::new(),
            agent_max_context_tokens: default_agent_max_context_tokens(),
            agent_max_context_turns: default_agent_max_context_turns(),
            agent_max_steps: default_agent_max_steps(),
            deep_dream_enabled: true,
            archive_after_days: default_archive_after_days(),
            plan_tool_enabled: false,
            self_evolution_enabled: true,
            subconscious_enabled: true,
            subconscious_interval_min: default_subconscious_interval(),
            self_evolution_idle_minutes: default_evolution_idle_minutes(),
            self_evolution_min_turns: default_evolution_min_turns(),
            profile_learning_enabled: true,
            supercontext_enabled: true,
            standing_instructions: String::new(),
            stall_seconds: default_stall_seconds(),
            stall_seconds_local: default_stall_seconds_local(),
            audit_retention_days: 0,
            generative_ui: false,
            browser_enabled: false,
            browser_allow_loopback: false,
            browser_headed: false,
            has_cloud_key: false,
        }
    }
}
