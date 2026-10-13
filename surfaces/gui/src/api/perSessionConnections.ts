import { fetch, httpBase } from "./neverRejectingJson";

// -- Per-session connections (Sources bar + drawer, §6) -----------------------
// An effective-enabled connector for a session, with a short human detail (e.g. "#ocw-test · DMs").
// `enabled` reflects the session override/persona default so the drawer toggle shows correct state.
/** How far a session may go through a source. "read" consults it; "write" may also
 *  send, post or modify through it. The folder half of this model is `Root.writable`. */
export type ConnectionScope = "read" | "write";

export interface SessionConnectedConnector {
  connector: string;
  enabled: boolean;
  detail: string;
  /** Absent from a server older than session scopes — treated as "write" there, which
   *  is what every connected source could do before the gate existed. */
  scope?: ConnectionScope;
  /** Whether a message can actually leave through this source. Comes from the send
   *  path itself, not the catalogue's `two_way` flag — the two disagree. */
  sendable?: boolean;
}

// A persona-recommended connector not yet connected (drives the `⚠ N` attention count).
export interface SessionRecommendedConnector {
  connector: string;
  reason: string;
  tier: string;
  connected: boolean;
}

export interface SessionConnections {
  connected: SessionConnectedConnector[];
  recommended: SessionRecommendedConnector[];
  attention: number; // ⚠ count = recommended connectors not yet connected
}

/** `persona` = the active persona hint — required for brand-new sessions (no server-side
 * record yet), otherwise the view resolves to the default persona's defaults/recommends. */
export async function getSessionConnections(
  sessionId: string,
  persona?: string,
): Promise<SessionConnections> {
  const q = persona ? `?persona=${encodeURIComponent(persona)}` : "";
  const res = await fetch(
    `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/connections${q}`,
  );
  return res.json();
}

/**
 * Set a per-session connection override (mute/unmute a connector for THIS session). Pass
 * `clear: true` to drop the override and inherit the persona default again.
 */
export async function setSessionConnection(
  sessionId: string,
  connector: string,
  enabled: boolean,
  clear = false,
): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/connections`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ connector, enabled, ...(clear ? { clear: true } : {}) }),
  });
  return res.json();
}

/**
 * Narrow or widen what this session may do through one source, independently of the
 * on/off toggle — muting a source must not silently reset the scope chosen for it.
 * The gate is enforced server-side at the send site, not just drawn here.
 */
export async function setSessionConnectionScope(
  sessionId: string,
  connector: string,
  scope: ConnectionScope,
): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/connections`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ connector, scope }),
  });
  return res.json();
}
