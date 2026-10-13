
// What the mock API serves: cloud, gallery, persona detail, connections, automations, providers.

// Mutable cloud sign-in state: POST /v1/cloud/login flips it (the real flow
// goes through the browser; the mock completes instantly), logout flips back.
export const CLOUD_STATE = {
  signed_in: false,
  account: "",
  user_id: "",
  telemetry_enabled: true,
};

export const GALLERY_PERSONAS = [
  {
    slug: "sales",
    version: 1,
    name: "Sales Coworker",
    icon: "chart",
    tagline: "Research accounts, prep meetings, draft follow-ups",
    description: "A sales-focused coworker.",
    family: "knowledge",
    workspace: "deliverable",
    publisher: "Superuser",
    recommended_connectors: ["hubspot", "gmail"],
    risk_summary: "Declarative manifest; no executable code.",
    featured: true,
  },
  {
    slug: "recruiter",
    version: 1,
    name: "Recruiter",
    icon: "search",
    tagline: "Sourcing summaries and scheduling loops",
    description: "A recruiting coworker.",
    family: "knowledge",
    workspace: "deliverable",
    publisher: "Superuser",
    recommended_connectors: ["gmail"],
    risk_summary: "Declarative manifest; no executable code.",
    featured: false,
  },
];

// Persona detail (GET /v1/personas/:id) — SourcesDrawer/PersonaView read `recommends` and
// `default_connections` as arrays, so these must be present (not the catch-all {}).
export const PERSONA_DETAIL = {
  id: "cowork",
  name: "Assistant",
  icon: "cowork",
  tagline: "Produce a deliverable — research, analysis, scripts",
  description: "",
  enabled: true,
  tools: ["files", "search"],
  recommended_models: ["anthropic:claude-opus-4-8"],
  default_permission_mode: "interactive",
  workspace: "deliverable",
  recommends: [],
  default_connections: [],
};

export const CONNECTIONS = {
  connected: [
    { connector: "browser", enabled: true, detail: "Browser", sendable: false },
    { connector: "slack", enabled: true, detail: "Slack", sendable: true, scope: "write" },
    // two_way WITHOUT channels (relay mentions, no subscriptions) — pins the
    // "GitHub shows Channels" regression (owner report 2026-07-13).
    //
    // `sendable: false` pins the second half of that story: GitHub is catalogued
    // two_way, but nothing can be SENT through it (`senders::send` has no github
    // arm), so it must not offer a Read/Write choice either. The gate keys on the
    // send path, not on the catalogue flag.
    { connector: "github", enabled: true, detail: "GitHub", sendable: false },
  ],
  recommended: [
    { connector: "gmail", reason: "email context for morning summaries", tier: "core", connected: false },
  ],
  attention: 1,
};

// One scheduled automation with a running run: its session id uses the real `__run__` convention
// so the session view's run banner (detection is id-based) can be exercised end-to-end.
export const AUTOMATION = {
  id: "task-1",
  title: "Daily AI News",
  instructions: "Fetch the latest AI news and produce an HTML+Tailwind presentation.",
  schedule: "Every day at ~5:40 PM",
  schedule_raw: { kind: "cron", cron: "40 17 * * *", fire_at: null, timezone: "local" },
  workspace: "",
  agent: "cowork",
  enabled: true,
  next_run: Math.floor(Date.now() / 1000) + 3600,
  last_run: Math.floor(Date.now() / 1000) - 60,
  last_status: "running",
  run_count: 1,
  notify_on_completion: false,
  // One standing scoped approval (§25) so the detail page's revoke list has content.
  always_allowed: [
    { entry: "send_message slack:T1/C1", tool: "send_message", target: "slack:T1/C1" },
  ],
  // UX-023 sidebar badges: two unopened runs, the newest of them failed.
  unseen_runs: 2,
  unseen_failed: true,
  seen_runs_at: 0,
};
// A second, quiet automation so the Scheduled band shows badge-less rows too.
export const AUTOMATION_CLEAN = {
  ...AUTOMATION,
  id: "task-2",
  title: "Weekly CRM digest",
  schedule: "Every Monday at ~9:00 AM",
  last_status: "ok",
  unseen_runs: 0,
  unseen_failed: false,
  always_allowed: [],
};
export const AUTOMATION_RUNS = [
  {
    run_id: "r1",
    task_id: "task-1",
    session_id: "__run__r1",
    started_at: Math.floor(Date.now() / 1000) - 60,
    finished_at: null,
    status: "running",
    result_text: null,
    artifacts: [],
    error: null,
    trigger: "schedule",
  },
];

export const PRIMARY_ROOT = { path: "/Users/test/Superuser/launch-note", writable: true, label: "scratch", primary: true, exists: true };
export const baseName = (p: string) => p.split("/").filter(Boolean).pop() || p;

export const PROVIDERS = [
  // openai: configured + used (drives the "Last used" sub-line and the status dot).
  { name: "openai", title: "OpenAI", needs_key: true, fields: [{ key: "api_key", label: "OpenAI API key", secret: true, required: true, help: "", placeholder: "sk-…" }], configured: true, values: {}, suggested_models: ["gpt-5.5"], key_set_at: "2026-06-12", last_used_at: Math.floor(Date.now() / 1000) - 7200 },
  // anthropic: configured but never used ("Not used yet").
  { name: "anthropic", title: "Claude (Anthropic)", needs_key: true, fields: [{ key: "api_key", label: "API key", secret: true, required: true, help: "", placeholder: "sk-…" }], configured: true, values: {}, suggested_models: ["claude-opus-4-8"], key_set_at: null, last_used_at: null },
  // zai: an OpenAI-compatible vendor — unconfigured, with a prefilled editable endpoint + blurb.
  { name: "zai", title: "Z AI (GLM)", needs_key: true, blurb: "Uses Z AI's OpenAI-compatible API — the endpoint is prefilled, just add your key.", fields: [{ key: "api_key", label: "Z AI API key", secret: true, required: true, help: "", placeholder: "" }, { key: "base_url", label: "Endpoint", secret: false, required: false, help: "Prefilled with Z AI's international endpoint.", placeholder: "https://api.z.ai/api/paas/v4", default: "https://api.z.ai/api/paas/v4" }], configured: false, values: {}, suggested_models: ["glm-5.2"], key_set_at: null, last_used_at: null },
  // ollama: keyless local provider — "configured" without proving anything runs; the
  // onboarding gallery shows "No key needed" and its form is endpoint + Detect (§39).
  { name: "ollama", title: "Ollama (local models)", needs_key: false, fields: [{ key: "base_url", label: "Endpoint", secret: false, required: false, help: "", placeholder: "http://127.0.0.1:11434", default: "http://127.0.0.1:11434" }], configured: true, values: {}, suggested_models: ["qwen3-coder:30b"], key_set_at: null, last_used_at: null },
];
