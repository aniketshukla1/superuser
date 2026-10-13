import type { Request } from "@playwright/test";
import { CLOUD_STATE, PERSONA_DETAIL } from "../catalog";
import type { Json, MockState } from "../mockApi";

/** Personas, the session list, and the Slack relay. */
export function personaRoutes(s: MockState, req: Request, p: string, m: string, json: Json) {
  const { parked, slackState, personas, sessions } = s;
  // must precede the /v1/personas/{id} catch-all (install matches it too)
  if (p.endsWith("/v1/personas/install") && m === "POST") {
    const b = req.postDataJSON();
    if (b.gallery_slug) {
      return json(
        CLOUD_STATE.signed_in
          ? { ok: true, consent: [{ id: b.gallery_slug }], personas }
          : { ok: false, error: "gallery requires cloud sign-in" },
      );
    }
    return json({ ok: false, error: "unsupported in mock" });
  }
  if (/\/v1\/personas\/[^/]+$/.test(p) && m === "POST") {
    // Persona flag update (enabled/surfaced/default). Backend parity: enabling implies
    // surfacing (registry.set_enabled sets surfaced — the PM-invisible bug fix).
    const id = p.split("/").pop();
    const t = personas.find((x) => x.id === id);
    if (!t) return json({ ok: false, error: `unknown persona: ${id}` });
    const b = req.postDataJSON();
    if (b.default) personas.forEach((x) => (x.default = x.id === id));
    let archivedCount = 0;
    if (typeof b.enabled === "boolean") {
      t.enabled = b.enabled;
      if (b.enabled) t.surfaced = true;
      // Backend parity (disable-archives, §18): disabling archives the persona's real
      // sessions server-side, so its sidebar section disappears with it.
      if (!b.enabled) {
        for (const s of sessions) {
          if (s.agent === id && !s.archived && !s.session_id.startsWith("__")) {
            s.archived = true;
            archivedCount++;
          }
        }
      }
    }
    if (typeof b.surfaced === "boolean") t.surfaced = b.surfaced;
    return json({ ok: true, personas, archived_sessions: archivedCount });
  }
  if (/\/v1\/personas\/[^/]+$/.test(p) && m === "DELETE") {
    const id = p.split("/").pop();
    const i = personas.findIndex((x) => x.id === id && !x.builtin);
    if (i < 0) return json({ ok: false, error: `unknown persona: ${id}` });
    personas.splice(i, 1);
    return json({ ok: true, personas });
  }
  if (/\/v1\/personas\/[^/]+$/.test(p)) return json(PERSONA_DETAIL);
  if (p.endsWith("/v1/personas")) return json({ personas });
  if (p.endsWith("/v1/sessions")) return json({ sessions });
  if (/\/v1\/connectors\/slack\/unauthorized\/[^/]+$/.test(p) && m === "POST") {
    const id = p.split("/").pop();
    const i = parked.findIndex((x) => x.id === id);
    if (i < 0) return json({ ok: false, error: "unknown item" });
    const b = req.postDataJSON();
    const item = parked.splice(i, 1)[0];
    // Backend parity: allowing routes to the item's OWN workspace's list (ids are
    // workspace-scoped); a team-less item lands on the flat list (manual mode).
    if (b.action === "allow" || b.action === "allow_deliver") {
      const pool = item.team_id
        ? slackState.workspaces.find((w) => w.team_id === item.team_id)?.allowed_users
        : slackState.allowed_users;
      if (pool && !pool.includes(item.user_id)) pool.push(item.user_id);
    }
    return json({ ok: true });
  }
  // Per-workspace allow/disallow (team_id in the body) + the flat manual list without it.
  if (/\/v1\/connectors\/slack\/(allow|disallow)$/.test(p) && m === "POST") {
    const b = req.postDataJSON();
    const add = p.endsWith("/allow");
    const ws = b.team_id
      ? slackState.workspaces.find((w) => w.team_id === b.team_id)
      : null;
    const pool = b.team_id ? ws?.allowed_users : slackState.allowed_users;
    if (!pool) return json({ ok: false, error: "workspace not connected" });
    const i = pool.indexOf(b.user_id);
    if (add && i < 0) pool.push(b.user_id);
    if (!add && i >= 0) pool.splice(i, 1);
    // Directory picks carry the display name — backend seeds the people directory.
    if (add && b.name && ws) ws.allowed_user_names[b.user_id] = b.name;
    return json({ ok: true, allowed_users: [...pool], team_id: b.team_id ?? null });
  }
  if (p.endsWith("/v1/connectors/slack/approval-owners/add") && m === "POST") {
    const b = req.postDataJSON();
    if (!slackState.approval_owner_ids.includes(b.user_id))
      slackState.approval_owner_ids.push(b.user_id);
    if (!slackState.allowed_users.includes(b.user_id))
      slackState.allowed_users.push(b.user_id);
    return json({
      ok: true,
      approval_owner_ids: [...slackState.approval_owner_ids],
      allowed_users: [...slackState.allowed_users],
    });
  }
  if (p.endsWith("/v1/connectors/slack/approval-owners/remove") && m === "POST") {
    const b = req.postDataJSON();
    const i = slackState.approval_owner_ids.indexOf(b.user_id);
    if (i >= 0) slackState.approval_owner_ids.splice(i, 1);
    return json({ ok: true, approval_owner_ids: [...slackState.approval_owner_ids] });
  }
  // Workspace rosters for the pickers (users.list / conversations.list, mocked).
  if (/\/v1\/connectors\/slack\/workspaces\/[^/]+\/directory$/.test(p) && m === "GET") {
    const q = (new URL(req.url()).searchParams.get("q") || "").toLowerCase();
    const members = [
      { id: "U9MAYA", name: "Maya Chen", handle: "maya", guest: false },
      { id: "U8ROHIT", name: "Rohit Prasad", handle: "rohit", guest: false },
      { id: "U7CAL", name: "Contractor Cal", handle: "cal", guest: true },
    ].filter((mem) => !q || mem.name.toLowerCase().includes(q) || mem.handle.includes(q));
    return json({ ok: true, members });
  }
  if (/\/v1\/connectors\/slack\/workspaces\/[^/]+\/channels$/.test(p) && m === "GET") {
    const team = decodeURIComponent(p.split("/workspaces/")[1].split("/")[0]);
    const q = (new URL(req.url()).searchParams.get("q") || "").toLowerCase();
    const channels = [
      { id: "C9LAUNCH", name: "launch-team", is_private: false, is_member: true },
      { id: "C8LEADS", name: "leads", is_private: true, is_member: true },
      { id: "C7LOBBY", name: "lobby", is_private: false, is_member: false },
    ].filter((c) => !q || c.name.includes(q));
    return json({ ok: true, channels, team });
  }
  // Slack health, three layers (M3.6 Step 2): socket live + all tokens good by
  // default; sign-in mirrors CLOUD_STATE. Specs force reconnecting/offline/dead
  // tokens by registering a later page.route override (later routes match first).
  if (p.endsWith("/v1/connectors/slack/status"))
    return json({
      ok: true,
      mode: slackState.mode,
      relay: { state: "live", reconnects: 0, last_event_at: Date.now() / 1000 - 30, last_error: "" },
      signed_in: CLOUD_STATE.signed_in,
      teams: Object.fromEntries(
        slackState.workspaces.map((w) => [w.team_id, { token_ok: true }]),
      ),
    });
  // Stop relaying one workspace; removing the last flips the connector off (backend parity).
  if (/\/v1\/connectors\/slack\/workspaces\/[^/]+\/disconnect$/.test(p) && m === "POST") {
    const teamId = decodeURIComponent(p.split("/").slice(-2)[0]);
    const i = slackState.workspaces.findIndex((w) => w.team_id === teamId);
    if (i < 0) return json({ ok: false, error: "workspace not connected" });
    slackState.workspaces.splice(i, 1);
    if (slackState.workspaces.length === 0) {
      slackState.connected = false;
      slackState.mode = "";
    }
    return json({ ok: true, remaining_workspaces: slackState.workspaces.length });
  }
}
