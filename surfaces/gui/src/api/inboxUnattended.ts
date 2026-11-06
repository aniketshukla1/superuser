import { fetch, httpBase, jsonBody, listOf } from "./neverRejectingJson";

// The Inbox: what waits on the user from sessions nobody is watching.

export interface InboxItem {
  id: string;
  kind: "approval" | "directory" | "notification" | "plan" | "question";
  state: "pending" | "resolved";
  title: string;
  body: string;
  inbox: string;
  visibility?: "inline" | "inbox";
  created_at: string;
  resolved_at: string | null;
  resolution: string | null;
  /** For a question (ask_user): suggested answers, and whether free text or several picks are allowed. */
  options?: string[];
  allow_text?: boolean;
  multi?: boolean;
  /** What the kind needs, e.g. {path, writable} for a directory request. */
  data?: Record<string, any>;
  /** The session it came from, joined in by the server so the Inbox needs nothing else. */
  session_id: string;
  session_title?: string;
  session_agent?: string | null;
  session_workspace?: string | null;
  session_exists?: boolean;
}

export async function getInbox(sessionId?: string, state?: string): Promise<InboxItem[]> {
  const filters = new URLSearchParams();
  if (sessionId) filters.set("session_id", sessionId);
  if (state) filters.set("state", state);
  return listOf<InboxItem>(await fetch(`${httpBase()}/v1/inbox?${filters.toString()}`), "items");
}

export async function resolveInboxItem(id: string, resolution: string): Promise<{ ok: boolean }> {
  const res = await fetch(`${httpBase()}/v1/inbox/${encodeURIComponent(id)}/resolve`, jsonBody("POST", { resolution }));
  return res.json();
}
