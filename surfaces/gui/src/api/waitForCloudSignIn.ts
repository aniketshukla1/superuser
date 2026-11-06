import type { Connector } from "./connectors";
import { fetch, httpBase, jsonBody, listOf } from "./neverRejectingJson";

// Superuser Cloud: an optional sign-in. Pasting a token by hand always works without it.
// Also the connector list, and the shapes the activity trail records actions with.

const cloudUrl = (path: string) => `${httpBase()}/v1/cloud/${path}`;
const connectorUrl = (name: string, action: string) => `${httpBase()}/v1/connectors/${encodeURIComponent(name)}/${action}`;

export interface CloudStatus {
  signed_in: boolean;
  account: string;
  user_id: string;
  /** Phase 5 opt-out. Someone signed out sends nothing whatever this says. */
  telemetry_enabled?: boolean;
}

/** Switch product telemetry on or off. Stored here, and it only matters when signed in. */
export async function setCloudTelemetry(enabled: boolean): Promise<{ ok: boolean; telemetry_enabled?: boolean }> {
  return (await fetch(cloudUrl("telemetry"), jsonBody("POST", { enabled }))).json();
}

export async function getCloudStatus(): Promise<CloudStatus> {
  return (await fetch(cloudUrl("status"))).json();
}

/** The sidecar opens the system browser; the GUI then watches the status. */
export async function cloudLogin(): Promise<{ ok: boolean }> {
  return (await fetch(cloudUrl("login"), { method: "POST" })).json();
}

export async function cloudLogout(): Promise<{ ok: boolean }> {
  return (await fetch(cloudUrl("logout"), { method: "POST" })).json();
}

/** Watch for the browser sign-in to finish, for about two minutes at most.
 *
 *  For the first 20 seconds it checks every half second: the moment someone finishes in the
 *  browser they are looking at the app, waiting for it to change, and a two-second gap feels
 *  like a slow sign-in (owner, 2026-07-16). After that it checks every two seconds. `onDone`
 *  gets the signed-in status, or null if time ran out. The returned function cancels it; call
 *  it on unmount. */
export function waitForCloudSignIn(onDone: (s: CloudStatus | null) => void): () => void {
  const QUICK_CHECKS = 40;
  const MAX_CHECKS = 90; // 40 × 0.5 s, then 50 × 2 s
  let stopped = false;
  let pending: ReturnType<typeof setTimeout> | undefined;
  let checks = 0;

  const later = (ms: number) => {
    pending = setTimeout(check, ms);
  };
  async function check() {
    checks += 1;
    let status: CloudStatus | null;
    try {
      status = await getCloudStatus();
    } catch {
      status = null;
    }
    if (stopped) return;
    if (status?.signed_in) {
      onDone(status);
      return;
    }
    if (checks >= MAX_CHECKS) {
      onDone(null);
      return;
    }
    later(checks < QUICK_CHECKS ? 500 : 2000);
  }

  later(500);
  return () => {
    stopped = true;
    if (pending !== undefined) clearTimeout(pending);
  };
}

/** One-click connect through the cloud. `access` picks a consent level the broker defines
 *  (HubSpot: read or write). GitHub needs no choice: the broker authorizes first, so one
 *  connect either links an existing App installation or carries on to the install page. */
export async function connectManaged(name: string, options?: { access?: "read" | "write" }): Promise<{ ok: boolean; error?: string }> {
  const body = options?.access ? { access: options.access } : {};
  return (await fetch(connectorUrl(name, "connect-managed"), jsonBody("POST", body))).json();
}

/** One-click connect for a connector backed by MCP (monday, asana, jira): the sidecar opens
 *  the vendor's sign-in in the browser, with local OAuth and no cloud account. Poll
 *  getConnectors until the card shows connected. */
export async function connectMcpBacked(name: string): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(connectorUrl(name, "mcp-connect"), { method: "POST" })).json();
}

export interface ConnectorTool {
  name: string;
  label: string;
  description: string;
  kind: "read" | "write" | string;
  enabled: boolean;
  requires_approval: boolean;
}

export async function getConnectors(): Promise<Connector[]> {
  return listOf(await fetch(`${httpBase()}/v1/connectors`), "connectors");
}

export async function connectConnector(name: string, fields: Record<string, string>): Promise<{ ok: boolean; account?: string; error?: string }> {
  return (await fetch(connectorUrl(name, "connect"), jsonBody("POST", { fields }))).json();
}

export async function disconnectConnector(name: string): Promise<{ ok: boolean }> {
  return (await fetch(connectorUrl(name, "disconnect"), { method: "POST" })).json();
}

/** What started the run an action belongs to. */
export interface AuditInitiator {
  /** "person", "approval", "schedule", "watch", "message", "subagent", "api" or "startup". */
  kind: string;
  id?: string;
}

/** The facts an action was judged on; never its content. */
export interface AuditSubject {
  tool: string;
  intent?: string;
  risk?: string;
  target?: string;
  command?: string;
  file?: string;
  page?: string;
  mcp_server?: string;
  mcp_tool?: string;
  mcp_effect?: string;
  /** How long the text it carried was (a message, a typed value), in characters. */
  chars?: number;
  tainted?: boolean;
}

/** How the user's boundaries ruled on one action. */
export interface AuditDecision {
  allowed: boolean;
  carried_out: boolean;
  mode: "enforce" | "record" | string;
  /** "deny", "allow" or "default". */
  source: string;
  rule?: string;
  rule_name?: string;
  reason?: string;
}
