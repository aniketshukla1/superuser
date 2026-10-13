import type { Request } from "@playwright/test";
import { CONNECTORS } from "../data";
import { CLOUD_STATE } from "../catalog";
import type { Json, MockState } from "../mockApi";

/** GitHub, Gmail, Calendar, Notion and HubSpot accounts, and the connector list. */
export function connectorRoutes(s: MockState, req: Request, p: string, m: string, json: Json) {
  const { slackConnector, githubParked, githubState, githubConnector, gmailState, gmailConnector,
      gcalState, gcalConnector, notionState, notionConnector, mcpConnector, outlookConnector,
      hubspotState, hubspotConnector } = s;
  // GitHub relay (github-relay-spec §8): per-installation allow/disallow, parked
  // resolution, status, per-installation disconnect.
  if (/\/v1\/connectors\/github\/unauthorized\/[^/]+$/.test(p) && m === "POST") {
    const id = p.split("/").pop();
    const i = githubParked.findIndex((x) => x.id === id);
    if (i < 0) return json({ ok: false, error: "unknown item" });
    const b = req.postDataJSON();
    const item = githubParked.splice(i, 1)[0];
    if (b.action === "allow" || b.action === "allow_deliver") {
      const pool = githubState.installations.find(
        (x) => x.installation_id === item.team_id,
      )?.allowed_users;
      if (pool && !pool.includes(item.user_id)) pool.push(item.user_id);
    }
    return json({ ok: true });
  }
  if (/\/v1\/connectors\/github\/(allow|disallow)$/.test(p) && m === "POST") {
    const b = req.postDataJSON();
    const pool = githubState.installations.find(
      (x) => x.installation_id === b.team_id,
    )?.allowed_users;
    if (!pool) return json({ ok: false, error: "installation not connected" });
    const add = p.endsWith("/allow");
    const i = pool.indexOf(b.user_id);
    if (add && i < 0) pool.push(b.user_id);
    if (!add && i >= 0) pool.splice(i, 1);
    return json({ ok: true, allowed_users: [...pool], team_id: b.team_id ?? null });
  }
  if (p.endsWith("/v1/connectors/github/status"))
    return json({
      ok: true,
      mode: githubState.mode,
      relay: { state: "live", reconnects: 0, last_event_at: Date.now() / 1000 - 30, last_error: "" },
      signed_in: CLOUD_STATE.signed_in,
      installs: Object.fromEntries(
        githubState.installations.map((x) => [x.installation_id, { token_ok: true }]),
      ),
      missed: {},
    });
  if (/\/v1\/connectors\/github\/installations\/[^/]+\/disconnect$/.test(p) && m === "POST") {
    const iid = decodeURIComponent(p.split("/").slice(-2)[0]);
    const i = githubState.installations.findIndex((x) => x.installation_id === iid);
    if (i < 0) return json({ ok: false, error: "installation not connected" });
    githubState.installations.splice(i, 1);
    if (githubState.installations.length === 0) {
      githubState.connected = false;
      githubState.mode = "";
    }
    return json({ ok: true, remaining_installs: githubState.installations.length });
  }
  // Gmail multi-account management (M3.6 Step 3): per-account disconnect/default
  // + the "Never show agents" filter lists.
  if (/\/v1\/connectors\/gmail\/accounts\/[^/]+\/disconnect$/.test(p) && m === "POST") {
    const email = decodeURIComponent(p.split("/").slice(-2)[0]);
    const i = gmailState.accounts.findIndex((a) => a.email === email);
    if (i < 0) return json({ ok: false, error: "account not connected" });
    const wasDefault = gmailState.accounts[i].default;
    gmailState.accounts.splice(i, 1);
    if (wasDefault && gmailState.accounts[0]) gmailState.accounts[0].default = true;
    return json({ ok: true, remaining_accounts: gmailState.accounts.length });
  }
  if (/\/v1\/connectors\/google_calendar\/accounts\/[^/]+\/disconnect$/.test(p) && m === "POST") {
    const email = decodeURIComponent(p.split("/accounts/")[1].split("/")[0]);
    const i = gcalState.accounts.findIndex((a) => a.email === email);
    if (i < 0) return json({ ok: false, error: "account not connected" });
    const wasDefault = gcalState.accounts[i].default;
    gcalState.accounts.splice(i, 1);
    if (wasDefault && gcalState.accounts[0]) gcalState.accounts[0].default = true;
    return json({ ok: true, remaining_accounts: gcalState.accounts.length });
  }
  if (/\/v1\/connectors\/google_calendar\/accounts\/[^/]+\/default$/.test(p) && m === "POST") {
    const email = decodeURIComponent(p.split("/accounts/")[1].split("/")[0]);
    if (!gcalState.accounts.some((a) => a.email === email))
      return json({ ok: false, error: "account not connected" });
    for (const a of gcalState.accounts) a.default = a.email === email;
    return json({ ok: true, default_account: email });
  }
  if (/\/v1\/connectors\/gmail\/accounts\/[^/]+\/default$/.test(p) && m === "POST") {
    const email = decodeURIComponent(p.split("/").slice(-2)[0]);
    if (!gmailState.accounts.some((a) => a.email === email))
      return json({ ok: false, error: "account not connected" });
    for (const a of gmailState.accounts) a.default = a.email === email;
    return json({ ok: true, default_account: email });
  }
  if (p.endsWith("/v1/connectors/gmail/filters") && m === "PATCH") {
    const b = req.postDataJSON() || {};
    if (Array.isArray(b.senders)) gmailState.filters.senders = b.senders;
    if (Array.isArray(b.labels)) gmailState.filters.labels = b.labels;
    return json({ ok: true, filters: { ...gmailState.filters } });
  }
  // Generic multi-account management (accounts.py layer; notion in fixtures).
  if (/\/v1\/connectors\/notion\/accounts\/[^/]+\/disconnect$/.test(p) && m === "POST") {
    const id = decodeURIComponent(p.split("/accounts/")[1].split("/")[0]);
    const i = notionState.accounts.findIndex((a) => a.account_id === id);
    if (i < 0) return json({ ok: false, error: "account not connected" });
    const wasDefault = notionState.accounts[i].default;
    notionState.accounts.splice(i, 1);
    if (wasDefault && notionState.accounts[0]) notionState.accounts[0].default = true;
    return json({ ok: true, remaining_accounts: notionState.accounts.length });
  }
  if (/\/v1\/connectors\/notion\/accounts\/[^/]+\/default$/.test(p) && m === "POST") {
    const id = decodeURIComponent(p.split("/accounts/")[1].split("/")[0]);
    if (!notionState.accounts.some((a) => a.account_id === id))
      return json({ ok: false, error: "account not connected" });
    for (const a of notionState.accounts) a.default = a.account_id === id;
    return json({ ok: true, default_account: id });
  }
  // HubSpot multi-portal management (M3.6 Step 4).
  if (/\/v1\/connectors\/hubspot\/portals\/[^/]+\/disconnect$/.test(p) && m === "POST") {
    const hub = decodeURIComponent(p.split("/").slice(-2)[0]);
    const i = hubspotState.portals.findIndex((x) => x.hub_id === hub);
    if (i < 0) return json({ ok: false, error: "portal not connected" });
    const wasDefault = hubspotState.portals[i].default;
    hubspotState.portals.splice(i, 1);
    if (wasDefault && hubspotState.portals[0]) hubspotState.portals[0].default = true;
    return json({ ok: true, remaining_portals: hubspotState.portals.length });
  }
  if (/\/v1\/connectors\/hubspot\/portals\/[^/]+\/default$/.test(p) && m === "POST") {
    const hub = decodeURIComponent(p.split("/").slice(-2)[0]);
    if (!hubspotState.portals.some((x) => x.hub_id === hub))
      return json({ ok: false, error: "portal not connected" });
    for (const x of hubspotState.portals) x.default = x.hub_id === hub;
    return json({ ok: true, default_portal: hub });
  }
  if (p.endsWith("/v1/connectors/hubspot/hidden-fields") && m === "PATCH") {
    const b = req.postDataJSON() || {};
    if (Array.isArray(b.hidden_fields))
      hubspotState.hidden_fields = b.hidden_fields.map((f: string) => f.trim().toLowerCase());
    return json({ ok: true, hidden_fields: [...hubspotState.hidden_fields] });
  }
  if (p.endsWith("/v1/connectors"))
    return json({
      connectors: [
        slackConnector(),
        githubConnector(),
        ...CONNECTORS.connectors.map((c: any) =>
          c.name === "gmail"
            ? gmailConnector()
            : c.name === "google_calendar"
              ? gcalConnector()
              : c.name === "hubspot"
                ? hubspotConnector()
                : c.name === "notion"
                  ? notionConnector()
                  : c.name === "outlook"
                    ? outlookConnector()
                    : c.name === "monday" || c.name === "jira"
                      ? mcpConnector(c.name)
                      : { ...c },
        ),
      ],
    });
}
