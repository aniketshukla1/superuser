import type { SessionInfo } from "../types";
import { fetch, httpBase, listOf, type BoardItem, type WorkspaceCommandTrust } from "./neverRejectingJson";

/** The project's work board. Read-only: assistants change it, people read it. */
export async function getBoard(workspace: string): Promise<BoardItem[]> {
  const res = await fetch(`${httpBase()}/v1/board?workspace=${encodeURIComponent(workspace)}`);
  return listOf(res, "items");
}

export interface RiskRule {
  pattern: string;
  risk: string;
  /** False when the rule can never take effect — it asks to loosen a built-in tool. */
  effective: boolean;
}

export async function getRiskOverrides(): Promise<RiskRule[]> {
  const res = await fetch(`${httpBase()}/v1/risk-overrides`);
  return listOf(res, "rules");
}

/** Replace the whole list — the rules are order-sensitive, so appending isn't enough. */
export async function setRiskOverrides(
  rules: { pattern: string; risk: string }[],
): Promise<{ ok: boolean; rules?: RiskRule[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/risk-overrides`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ rules }),
  });
  return res.json();
}

export async function getTrustedWorkspaces(): Promise<WorkspaceCommandTrust[]> {
  const res = await fetch(`${httpBase()}/v1/workspaces/trusted`);
  return listOf(res, "workspaces");
}

export async function setWorkspaceTrusted(
  path: string,
  trusted: boolean,
): Promise<{ ok: boolean; error?: string } & WorkspaceCommandTrust> {
  const res = await fetch(`${httpBase()}/v1/workspaces/trust`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, trusted }),
  });
  return res.json();
}

export async function getSessions(workspace?: string): Promise<SessionInfo[]> {
  const q = workspace ? `?workspace=${encodeURIComponent(workspace)}` : "";
  const res = await fetch(`${httpBase()}/v1/sessions${q}`);
  return listOf(res, "sessions");
}

/** What this workspace has done, for the panel a new chat opens on.
 *
 * Tokens are input+output as the provider reported them, summed over messages that carried
 * a figure — see `priced_messages`. */
export interface Stats {
  sessions: number;
  messages: number;
  tokens: number;
  /** How many messages the token total is actually built from. Providers report usage
   *  inconsistently and nothing written before usage existed has any, so a bare total
   *  would present a partial count as a complete one. */
  priced_messages: number;
  active_days: number;
  current_streak: number;
  longest_streak: number;
  peak_hour: number | null;
  favorite_model: string | null;
  favorite_agent: string | null;
  models: { model: string; sessions: number; messages: number; input: number; output: number; tokens: number }[];
  agents: { agent: string; sessions: number }[];
  days: { date: string; count: number }[];
}

export async function getStats(range: "all" | "30d" | "7d" = "all"): Promise<Stats | null> {
  try {
    const res = await fetch(`${httpBase()}/v1/stats?range=${range}`);
    if (!res.ok) return null;
    return (await res.json()) as Stats;
  } catch {
    // An empty screen is a better failure than an error card where a greeting used to be.
    return null;
  }
}

// A structured connector-delivered inbound message (§3.1). Attached to the user message it framed,
// for display only — the model still sees the framed `content`; this drives the ConnectorMessageCard.
export interface MessageSource {
  connector: string; // platform id, e.g. "slack"
  kind: "channel" | "dm";
  channel_id: string; // e.g. "C0BD7KZ1AH5"
  channel_name: string; // resolved; may equal the id (e.g. "#ocw-test")
  sender_id: string;
  sender_name: string; // resolved; may equal the id
  ts: number; // epoch seconds
  text: string; // the RAW message (what the card shows)
}

// A transcript message from GET /v1/sessions/{id}/messages. Kept permissive (open shape) because
// itemsFromMessages reads several role-specific fields; `source` is the optional connector sidecar.
export interface ConversationMessage {
  role: string;
  content?: any;
  tool_calls?: any[];
  tool_call_id?: string;
  source?: MessageSource;
  // Token counts for the round-trip that produced an assistant message
  // ({model, input, output, cache_read, cache_write}); absent on older servers.
  usage?: import("../types").TurnUsage;
  [key: string]: any;
}

export async function getSessionMessages(sessionId: string): Promise<ConversationMessage[]> {
  const res = await fetch(`${httpBase()}/v1/sessions/${sessionId}/messages`);
  return listOf(res, "messages");
}

export async function renameSession(sessionId: string, title: string): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ title }),
  });
  return res.json();
}

export async function setSessionFlags(
  sessionId: string,
  flags: { pinned?: boolean; archived?: boolean },
): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(flags),
  });
  return res.json();
}

export async function deleteSession(sessionId: string): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}`, { method: "DELETE" });
  return res.json();
}

export interface ArtifactInfo {
  path: string; // workspace-relative (the display/API identifier)
  abs_path?: string; // absolute — what "Copy path" copies
  name: string;
  kind: "markdown" | "html" | "image" | "code" | "text" | string;
  size: number;
  modified_at: number;
}

export interface ArtifactContent {
  ok: boolean;
  error?: string;
  path: string;
  kind: string;
  content?: string;
  data_url?: string;
  truncated?: boolean;
  // kind === "folder": a directory listing (models sometimes link a whole package dir).
  entries?: { name: string; dir: boolean; size: number }[];
  // kind === "doc": a .docx as its paragraphs, editable one at a time.
  paragraphs?: string[];
  // kind === "office": why this one could not be opened for editing.
  readonly_reason?: string;
}

export async function getArtifacts(sessionId: string): Promise<ArtifactInfo[]> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts`);
  // A failed request is not an empty folder.
  //
  // `listOf` flattens every failure to `[]` — deliberately, so callers take a
  // failure branch instead of rejecting into whoever forgot a `.catch()`. For a list
  // that is rendered, that is the right trade. For this one it is not, because every
  // caller renders `[]` as a *claim*: the Artifacts button decides it has nothing to
  // show, the Files tab says the conversation produced nothing, and the delete
  // confirmation tells you no files will be lost. The 404 you get while the sidecar
  // is still binding then reads as all three.
  //
  // Rejecting instead costs nothing here: all five call sites already have a
  // `.catch`, and the one in App.tsx now retries the way the persona load does.
  if (!res.ok) throw new Error(`artifacts unavailable (${res.status})`);
  return listOf(res, "artifacts");
}
