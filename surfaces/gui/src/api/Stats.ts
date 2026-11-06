import type { SessionInfo, TurnUsage } from "../types";
import { fetch, httpBase, jsonBody, listOf, type BoardItem, type WorkspaceCommandTrust } from "./neverRejectingJson";

// Sessions and their messages and files, the work board, trust and risk rules, and the
// numbers on the new-chat panel.

const v1 = (path: string) => `${httpBase()}/v1/${path}`;
const sessionUrl = (sessionId: string, rest = "") => v1(`sessions/${encodeURIComponent(sessionId)}${rest}`);

/** The project's work board. Read-only: assistants change it, people read it. */
export async function getBoard(workspace: string): Promise<BoardItem[]> {
  return listOf(await fetch(v1(`board?workspace=${encodeURIComponent(workspace)}`)), "items");
}

export interface RiskRule {
  pattern: string;
  risk: string;
  /** False when the rule can never apply because it tries to loosen a built-in tool. */
  effective: boolean;
}

export async function getRiskOverrides(): Promise<RiskRule[]> {
  return listOf(await fetch(v1("risk-overrides")), "rules");
}

/** Replace the whole list: order matters to these rules, so adding to the end isn't enough. */
export async function setRiskOverrides(rules: { pattern: string; risk: string }[]): Promise<{ ok: boolean; rules?: RiskRule[]; error?: string }> {
  return (await fetch(v1("risk-overrides"), jsonBody("POST", { rules }))).json();
}

export async function getTrustedWorkspaces(): Promise<WorkspaceCommandTrust[]> {
  return listOf(await fetch(v1("workspaces/trusted")), "workspaces");
}

export async function setWorkspaceTrusted(path: string, trusted: boolean): Promise<{ ok: boolean; error?: string } & WorkspaceCommandTrust> {
  return (await fetch(v1("workspaces/trust"), jsonBody("POST", { path, trusted }))).json();
}

export async function getSessions(workspace?: string): Promise<SessionInfo[]> {
  const filter = workspace ? `?workspace=${encodeURIComponent(workspace)}` : "";
  return listOf(await fetch(v1(`sessions${filter}`)), "sessions");
}

/** What this workspace has done, for the panel a new chat opens on. `tokens` is input plus
 *  output as providers reported them, added up over the messages that carried a figure: see
 *  `priced_messages`. */
export interface Stats {
  sessions: number;
  messages: number;
  tokens: number;
  /** How many messages the token total comes from. Providers report usage unevenly, and
   *  nothing from before usage existed has any, so a bare total would pass off a partial count
   *  as a complete one. */
  priced_messages: number;
  active_days: number;
  current_streak: number;
  longest_streak: number;
  peak_hour: number | null;
  favorite_model: string | null;
  favorite_agent: string | null;
  days: { date: string; count: number }[];
  agents: { agent: string; sessions: number }[];
  models: { model: string; sessions: number; messages: number; input: number; output: number; tokens: number }[];
}

export async function getStats(range: "all" | "30d" | "7d" = "all"): Promise<Stats | null> {
  try {
    const res = await fetch(v1(`stats?range=${range}`));
    return res.ok ? ((await res.json()) as Stats) : null;
  } catch {
    // Better an empty panel than an error card where the greeting used to be.
    return null;
  }
}

/** A message that came in through a connector (§3.1), attached to the user message it was
 *  wrapped in. It is only for display, as the ConnectorMessageCard; the model still gets the
 *  wrapped `content`. */
export interface MessageSource {
  /** The platform id, e.g. "slack". */
  connector: string;
  kind: "channel" | "dm";
  channel_id: string;
  /** Looked up, though it may still equal the id ("#team-test"). */
  channel_name: string;
  sender_id: string;
  /** Looked up; may equal the id. */
  sender_name: string;
  /** Seconds since the epoch. */
  ts: number;
  /** The message exactly as sent, which is what the card shows. */
  text: string;
}

/** One message from GET /v1/sessions/{id}/messages. Left open, because itemsFromMessages reads
 *  several fields that only some roles have. */
export interface ConversationMessage {
  role: string;
  content?: any;
  tool_calls?: any[];
  tool_call_id?: string;
  /** Present when the message came in through a connector. */
  source?: MessageSource;
  /** The token counts of the request that produced an assistant message; older servers omit them. */
  usage?: TurnUsage;
  [key: string]: any;
}

export async function getSessionMessages(sessionId: string): Promise<ConversationMessage[]> {
  return listOf(await fetch(v1(`sessions/${sessionId}/messages`)), "messages");
}

export async function renameSession(sessionId: string, title: string): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(sessionUrl(sessionId), jsonBody("PATCH", { title }))).json();
}

export async function setSessionFlags(sessionId: string, flags: { pinned?: boolean; archived?: boolean }): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(sessionUrl(sessionId), jsonBody("PATCH", flags))).json();
}

export async function deleteSession(sessionId: string): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(sessionUrl(sessionId), { method: "DELETE" })).json();
}

export interface ArtifactInfo {
  name: string;
  /** Relative to the workspace; how the GUI and the API name the file. */
  path: string;
  /** The absolute path, which "Copy path" copies. */
  abs_path?: string;
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
  /** For kind "folder": what the folder holds (models sometimes link a whole package folder). */
  entries?: { name: string; dir: boolean; size: number }[];
  /** For kind "doc": a .docx as its paragraphs, each editable on its own. */
  paragraphs?: string[];
  /** For kind "office": why this file couldn't be opened for editing. */
  readonly_reason?: string;
}

export async function getArtifacts(sessionId: string): Promise<ArtifactInfo[]> {
  const res = await fetch(sessionUrl(sessionId, "/artifacts"));
  // A failed request is not an empty folder. `listOf` turns every failure into `[]` on
  // purpose, so callers take a failure branch instead of throwing, and for most lists that's
  // right. Not this one: every caller treats `[]` as a statement. The Artifacts button
  // decides it has nothing to show, the Files tab says the conversation made nothing, and
  // the delete confirmation promises no files will be lost; the 404 that comes back while
  // the sidecar is still starting would say all three. Throwing costs nothing here, since
  // every caller already has a `.catch`, and the one in App retries like the persona load.
  if (!res.ok) throw new Error(`artifacts unavailable (${res.status})`);
  return listOf(res, "artifacts");
}
