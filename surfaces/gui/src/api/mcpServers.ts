import { fetch, httpBase, listOf } from "./neverRejectingJson";

// -- MCP servers --------------------------------------------------------------
export interface McpServer {
  name: string;
  enabled: boolean;
  transport: string;
  requires_approval: boolean;
  // "connected" | "configured" | "disabled" | and for auth:"oauth" servers:
  // "needs_auth" (no tokens yet) | "authorizing" (browser sign-in in flight)
  status: string;
  auth?: "oauth" | null;
  last_error?: string | null;
  tool_count: number | null;
  config: Record<string, any>;
}

export async function getMcpServers(): Promise<McpServer[]> {
  const res = await fetch(`${httpBase()}/v1/mcp`);
  return (await listOf<Record<string, any>>(res, "servers")).map(asMcpServer);
}

/**
 * Wire row → `McpServer`, filling anything the server left out.
 *
 * The two shapes have drifted: `GET /v1/mcp` currently answers with the remote MCP
 * connector list (`{id, name, url, connected}`), which carries none of the registry
 * fields this contract declares. Reading `status.replace(...)` off one of those rows
 * threw, and because it threw during render it took the whole MCP tab down to an
 * error boundary rather than showing one bad row.
 *
 * A view should not be able to crash on a field a server omitted, so the conversion
 * happens here, once, at the edge — every consumer downstream gets the full shape.
 * The defaults are deliberately dull: they describe the row honestly and never invent
 * a capability (an absent `auth` stays null rather than guessing "oauth", which would
 * draw a Sign-in button for a flow the server may not implement).
 */
export function asMcpServer(raw: Record<string, any>): McpServer {
  const str = (v: unknown, fallback: string) => (typeof v === "string" && v ? v : fallback);
  return {
    name: str(raw.name, str(raw.id, "unnamed")),
    enabled: typeof raw.enabled === "boolean" ? raw.enabled : !!raw.connected,
    transport: str(raw.transport, raw.url ? "http" : "unknown"),
    requires_approval: !!raw.requires_approval,
    status: str(raw.status, raw.connected ? "connected" : "configured"),
    auth: raw.auth === "oauth" ? "oauth" : null,
    last_error: typeof raw.last_error === "string" ? raw.last_error : null,
    tool_count: typeof raw.tool_count === "number" ? raw.tool_count : null,
    config: raw.config && typeof raw.config === "object" ? raw.config : raw.url ? { type: "http", url: raw.url } : {},
  };
}

export async function addMcpServer(name: string, config: Record<string, any>) {
  const res = await fetch(`${httpBase()}/v1/mcp`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name, config }),
  });
  return res.json();
}

export async function patchMcpServer(name: string, changes: Record<string, any>) {
  const res = await fetch(`${httpBase()}/v1/mcp/${encodeURIComponent(name)}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(changes),
  });
  return res.json();
}

export async function deleteMcpServer(name: string) {
  const res = await fetch(`${httpBase()}/v1/mcp/${encodeURIComponent(name)}`, { method: "DELETE" });
  return res.json();
}

export async function getMcpTools(
  name: string,
): Promise<{ ok: boolean; error?: string; tools: { name: string; description: string }[] }> {
  const res = await fetch(`${httpBase()}/v1/mcp/${encodeURIComponent(name)}/tools`);
  return res.json();
}

export async function reloadMcp() {
  const res = await fetch(`${httpBase()}/v1/mcp/reload`, { method: "POST" });
  return res.json();
}

/** Connect one MCP server now. For OAuth servers this opens the system browser;
 * poll getMcpServers() for the status flip (authorizing → connected / needs_auth). */
export async function connectMcp(name: string): Promise<{ ok: boolean; started?: boolean }> {
  const res = await fetch(`${httpBase()}/v1/mcp/${encodeURIComponent(name)}/connect`, {
    method: "POST",
  });
  return res.json();
}

/** Drop the connection and forget the stored OAuth tokens. */
export async function signoutMcp(name: string): Promise<{ ok: boolean }> {
  const res = await fetch(`${httpBase()}/v1/mcp/${encodeURIComponent(name)}/signout`, {
    method: "POST",
  });
  return res.json();
}
