// What the mock API serves: health, settings, personas, sessions, connectors, inbox.

export const HEALTH = { status: "ok", default_workspace: null, model: "anthropic:claude-opus-4-8" };

export const SETTINGS = {
  provider: "openai",
  model: "anthropic:claude-opus-4-8",
  models: ["anthropic:claude-opus-4-8", "gpt-5.5", "gpt-4o", "gpt-4o-mini", "o3-mini"],
  has_key: true,
  model_ready: true,
  source: "store",
  onboarded: true,
  experimental_connectors: false,
  surfaces: { cowork: true, chat: false, code: true },
  nav_layout: "grouped",
  scratch_base: "~/Superuser",
  secrets_path: "/Users/test/.config/coworker/secrets.json",
  sessions_peek: 5,
  // Token savings (PDF attachments): 2-page limit keeps the composer threshold test's
  // fixture PDF small; the real default is 20.
  pdf_fallback: "text",
  pdf_max_pages: 2,
  pdf_max_mb: 10,
  // Curated-matrix display names (subset — mirrors /v1/settings.model_labels).
  model_labels: {
    "anthropic:claude-opus-4-8": "Claude Opus 4.8 · Anthropic",
    "zai:glm-5.2": "GLM-5.2 · Z AI",
  },
  // Context windows (subset — mirrors /v1/settings.model_context_windows); drives the
  // composer usage chip's context-fill meter.
  model_context_windows: {
    "anthropic:claude-opus-4-8": 200_000,
  },
};

// GET /v1/usage/rollup (usage_rollup.rs): the context ring's "Tokens through this app".
// Today carries one turn that reported nothing, which the panel must say rather than fold
// into the total.
export const USAGE_ROLLUP = {
  today: { input: 30_000, output: 4_500, tokens: 34_500, counted: 3, uncounted: 1, resets_at: 0 },
  week: { input: 400_000, output: 56_000, tokens: 456_000, counted: 21, uncounted: 1, resets_at: 0 },
  now: 0,
};

export const PERSONAS = {
  personas: [
    { id: "cowork", name: "Assistant", icon: "cowork", tagline: "Produce a deliverable — research, analysis, scripts", needs_workspace: true, builtin: true, family: "knowledge", workspace: "deliverable", tools: ["files", "search"], enabled: true, surfaced: true, default: true },
    { id: "code", name: "Code", icon: "code", tagline: "Work in a codebase — files, git, shell", needs_workspace: true, builtin: true, family: "code", workspace: "git", tools: ["code_files", "git"], enabled: true, surfaced: true, default: false },
    { id: "chat", name: "Chat", icon: "chat", tagline: "Quick questions — no workspace", needs_workspace: false, builtin: true, family: "knowledge", workspace: "none", tools: [], enabled: true, surfaced: false, default: false },
    // The builtin manifest's own name (personas_builtin/ops.md). Chrome shows it short —
    // shortPersonaName drops the " Assistant" — so the sidebar group reads "Ops".
    { id: "ops", name: "Ops Assistant", icon: "wrench", tagline: "Incidents and runbooks — logs, metrics, infrastructure", needs_workspace: true, builtin: true, family: "knowledge", workspace: "deliverable", tools: ["files", "shell"], enabled: true, surfaced: true, default: false },
    // A non-builtin install (disabled pending consent — invisible to picker specs) so the
    // Personas page's delete/enable affordances have a target.
    { id: "acme-notes", name: "Acme Notes", icon: "pencil", tagline: "Acme's note-taking coworker", needs_workspace: true, builtin: false, family: "knowledge", workspace: "deliverable", tools: ["files"], enabled: false, surfaced: false, default: false },
  ],
};

// The boot-resume target (most recent updated_at) — existing specs open it by title.
export const PINNED_SESSION = {
  session_id: "pinned-cowork-1",
  title: "Draft the launch note",
  workspace: "/Users/test/Superuser/launch-note",
  agent: "cowork",
  model: "anthropic:claude-opus-4-8",
  mode: "interactive",
  updated_at: "2026-07-01 09:00:00",
  messages: 2,
  pinned: true,
  archived: false,
  attention: 0,
  liveness: "idle",
  subscriptions: [],
};

