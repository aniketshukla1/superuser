import { fetch, httpBase, jsonBody, jsonOf } from "./neverRejectingJson";
import { announcePersonasChanged, type Persona } from "./personas";

// One assistant in detail, and which connections its new sessions start with (§5).

/** Something an assistant's manifest recommends (`recommends`): a connector or MCP server it
 *  works best with, why, and how much (core or optional). The server marks `connected` from
 *  the connector list, so the page needs no second request to show it. */
export interface PersonaRecommendation {
  /** "connector", "mcp", … */
  kind: string;
  /** A connector id ("github") or an MCP server's name. */
  ref: string;
  /** "core" or "optional". */
  tier: string;
  reason: string;
  connected: boolean;
}

/** The middle layer of §4: whether this assistant's new sessions start with a connected
 *  connector switched on. */
export interface PersonaDefaultConnection {
  connector: string;
  enabled: boolean;
  /** Whether the account is really connected; the switch is disabled when it isn't. */
  connected: boolean;
}

export interface PersonaDetail {
  id: string;
  name: string;
  tagline: string;
  description: string;
  icon: string;
  /** Picks the fallback glyph when `icon` names something the client doesn't know. */
  family: string;
  /** Whether it appears in the new-session picker. */
  enabled: boolean;
  workspace: string;
  default_permission_mode: string;
  tools: string[];
  /** This assistant's work suffers badly on a small model (`needs_capable_model` in the
   *  manifest). It replaced a list of model ids that nothing declared or read. */
  needs_capable_model?: boolean;
  recommends: PersonaRecommendation[];
  /** The procedures it was given. Empty means the whole installed catalogue: right for a
   *  generalist, wrong for a specialist. The server always sent this; the client never
   *  declared it, so the page showed an assistant's capabilities and not its skills. */
  skills?: string[];
  /** Worker types it staffs when it delegates; empty means whatever is installed. */
  subagents?: string[];
  /** Other assistants it may hand work to; empty means none. */
  asks?: string[];
  /** The AG-UI address of a remote agent that takes its turns, if there is one. */
  remote?: string | null;
  /** Whether a token is saved for that agent. The token itself never comes back. */
  remote_token_set?: boolean;
  /** Optional because the server hasn't always sent it, and a required array that arrives
   *  undefined is a crash, not a gap. */
  default_connections?: PersonaDefaultConnection[];
}

const personaUrl = (id: string, rest = "") => `${httpBase()}/v1/personas/${encodeURIComponent(id)}${rest}`;

/** Does this address answer as an AG-UI agent? Tests what was typed, before saving. Without a
 *  token, the saved one is sent only to the host of the saved address. */
export async function testRemoteAgent(id: string, endpoint: string, token?: string): Promise<{ ok: boolean; message?: string; error?: string }> {
  const reply = await jsonOf(await fetch(personaUrl(id, "/remote/test"), jsonBody("POST", { endpoint, token: token || undefined })));
  return { ok: reply.ok === true, message: reply.message, error: reply.error };
}

export async function getPersonaDetail(id: string): Promise<PersonaDetail> {
  return (await fetch(personaUrl(id))).json();
}

/** Whether this assistant's new sessions start with the connector on. */
export async function setPersonaConnection(
  id: string,
  connector: string,
  enabled: boolean,
): Promise<{ ok: boolean; default_connections?: PersonaDefaultConnection[]; error?: string }> {
  return (await fetch(personaUrl(id, "/connections"), jsonBody("POST", { connector, enabled }))).json();
}

/** Switch the assistant on or off, i.e. whether it shows in the new-session picker. */
export async function setPersonaEnabled(id: string, enabled: boolean): Promise<{ ok: boolean; personas?: Persona[]; error?: string }> {
  const reply = await jsonOf(await fetch(personaUrl(id, "/enable"), jsonBody("POST", { enabled })));
  if (reply.ok) announcePersonasChanged();
  return { ...reply, ok: reply.ok === true };
}
