import { vi } from "vitest";

// A hermetic fetch stub routing by URL substring + method. Records calls so tests can assert POSTs.
export type Call = { url: string; method: string; body: any };

export function stubFetch(routes: { match: string; method?: string; json: any }[]) {
  const calls: Call[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const method = (init?.method || "GET").toUpperCase();
    calls.push({ url, method, body: init?.body ? JSON.parse(String(init.body)) : undefined });
    for (const r of routes) {
      if (url.includes(r.match) && (!r.method || r.method === method)) {
        return { ok: true, json: async () => r.json } as Response;
      }
    }
    return { ok: true, json: async () => ({}) } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

export const DETAIL = {
  id: "ops",
  name: "Ops Assistant",
  icon: "🛠️",
  tagline: "Incidents and runbooks",
  description: "A careful, methodical operations engineer.",
  enabled: true,
  tools: ["files", "search", "shell"],
  needs_capable_model: true,
  default_permission_mode: "interactive",
  workspace: "deliverable",
  recommends: [
    { kind: "connector", ref: "github", reason: "confirm deploys", tier: "core", connected: true },
    { kind: "connector", ref: "datadog", reason: "pull alerts", tier: "core", connected: false },
    { kind: "mcp", ref: "filesystem", reason: "read runbooks", tier: "optional", connected: false },
  ],
  default_connections: [
    { connector: "slack", enabled: true, connected: true },
    { connector: "datadog", enabled: false, connected: false },
  ],
};

export const CONNECTORS = {
  connectors: [
    { name: "github", title: "GitHub", logo: "github", brand_color: "#1f2328" },
    { name: "slack", title: "Slack", logo: "slack", brand_color: "#611f69" },
    { name: "datadog", title: "Datadog", logo: "datadog", brand_color: "#632ca6" },
  ],
};