// Seven unpinned Coworker sessions: enough to exercise the sidebar peek cap (5) + "Show more (2)".
// wp-3 carries the pending Inbox approval below (attention badge parity). All OLDER than the
// pinned session so boot-resume stays deterministic.
export const EXTRA_SESSIONS = Array.from({ length: 7 }, (_, i) => ({
  session_id: `wp-${i + 1}`,
  title: `Weekly plan ${i + 1}`,
  workspace: "",
  agent: "cowork",
  model: "anthropic:claude-opus-4-8",
  mode: "interactive",
  updated_at: `2026-06-2${8 - Math.min(i, 7)} 10:00:00`,
  messages: 3,
  pinned: false,
  archived: false,
  attention: i + 1 === 3 ? 1 : 0,
  liveness: "idle",
  subscriptions: [],
}));

// One Ops session (older than everything above so boot-resume stays deterministic) — the
// target for the disable-archives-conversations confirm flow on the Personas page.
export const OPS_SESSION = {
  session_id: "ops-1",
  title: "Ops triage",
  workspace: "/Users/test/Superuser/ops-triage",
  agent: "ops",
  model: "anthropic:claude-opus-4-8",
  mode: "interactive",
  updated_at: "2026-06-15 10:00:00",
  messages: 4,
  pinned: false,
  archived: false,
  attention: 0,
  liveness: "idle",
  subscriptions: [],
};

// §31: a mention-spawned session — lives in the sidebar's collapsed "From Slack" group, never
// in Recent. Older than everything else so boot-resume stays deterministic.
export const SLACK_SESSION = {
  session_id: "slack-thread-1",
  title: "#general — check the deploy?",
  workspace: "",
  agent: "cowork",
  model: "anthropic:claude-opus-4-8",
  mode: "interactive",
  updated_at: "2026-06-10 10:00:00",
  messages: 2,
  pinned: false,
  archived: false,
  attention: 0,
  liveness: "idle",
  subscriptions: [],
  origin: "slack",
  origin_label: "#general · T0AB",
};

