use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub local_only: bool,
    /// Optional headless renderer for `web_fetch`, as a command; empty (the default) is off.
    /// See core/webrender.rs for what it is for and why it does not ship on — in short, the
    /// renderer's own subresource requests are outside the egress guard's reach.
    /// For Lightpanda: `lightpanda fetch --dump`.
    #[serde(default)]
    pub web_render_cmd: String,
    pub ollama_url: String,
    pub ollama_model: String,
    /// A smaller local model for work the user isn't waiting on — triage, summaries,
    /// the nightly consolidation. Empty means "no separate one", and everything
    /// falls back to `ollama_model` exactly as it did before this setting existed.
    ///
    /// Worth setting on any machine whose chat model is large: background work
    /// otherwise takes the same weights the user is talking to.
    #[serde(default)]
    pub small_model: String,
    pub embed_model: String,
    pub cloud_enabled: bool,
    pub cloud_model: String,
    /// Background auto-sync interval in minutes (0 = off).
    #[serde(default = "default_sync_interval")]
    pub sync_interval_min: u32,
    /// Optional self-hosted SearXNG base URL for web search (keyless, private).
    /// Empty = fall back to DuckDuckGo.
    #[serde(default)]
    pub searxng_url: String,
    /// Ollama context window in tokens (how much input the model reads).
    #[serde(default = "default_num_ctx")]
    pub num_ctx: u32,
    /// Max tokens the model may generate in a reply (Ollama num_predict / Claude max_tokens).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// How hard to think: one of core::effort::LEVELS. Defaulted rather than required so
    /// every settings file written before this existed still parses.
    #[serde(default = "default_effort")]
    pub effort: String,
    /// Which microphone to record from, by cpal name. Empty = whatever the OS says is
    /// the default, which is also the fallback when the named one is unplugged.
    #[serde(default)]
    pub mic_device: String,
    /// Hold the mic button to record, instead of click-to-start / click-to-stop.
    #[serde(default)]
    pub mic_hold_to_record: bool,
    /// Auto-approve: let a second model call judge routine actions so they run
    /// without a card, and only the questionable ones interrupt.
    ///
    /// Off by default, and deliberately. It is the one setting here that changes
    /// how much happens without the user looking, so it is theirs to turn on. See
    /// `core::reviewer` for what the judge can and cannot do — in short, it can
    /// skip a card, never a refusal.
    #[serde(default)]
    pub auto_approve: bool,
    /// Let obviously read-only commands run without a card.
    ///
    /// Narrower and far more predictable than `auto_approve`: a deterministic
    /// classifier, local filesystem reads only, no network clients and no
    /// interpreters (`core::cmdgate::readonly`). It exists because a user who has
    /// clicked Allow fifteen times on `ls` and `git log` is a user who will click
    /// Allow on the sixteenth card without reading it.
    #[serde(default)]
    pub auto_allow_read_only_commands: bool,
    /// How many conversations a sidebar group shows before "Show more".
    #[serde(default = "default_sessions_peek")]
    pub sessions_peek: u32,
    /// Show the context-window meter in the composer, rather than a token total.
    #[serde(default)]
    pub context_bar: bool,
    /// Sidebar layout: "grouped" by assistant, or "flat".
    ///
    /// Grouped is the default, and the reason is a product one: assistants are the
    /// organising idea here, and a flat list is
    /// truncated prompts in a column.
    #[serde(default = "default_nav_layout")]
    pub nav_layout: String,
    /// Where NEW conversations get their working folder. Empty = `<data_dir>/sessions`.
    ///
    /// Only new ones: a conversation pins its folder the first time anything asks
    /// for it, and never re-resolves. Changing this must not move work that has
    /// already happened — files people have open, paths already quoted in a
    /// transcript — so the setting is a default for the next conversation, not a
    /// statement about every past one.
    #[serde(default)]
    pub scratch_base: String,
    /// Has the user been through first-run setup?
    ///
    /// Reported to the GUI as the constant `true` before this existed, which meant
    /// onboarding could never appear and Settings' "replay the setup guide" button
    /// had nothing to write to. `App.tsx` has read `s.onboarded` the whole time.
    #[serde(default)]
    pub onboarded: bool,
    /// Which session surfaces the sidebar offers besides Cowork.
    #[serde(default)]
    pub surface_chat: bool,
    #[serde(default)]
    pub surface_code: bool,
    /// Default Telegram chat id for outbound messages / notifications (the bot
    /// token itself lives in the keychain, not here).
    #[serde(default)]
    pub telegram_chat_id: String,
    /// Extra Telegram chat ids allowed to talk to the agent, comma-separated.
    /// Anyone can message a bot they find, and this one is wired to an agent with
    /// file and shell access — so this is an allowlist, and empty means nobody.
    #[serde(default)]
    pub telegram_allowed: String,
    /// Answer incoming Telegram messages. Off means outbound only: the app can
    /// still push you a reminder, but nothing you send back drives anything.
    #[serde(default)]
    pub telegram_inbound: bool,
    /// The email bot (`core::email_bot`): draft a reply to new email from the senders in
    /// `email_allowed`, each draft waiting in the Inbox for approval. Off by default, for the
    /// Telegram reason: anyone can write to you.
    #[serde(default)]
    pub email_inbound: bool,
    /// Who the email bot answers: addresses, or `@domain` for everyone at one, separated by
    /// commas, spaces or lines. Empty means nobody.
    #[serde(default)]
    pub email_allowed: String,
    /// The Expense Manager's intake (`core::expenses`): read receipts that arrive by email into
    /// the expense ledger, with the model on this machine. Off by default.
    #[serde(default)]
    pub expense_intake: bool,
    /// The expense policy the Expense Manager checks receipts against, one rule a line. Empty
    /// means there is nothing to be an exception to.
    #[serde(default)]
    pub expense_policy: String,
    /// The Chief of Staff's priorities (`core::chief_of_staff`), one a line: what the digest maps
    /// activity to. Empty means nothing can map to one.
    #[serde(default)]
    pub cos_priorities: String,
    /// Where Bug Reproduction works (`core::repro`): the staging address, and the only site its
    /// browser may open or act on. Empty means nowhere.
    #[serde(default)]
    pub repro_staging: String,
    /// Product Performance's signals to watch: `Name: PromQL`, one a line (`core::perf::signals`).
    #[serde(default)]
    pub perf_signals: String,
    /// Input-token budget for one assistant turn. Over it, the oldest turns are
    /// summarized away and over-long tool results are truncated. A local model's
    /// real ceiling is `num_ctx`, so the effective budget is the lower of the two.
    #[serde(default = "default_agent_max_context_tokens")]
    pub agent_max_context_tokens: u32,
    /// How many past conversation turns an assistant run carries in. Older turns
    /// are summarized into one paragraph instead of being sent verbatim.
    #[serde(default = "default_agent_max_context_turns")]
    pub agent_max_context_turns: u32,
    /// Tool-call steps a single assistant turn may take before it must answer.
    #[serde(default = "default_agent_max_steps")]
    pub agent_max_steps: u32,
    /// Nightly consolidation of daily memory into the core memory. Off means the
    /// core memory is only ever written by hand (or by an explicit run) — daily
    /// memory keeps accumulating either way, so nothing is lost by disabling it.
    #[serde(default = "tru")]
    pub deep_dream_enabled: bool,
    /// After how many days an untouched *synced* memory is filed under
    /// `vault/archive/` and stops appearing in recall. `0` disables archiving.
    ///
    /// Long by default. The cost of keeping a stale memory one more month is a
    /// slightly noisier ranking; the cost of retiring one too early is the user
    /// asking a question the app used to be able to answer.
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: i64,
    /// Offer `run_plan`, which runs several ordinary tool calls in one model step. Off
    /// by default: it changes how a turn is shaped, and a person should choose that.
    /// Every step is still governed exactly as a single call is.
    #[serde(default)]
    pub plan_tool_enabled: bool,
    /// After a conversation goes idle, review it for anything worth durably
    /// learning — a skill that showed a flaw, a fact the chat missed. Off means the
    /// assistant only ever changes when you change it.
    ///
    /// Needs a model that can follow a structured output format: 7B+ locally, or a
    /// cloud model. Smaller ones reason about the change correctly and then fail to
    /// emit it, so every review comes back empty (the log says so when it happens).
    #[serde(default = "tru")]
    pub self_evolution_enabled: bool,
    /// How long a conversation must be untouched before it's reviewed, so a pass
    /// never runs while someone is still thinking.
    #[serde(default = "default_evolution_idle_minutes")]
    pub self_evolution_idle_minutes: u32,
    /// User turns that must have accumulated since the last review for another to
    /// be worth a model call.
    #[serde(default = "default_evolution_min_turns")]
    pub self_evolution_min_turns: u32,
    /// The evaluating background loop: check what changed against your standing
    /// watches and decide, on the local model, whether any of it is worth acting on.
    /// Off means watches are never evaluated (they stay, and nothing else changes).
    ///
    /// On by default only because it costs nothing when nothing has changed — a
    /// quiet tick makes no model call at all.
    #[serde(default = "tru")]
    pub subconscious_enabled: bool,
    /// Minutes between ticks. Floored at 5: this loop exists to be unobtrusive, and
    /// a tighter interval mostly re-asks the same question about the same items.
    #[serde(default = "default_subconscious_interval")]
    pub subconscious_interval_min: u32,
    /// Learn how you want to be worked with from what you say, instead of only from
    /// what you typed into the profile form once. Off means the profile is exactly
    /// the form and nothing else — what already exists keeps applying.
    ///
    /// Reads only your own messages, never tool results or synced content, and
    /// everything it learns is visible and removable in Brain ▸ Profile.
    #[serde(default = "tru")]
    pub profile_learning_enabled: bool,
    /// Sweep memory before the first turn of a new conversation, so it starts
    /// knowing what this machine already holds on the subject. Off means every
    /// thread starts cold and the agent has to think to go looking.
    ///
    /// Free: the sweep is keyword scoring over local files — no model call.
    #[serde(default = "tru")]
    pub supercontext_enabled: bool,
    /// How the user wants every assistant to work, in their own words — applied to
    /// every run, after the assistant's own role.
    ///
    /// Different from the profile, which is learned and fades, and from AGENTS.md,
    /// which belongs to one workspace: this is the one place a person writes down,
    /// once, what should always hold. Bounded at [`STANDING_INSTRUCTIONS_MAX`].
    #[serde(default)]
    pub standing_instructions: String,
    /// Seconds a cloud model may send nothing before its turn is ended. 0 is off.
    /// See `core::stall`.
    #[serde(default = "default_stall_seconds")]
    pub stall_seconds: u32,
    /// The same for a model on this machine, which is allowed far longer: loading a
    /// model and reading a long prompt are silent and slow on a laptop.
    #[serde(default = "default_stall_seconds_local")]
    pub stall_seconds_local: u32,
    /// Days of audit trail to keep. 0 — the default — keeps all of it: deleting
    /// somebody's record because a default said so is the worse failure.
    #[serde(default)]
    pub audit_retention_days: u32,
    /// Give assistants that can use the web their own browser.
    ///
    /// Off until the user turns it on. A browser that stays signed in to the user's sites
    /// and can click in them is a different kind of reach from reading a page, and the
    /// user should meet it in Settings, not discover it in a transcript.
    #[serde(default)]
    pub browser_enabled: bool,
    /// Let the assistant's browser open pages served by this machine — a dev server on
    /// localhost. Off by default: this machine's own services are not the web.
    #[serde(default)]
    pub browser_allow_loopback: bool,
    /// Run the browser in a visible window rather than headless, for sites that refuse a
    /// headless browser at sign-in.
    #[serde(default)]
    pub browser_headed: bool,
    /// Let assistants draw a small interactive view in the conversation (Phase 5, G1).
    ///
    /// Off by default: it renders HTML an assistant wrote. Sandboxed — its own origin, no
    /// network, no reach into this app or its storage — but "a model wrote the page you
    /// are looking at" is a thing to agree to rather than to discover.
    #[serde(default)]
    pub generative_ui: bool,
    /// Derived field (not persisted): whether a key exists in the keychain.
    #[serde(default)]
    pub has_cloud_key: bool,
}
