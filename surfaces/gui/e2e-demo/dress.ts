import type { Page } from "@playwright/test";
import {
  AUTOMATIONS,
  BOUNDARIES,
  CONNECTED,
  EGRESS,
  HERO,
  HERO_MESSAGES,
  INBOX,
  MEMORIES,
  MODEL,
  MODEL_LABELS,
  PERSONAS,
  PROMETHEUS_TOOLS,
  SESSIONS,
  SETTINGS_SECTIONS,
  SYNC_TOOLS,
} from "./demoData";

// The demo backdrop shared by the screenshot run (demo.spec.ts) and the video (video/demoVideo.spec.ts):
// the hermetic e2e mock underneath, with every screen's GET answered from demoData on top.
export const API = "http://127.0.0.1:8765";

/** What the underlying mock answers for a path, to build a demo payload on top of. */
export const base = (page: Page, p: string) => page.evaluate((u) => fetch(u).then((r) => r.json()), `${API}${p}`);

/** Answer GETs for a path with demo data; everything else still reaches the mock. */
export async function serve(page: Page, p: string, body: unknown) {
  const pattern = new RegExp(p.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "(\\?.*)?$");
  await page.route(pattern, (route) =>
    route.request().method() === "GET" ? route.fulfill({ json: body }) : route.fallback(),
  );
}

export interface DressOptions {
  /** The sidebar's conversations, when a recording needs a different set. */
  sessions?: unknown[];
  /** What Privacy ▸ Network shows. */
  egress?: unknown;
}

/** Lay the demo data over the mock: every GET the screens make answers from demoData. */
export async function dress(page: Page, opts: DressOptions = {}) {
  const settings = await base(page, "/v1/settings");
  await serve(page, "/v1/settings", {
    ...settings,
    provider: "ollama",
    model: MODEL,
    models: Object.keys(MODEL_LABELS),
    model_labels: MODEL_LABELS,
    model_context_windows: { [MODEL]: 131_000 },
  });
  const health = await base(page, "/v1/health");
  await serve(page, "/v1/health", { ...health, model: MODEL });
  await serve(page, "/v1/personas", { personas: PERSONAS });
  await serve(page, "/v1/sessions", { sessions: opts.sessions ?? SESSIONS });
  await serve(page, `/v1/sessions/${HERO}/messages`, { messages: HERO_MESSAGES });
  // The Inbox page asks for everything; a conversation asks only for what is waiting on it.
  await page.route(/\/v1\/inbox(\?.*)?$/, (route) => {
    if (route.request().method() !== "GET") return route.fallback();
    const session = new URL(route.request().url()).searchParams.get("session_id");
    return route.fulfill({ json: { items: session ? INBOX.filter((i) => i.session_id === session) : INBOX } });
  });
  await serve(page, "/v1/feedback", { reasons: [], given: [] });
  await serve(page, "/v1/boundaries", BOUNDARIES);
  await serve(page, "/v1/automations", { tasks: AUTOMATIONS });
  await serve(page, "/v1/memory", { memories: MEMORIES });
  await serve(page, `/v1/memory/${MEMORIES[0].filename}`, {
    ok: true,
    memory: {
      ...MEMORIES[0],
      body: "Merged into checkout 2.41.0 by maria. Adds a synchronous call to the tax service on the confirm path.\n\n- Reviewers asked for a timeout; one of 800 ms was added.\n- No load test was attached.\n\n[Open on GitHub](https://github.com/northwind/checkout/pull/812)",
    },
  });
  await serve(page, "/v1/egress", opts.egress ?? EGRESS);
  for (const [p, body] of Object.entries(SETTINGS_SECTIONS)) await serve(page, p, body);

  // Connectors: the mock's catalogue, with a demo set connected and the tools each answers to.
  const catalogue = await base(page, "/v1/connectors");
  const list = (catalogue.connectors ?? catalogue) as any[];
  const template = list.find((c) => c.name === "github") ?? list[0];
  const want = ["prometheus", "github", "email", "jira", "linear"];
  const titles: Record<string, [string, string]> = {
    prometheus: ["Prometheus", "Query metrics live — latency, errors, saturation — to investigate a slowdown."],
    email: ["Email (IMAP)", "Read and send from any IMAP/SMTP mailbox."],
    linear: ["Linear", "Issues and project state."],
  };
  const connectors = [
    ...want.map((name) => {
      const found = list.find((c) => c.name === name) ?? { ...template, name, icon: name, logo: name, title: titles[name]?.[0] ?? name, blurb: titles[name]?.[1] ?? "" };
      return {
        ...found,
        title: titles[name]?.[0] ?? found.title,
        blurb: titles[name]?.[1] ?? found.blurb,
        auth: "token",
        managed: false,
        mcp: false,
        two_way: false,
        channels: false,
        connected: true,
        enabled: true,
        available: true,
        account: CONNECTED[name],
        workspaces: undefined,
        installations: undefined,
        accounts: undefined,
        tools: name === "prometheus" ? PROMETHEUS_TOOLS : SYNC_TOOLS,
      };
    }),
    ...list.filter((c) => !want.includes(c.name)).map((c) => ({ ...c, connected: false, account: null })),
  ];
  await serve(page, "/v1/connectors", { connectors });
}
