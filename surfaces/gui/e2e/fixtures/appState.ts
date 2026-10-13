import { PERSONAS, PINNED_SESSION, EXTRA_SESSIONS, OPS_SESSION, SLACK_SESSION, INBOX_ITEMS } from "./data";
import { CLOUD_STATE, CONNECTIONS, AUTOMATION, AUTOMATION_CLEAN, AUTOMATION_RUNS, PRIMARY_ROOT, PROVIDERS } from "./catalog";

/** The app's mutable state for one test: sessions, personas, inbox, roots, automations, skills. */
export function appState() {
  // Installed personas — mutable so enable/surface/delete round-trip through the UI.
  const personas: any[] = PERSONAS.personas.map((p) => ({ ...p }));
  // Sessions — mutable so archive (PATCH), rename (PATCH), and delete round-trip.
  const sessions: any[] = [
    { ...PINNED_SESSION },
    ...EXTRA_SESSIONS.map((s) => ({ ...s })),
    { ...OPS_SESSION },
    { ...SLACK_SESSION },
  ];
  // Inbox items + the outbound routing binding — mutable for resolve + the inline Slack config.
  const inbox: any[] = INBOX_ITEMS.map((i) => ({ ...i }));
  const routing: { name: string; channel: string | null; target: string } = {
    name: "default",
    channel: null,
    target: "",
  };
  // Session roots — the primary (writable, non-removable) scratch plus any added folders. Mutable so
  // the RO/RW add/toggle round-trips through the real UI. POST upserts by path (a toggle re-adds).
  const roots: any[] = [{ ...PRIMARY_ROOT }];
  // Session connections — PER-TEST copy so the Access section's mute toggle (POST) can flip
  // `enabled` without leaking into sibling tests.
  const connections = {
    connected: CONNECTIONS.connected.map((c) => ({ ...c })),
    recommended: CONNECTIONS.recommended.map((r) => ({ ...r })),
    attention: CONNECTIONS.attention,
  };
  // Providers — mutable so save (POST) flips `configured` and stamps key_set_at, matching the
  // backend's set_provider. verify (POST) never mutates: it's a live read-only credential check.
  const providers: any[] = PROVIDERS.map((p) => ({ ...p }));
  // Automations — mutable so Run now appends a run, enable/disable toggles, and delete removes.
  const automations: any[] = [{ ...AUTOMATION }, { ...AUTOMATION_CLEAN }];
  // MCP servers (empty by default; the granola OAuth quick-add test populates it).
  const mcpServers: any[] = [];
  const automationRuns: any[] = AUTOMATION_RUNS.map((r) => ({ ...r }));
  // Per-session unattended flag — mutable so the composer's "Send to Inbox" toggle persists and
  // the app reads it back (which is what gates parking approvals to the Inbox vs an inline card).
  const unattended: Record<string, boolean> = {};
  // Skills (SKILLS-SPEC) — mutable folder-is-truth mirror: the enabled flag, delete, install
  // from a link (preview → install), and the composer's per-session menu all round-trip
  // through this one list.
  const skills: any[] = [
    { name: "weekly-report", description: "Monday status report", instructions: "1. Collect updates\n2. Write it up", scope: "global", source: "local", enabled: true, path: "/state/skills/weekly-report", files: 0 },
    { name: "html-to-markdown", description: "Convert an HTML document or fragment to clean markdown.", instructions: "Convert the given HTML to markdown, preserving structure.", scope: "global", source: "uploaded", enabled: true, path: "/state/skills/html-to-markdown", files: 2 },
  ];

  // Fresh cloud sign-in state per test (module state outlives a page).
  Object.assign(CLOUD_STATE, {
    signed_in: false,
    account: "",
    user_id: "",
    telemetry_enabled: true,
  });


  return { personas, sessions, inbox, routing, roots, connections, providers, automations,
    mcpServers, automationRuns, unattended, skills };
}
