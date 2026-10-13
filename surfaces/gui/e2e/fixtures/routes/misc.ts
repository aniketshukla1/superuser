import type { Request } from "@playwright/test";
import type { Json, MockState } from "../mockApi";

/** Onboarding, MCP servers, unrouted and channel subscriptions. */
export function miscRoutes(s: MockState, req: Request, p: string, m: string, json: Json) {
  const { subscriptions, mcpServers } = s;
  if (p.endsWith("/v1/settings/onboarded") && m === "POST") {
    return json({ ok: true, onboarded: !!(req.postDataJSON() || {}).value });
  }
  // MCP servers — mutable so the OAuth quick-add (granola) flow reflects through the
  // UI: add → needs_auth, connect → authorizing, next poll → connected (6 tools).
  if (p.endsWith("/v1/mcp") && m === "GET") {
    for (const s2 of mcpServers) {
      if (s2.status === "authorizing" && s2._flip) {
        s2.status = "connected";
        s2.tool_count = 6;
      }
      if (s2.status === "authorizing") s2._flip = true;
    }
    return json({ servers: mcpServers.map(({ _flip, ...s2 }) => s2) });
  }
  if (p.endsWith("/v1/mcp") && m === "POST") {
    const b = req.postDataJSON();
    mcpServers.push({
      name: b.name,
      enabled: true,
      transport: b.config?.url ? "http" : "stdio",
      requires_approval: true,
      auth: b.config?.auth === "oauth" ? "oauth" : null,
      status: b.config?.auth === "oauth" ? "needs_auth" : "configured",
      last_error: null,
      tool_count: null,
      config: b.config || {},
    });
    return json({ ok: true, name: b.name });
  }
  {
    const mc = p.match(/\/v1\/mcp\/([^/]+)\/connect$/);
    if (mc && m === "POST") {
      const s2 = mcpServers.find((x) => x.name === decodeURIComponent(mc[1]));
      if (s2) s2.status = "authorizing";
      return json({ ok: true, started: true });
    }
    const ms = p.match(/\/v1\/mcp\/([^/]+)\/signout$/);
    if (ms && m === "POST") {
      const s2 = mcpServers.find((x) => x.name === decodeURIComponent(ms[1]));
      if (s2) {
        s2.status = "needs_auth";
        s2.tool_count = null;
        s2._flip = false;
      }
      return json({ ok: true });
    }
  }
  if (p.endsWith("/v1/unrouted")) return json([]);

  // channel subscriptions — mutable so add/remove reflect through the UI
  if (p.endsWith("/v1/subscriptions") && m === "GET") return json({ subscriptions });
  if (p.endsWith("/v1/subscriptions") && m === "POST") {
    const b = req.postDataJSON();
    // Backend parity with resolve_channel: Copy-link URLs resolve to the id; bare #names
    // can't be looked up and are rejected with the same hint the server gives.
    const raw = String(b.channel || "").trim();
    if (raw.startsWith("#"))
      return json({
        ok: false,
        error:
          "Channel names can't be looked up — paste the channel ID (channel name ▸ About) or the channel's Copy-link URL.",
      });
    const link = raw.match(/slack\.com\/archives\/([A-Za-z0-9]+)/);
    const channel = link ? `slack:${link[1].toUpperCase()}` : raw;
    subscriptions.push({ session_id: b.session_id, session_title: "", agent: "", channel, routing_target: null, collision: false });
    return json({ ok: true, channel });
  }
  if (p.endsWith("/v1/subscriptions/remove") && m === "POST") {
    const b = req.postDataJSON();
    const i = subscriptions.findIndex((s) => s.session_id === b.session_id && s.channel === b.channel);
    if (i >= 0) subscriptions.splice(i, 1);
    return json({ ok: true });
  }
}