export const CONNECTORS = {
  connectors: [
    { name: "browser", title: "Browser", icon: "B", blurb: "Headless browser.", auth: "none", two_way: false, channels: false, available: true, brand_color: "#6b7280", logo: "", fields: [], instructions: [], connected: true, account: null, enabled: true, allowed_users: [], tools: [], managed: false, managed_profile: false },
    { name: "telegram", title: "Telegram", icon: "T", blurb: "Two-way Telegram messaging.", auth: "bot_token", two_way: true, channels: true, available: true, brand_color: "#229ed9", logo: "telegram", fields: [{ key: "bot_token", label: "Bot token", secret: true, required: true, help: "", placeholder: "123456:ABC…" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: false, managed_profile: false },
    // Managed-capable connector (one-click via cloud when signed in; manual paste otherwise).
    // Carries pre-connect detail copy (§38): about + access + tools drive available-detail.spec.ts.
    { name: "gmail", title: "Gmail", icon: "✉", blurb: "Search, summarize, draft, and send email.", about: "Search, summarize, and send over your Gmail.", access: ["Reads and searches your mail.", "Sends email as you.", "Never deletes mail or changes account settings."], auth: "oauth", two_way: false, channels: false, available: true, brand_color: "#ea4335", logo: "gmail", fields: [{ key: "access_token", label: "OAuth access token", secret: true, required: true, help: "", placeholder: "" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [{ name: "gmail_search", label: "Search mail", kind: "read", description: "Search messages.", enabled: true, requires_approval: false }, { name: "gmail_send", label: "Send email", kind: "write", description: "Send a message.", enabled: true, requires_approval: true }], managed: true, managed_profile: false },
    { name: "google_calendar", title: "Google Calendar", icon: "◷", blurb: "Read availability, summarize schedules, and create events.", auth: "oauth", two_way: false, channels: false, available: true, brand_color: "#4285f4", logo: "google_calendar", fields: [{ key: "access_token", label: "OAuth access token", secret: true, required: true, help: "", placeholder: "" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: true, managed_profile: false },
    // Two-mode connector: one-click with access radios (read | write) OR a private-app token.
    { name: "hubspot", title: "HubSpot", icon: "⊚", blurb: "Search CRM records; log notes and tasks, update records. No deletes.", auth: "token", two_way: false, channels: false, available: true, brand_color: "#ff7a59", logo: "hubspot", fields: [{ key: "token", label: "Private app token", secret: true, required: true, help: "", placeholder: "pat-…" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: true, managed_profile: false },
    // Generic multi-account connector (accounts.py layer): one-click OR integration token.
    { name: "notion", title: "Notion", icon: "◰", blurb: "Search pages, read content, query databases, create pages.", auth: "oauth", two_way: false, channels: false, available: true, brand_color: "#1f2328", logo: "", fields: [{ key: "access_token", label: "Integration secret", secret: true, required: true, help: "", placeholder: "ntn_…" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: true, managed_profile: false },
    // Managed email-keyed multi-account connector (outlook) — drives the onboarding tools gallery.
    { name: "outlook", title: "Outlook", icon: "◎", blurb: "Microsoft 365 mail and calendar: search, draft, and send email; manage events and respond to invites.", aliases: ["calendar", "email", "mail", "microsoft", "office"], auth: "oauth", two_way: false, channels: false, available: true, brand_color: "#0078d4", logo: "outlook", fields: [{ key: "access_token", label: "OAuth access token", secret: true, required: true, help: "", placeholder: "" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: true, managed_profile: false },
    // Sixth active card in the onboarding gallery (promoted 2026-07-19 to even the grid).
    { name: "attio", title: "Attio", icon: "▣", blurb: "Search and read Attio CRM records; log notes.", auth: "oauth", two_way: false, channels: false, available: true, brand_color: "#2d6ae0", logo: "attio", fields: [{ key: "access_token", label: "OAuth access token", secret: true, required: true, help: "", placeholder: "" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: true, managed_profile: false },
    // MCP-BACKED connectors (§42): vendor-hosted MCP + local OAuth, pinned tool subset.
    // monday is one-click ONLY (no manual fields); jira also has a manual token path
    // (two-mode modal). Neither needs cloud sign-in.
    { name: "monday", title: "monday.com", icon: "▦", blurb: "Read boards and items, track work, create items and post updates.", aliases: ["project management", "tasks", "boards"], auth: "oauth", two_way: false, channels: false, available: true, brand_color: "#6161ff", logo: "monday", mcp: true, fields: [], instructions: ["One click connects via monday.com sign-in in your browser.", "Sign-in is fully local — tokens stay on this computer."], connected: false, account: null, enabled: false, allowed_users: [], tools: [{ name: "mcp__monday__get_board_info", label: "Read board", kind: "read", description: "Read a board's columns and groups.", enabled: true, requires_approval: false }, { name: "mcp__monday__create_item", label: "Create item", kind: "write", description: "Create an item on a board.", enabled: true, requires_approval: true }], managed: false, managed_profile: false },
    { name: "jira", title: "Jira", icon: "◆", blurb: "Search, summarize, create, and update issues.", aliases: ["issues", "tickets", "atlassian"], auth: "api_token", two_way: false, channels: false, available: true, brand_color: "#0052cc", logo: "jira", mcp: true, fields: [{ key: "base_url", label: "Atlassian site URL", secret: false, required: true, help: "", placeholder: "" }, { key: "email", label: "Account email", secret: false, required: true, help: "", placeholder: "" }, { key: "api_token", label: "API token", secret: true, required: true, help: "", placeholder: "" }], instructions: [], connected: false, account: null, enabled: false, allowed_users: [], tools: [], managed: false, managed_profile: false },
  ],
};

// Two pending items across two personas: drives the Inbox kind tabs, the persona filter chips
// (which only render with >1 persona), and resolve-removes-card. The question's session is NOT in
// the sessions list on purpose — the Inbox must be self-contained (server-joined context fields).
export const INBOX_ITEMS = [
  {
    id: "inb-approval-1",
    session_id: "wp-3",
    kind: "approval",
    title: "Approve: run_shell",
    body: "rm -rf build/",
    state: "pending",
    resolution: null,
    inbox: "default",
    created_at: "2026-07-01 08:00:00",
    resolved_at: null,
    session_title: "Weekly plan 3",
    session_agent: "cowork",
    session_workspace: "",
    session_exists: true,
  },
  {
    id: "inb-question-1",
    session_id: "ops-1",
    kind: "question",
    title: "Which environment should I restart?",
    body: "",
    options: ["staging", "production"],
    allow_text: true,
    multi: false,
    state: "pending",
    resolution: null,
    inbox: "default",
    created_at: "2026-07-01 08:05:00",
    resolved_at: null,
    session_title: "Investigate alerts",
    session_agent: "ops",
    session_workspace: "",
    session_exists: true,
  },
];
