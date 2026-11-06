import { fetch, httpBase, jsonBody, listOf } from "./neverRejectingJson";

// MCP servers the user has added.

export interface McpServer {
  name: string;
  transport: string;
  config: Record<string, any>;
  enabled: boolean;
  requires_approval: boolean;
  /** "connected", "configured" or "disabled"; for an OAuth server also "needs_auth" (no tokens
   *  yet) or "authorizing" (the browser sign-in is under way). */
  status: string;
  auth?: "oauth" | null;
  last_error?: string | null;
  tool_count: number | null;
}

const mcpUrl = (path = "") => `${httpBase()}/v1/mcp${path}`;
const serverUrl = (name: string, action = "") => mcpUrl(`/${encodeURIComponent(name)}${action}`);

export async function getMcpServers(): Promise<McpServer[]> {
  const rows = await listOf<Record<string, any>>(await fetch(mcpUrl()), "servers");
  return rows.map(asMcpServer);
}

const text = (value: unknown, fallback: string) => (typeof value === "string" && value !== "" ? value : fallback);

/** A row off the wire as a full `McpServer`, with anything missing filled in.
 *
 *  The wire and this type have drifted: `GET /v1/mcp` currently returns the remote MCP
 *  connector list (`{id, name, url, connected}`), which has none of the registry fields
 *  declared above. Calling `status.replace(...)` on such a row threw during render, and the
 *  whole MCP tab fell to an error boundary instead of showing one odd row. No view should be
 *  able to crash on a field the server left out, so the gap is filled once, here at the edge.
 *  The fillers say only what is known: a missing `auth` stays null rather than becoming
 *  "oauth", which would draw a Sign-in button for a flow the server may not have. */
export function asMcpServer(raw: Record<string, any>): McpServer {
  const config = raw.config && typeof raw.config === "object" ? raw.config : raw.url ? { type: "http", url: raw.url } : {};
  return {
    name: text(raw.name, text(raw.id, "unnamed")),
    transport: text(raw.transport, raw.url ? "http" : "unknown"),
    config,
    enabled: typeof raw.enabled === "boolean" ? raw.enabled : Boolean(raw.connected),
    requires_approval: Boolean(raw.requires_approval),
    status: text(raw.status, raw.connected ? "connected" : "configured"),
    auth: raw.auth === "oauth" ? "oauth" : null,
    last_error: typeof raw.last_error === "string" ? raw.last_error : null,
    tool_count: typeof raw.tool_count === "number" ? raw.tool_count : null,
  };
}

export async function addMcpServer(name: string, config: Record<string, any>) {
  return (await fetch(mcpUrl(), jsonBody("POST", { name, config }))).json();
}

export async function patchMcpServer(name: string, changes: Record<string, any>) {
  return (await fetch(serverUrl(name), jsonBody("PATCH", changes))).json();
}

export async function deleteMcpServer(name: string) {
  return (await fetch(serverUrl(name), { method: "DELETE" })).json();
}

export async function getMcpTools(name: string): Promise<{ ok: boolean; error?: string; tools: { name: string; description: string }[] }> {
  return (await fetch(serverUrl(name, "/tools"))).json();
}

export async function reloadMcp() {
  return (await fetch(mcpUrl("/reload"), { method: "POST" })).json();
}

/** Connect one server now. An OAuth server opens the system browser; poll getMcpServers() to
 *  see the status move from "authorizing" to "connected" or "needs_auth". */
export async function connectMcp(name: string): Promise<{ ok: boolean; started?: boolean }> {
  return (await fetch(serverUrl(name, "/connect"), { method: "POST" })).json();
}

/** Disconnect and forget the stored OAuth tokens. */
export async function signoutMcp(name: string): Promise<{ ok: boolean }> {
  return (await fetch(serverUrl(name, "/signout"), { method: "POST" })).json();
}
