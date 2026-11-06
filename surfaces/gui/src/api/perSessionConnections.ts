import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

// A session's connections: the Sources bar and its drawer (§6).

/** How far a session may go through a source: "read" looks things up, "write" may also send,
 *  post or change things through it. Folders have the same split in `Root.writable`. */
export type ConnectionScope = "read" | "write";

/** A connector the session can use, with a short human detail ("#team-test · DMs"). `enabled`
 *  already includes the session's override or the assistant's default, so the drawer's
 *  switch shows the real state. */
export interface SessionConnectedConnector {
  connector: string;
  detail: string;
  enabled: boolean;
  /** Missing from servers that predate scopes, and read there as "write", which is what any
   *  connected source could do before the gate existed. */
  scope?: ConnectionScope;
  /** Whether a message can really leave through this source. It comes from the sending code
   *  itself, not from the catalogue's `two_way` flag; the two don't always agree. */
  sendable?: boolean;
}

/** A connector the assistant recommends that isn't connected yet; these make the `⚠ N` count. */
export interface SessionRecommendedConnector {
  connector: string;
  connected: boolean;
  reason: string;
  tier: string;
}

export interface SessionConnections {
  connected: SessionConnectedConnector[];
  recommended: SessionRecommendedConnector[];
  /** The ⚠ count: recommended connectors still unconnected. */
  attention: number;
}

const connectionsUrl = (sessionId: string) => `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/connections`;

/** `persona` hints at the active assistant. A brand-new session has no record on the server
 *  yet, and without the hint the view would fall back to the default assistant's settings. */
export async function getSessionConnections(sessionId: string, persona?: string): Promise<SessionConnections> {
  const hint = persona ? `?persona=${encodeURIComponent(persona)}` : "";
  const res = await fetch(connectionsUrl(sessionId) + hint);
  return res.json();
}

/** Turn a connector off or on for THIS session only. `clear: true` drops the override, so the
 *  assistant's default applies again. */
export async function setSessionConnection(
  sessionId: string,
  connector: string,
  enabled: boolean,
  clear = false,
): Promise<{ ok: boolean; error?: string }> {
  const body = clear ? { connector, enabled, clear: true } : { connector, enabled };
  const res = await fetch(connectionsUrl(sessionId), jsonBody("POST", body));
  return res.json();
}

/** Narrow or widen what this session may do through one source. Separate from the on/off
 *  switch, so muting a source never quietly resets the scope chosen for it. The server
 *  enforces it where the message is sent; this only shows it. */
export async function setSessionConnectionScope(
  sessionId: string,
  connector: string,
  scope: ConnectionScope,
): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(connectionsUrl(sessionId), jsonBody("POST", { connector, scope }));
  return res.json();
}
