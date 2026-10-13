import type { Request } from "@playwright/test";
import { CLOUD_STATE, GALLERY_PERSONAS } from "../catalog";
import type { Json, MockState } from "../mockApi";

/** Superuser Cloud, MCP and managed connects, and the gallery. */
export function cloudRoutes(s: MockState, req: Request, p: string, m: string, json: Json) {
  const { slackState, githubState, gmailState, GMAIL_NEXT, gcalState, GCAL_NEXT, notionState,
      NOTION_NEXT, outlookState, mcpState, hubspotState, HUBSPOT_NEXT } = s;
  if (p.endsWith("/v1/cloud/status")) return json({ ...CLOUD_STATE });
  if (p.endsWith("/v1/cloud/login") && m === "POST") {
    Object.assign(CLOUD_STATE, { signed_in: true, account: "rohit@example.com", user_id: "usr_e2e" });
    return json({ ok: true });
  }
  if (p.endsWith("/v1/cloud/telemetry") && m === "POST") {
    CLOUD_STATE.telemetry_enabled = !!req.postDataJSON().enabled;
    return json({ ok: true, telemetry_enabled: CLOUD_STATE.telemetry_enabled });
  }
  if (p.endsWith("/v1/cloud/logout") && m === "POST") {
    Object.assign(CLOUD_STATE, { signed_in: false, account: "", user_id: "" });
    return json({ ok: true, signed_in: false });
  }
  if (/\/v1\/connectors\/[^/]+\/mcp-connect$/.test(p) && m === "POST") {
    // Local MCP OAuth flow — no cloud sign-in required; completes instantly here.
    const name = p.match(/\/v1\/connectors\/([^/]+)\/mcp-connect$/)?.[1] as
      | "monday"
      | "jira";
    if (name in mcpState) {
      mcpState[name] = true;
      return json({ ok: true, started: true });
    }
    return json({ ok: false, error: `${name} has no MCP connect path` });
  }
  if (/\/v1\/connectors\/[^/]+\/connect-managed$/.test(p) && m === "POST") {
    if (!CLOUD_STATE.signed_in) return json({ ok: false, error: "not signed in" });
    // Slack managed install = add a workspace. The real flow completes in the system
    // browser; the mock installs instantly so the page's poll picks it up.
    if (p.includes("/connectors/slack/")) {
      slackState.workspaces.push({ team_id: "T3NEW", account: "new-workspace", domain: "new-workspace", allowed_users: ["U_ME"], allow_all: false, allowed_user_names: { U_ME: "Rohit Prasad" }, approval_owner_ids: ["U_ME"], approval_owner_names: { U_ME: "Rohit Prasad" }, installer_user_id: "U_ME", installer_name: "Rohit Prasad" });
      slackState.connected = true;
      slackState.mode = "relay";
    }
    // GitHub managed connect = install on the next account (instant, like Slack).
    if (p.includes("/connectors/github/")) {
      githubState.installations.push({
        installation_id: "202", account_login: "hooli", account_type: "Organization",
        repo_selection: "all", github_login: "rohit-dev", allowed_users: ["rohit-dev"], allow_all: false,
      });
      githubState.connected = true;
      githubState.mode = "relay";
    }
    // Gmail managed connect = add the next mailbox; the first becomes default.
    if (p.includes("/connectors/gmail/")) {
      const email = GMAIL_NEXT[gmailState.accounts.length] || `acct${gmailState.accounts.length}@x.com`;
      gmailState.accounts.push({
        email, default: gmailState.accounts.length === 0, managed: true,
        scopes: "gmail.readonly gmail.send", needs_reauth: false,
      });
    }
    // Google Calendar managed connect = add the next account (gmail's flow).
    if (p.includes("/connectors/google_calendar/")) {
      const email = GCAL_NEXT[gcalState.accounts.length] || `acct${gcalState.accounts.length}@x.com`;
      gcalState.accounts.push({
        email, default: gcalState.accounts.length === 0, managed: true,
        scopes: "calendar", needs_reauth: false,
      });
    }
    // Outlook managed connect = add the next mailbox (email-keyed accounts).
    if (p.includes("/connectors/outlook/")) {
      outlookState.accounts.push({
        account_id: `mbx${outlookState.accounts.length + 1}@example.com`,
        name: `mbx${outlookState.accounts.length + 1}@example.com`,
        default: outlookState.accounts.length === 0,
        managed: true,
      });
    }
    // Notion managed connect = add the next workspace (generic accounts layer).
    if (p.includes("/connectors/notion/")) {
      const next = NOTION_NEXT[notionState.accounts.length] || {
        account_id: `ws-${notionState.accounts.length + 1}`, name: "extra",
      };
      notionState.accounts.push({
        ...next, default: notionState.accounts.length === 0, managed: true,
      });
    }
    // HubSpot managed connect = add the next portal at the requested access tier.
    if (p.includes("/connectors/hubspot/")) {
      const access = (req.postDataJSON() || {}).access || "read";
      const next = HUBSPOT_NEXT[hubspotState.portals.length] || {
        hub_id: `9${hubspotState.portals.length}`, name: "extra", sandbox: false,
      };
      hubspotState.portals.push({
        ...next, default: hubspotState.portals.length === 0, managed: true, access,
      });
    }
    return json({ ok: true });
  }
  if (p.endsWith("/v1/cloud/gallery")) {
    return json(
      CLOUD_STATE.signed_in
        ? { ok: true, personas: GALLERY_PERSONAS }
        : { ok: false, error: "gallery requires cloud sign-in", personas: [] },
    );
  }
  if (/\/v1\/cloud\/gallery\/[^/]+$/.test(p)) {
    if (!CLOUD_STATE.signed_in) return json({ ok: false, error: "gallery requires cloud sign-in" });
    const slug = p.split("/").pop();
    const cardBase = GALLERY_PERSONAS.find((g) => g.slug === slug) ?? GALLERY_PERSONAS[0];
    return json({
      ok: true,
      card: { ...cardBase, pitch_markdown: "**Walk into every call already knowing the account.**" },
      capabilities: {
        tools: ["files", "search", "todo"],
        risk: [],
        connectors: true,
        mcp: [],
        messaging: true,
        recommended_mode: "interactive",
        recommended_models: [],
      },
      recommends: [
        { kind: "connector", ref: "hubspot", reason: "read deals and contacts", tier: "core" },
      ],
    });
  }
}
