import { fetch, httpBase, jsonOf } from "./neverRejectingJson";
import { announcePersonasChanged, type Persona } from "./personas";

// -- Persona detail + connection defaults (§5) --------------------------------
// A persona's declared recommendation (manifest `recommends`): a connector or MCP server it works
// best with, with a reason + tier (core/optional). `connected` is annotated server-side from the
// connector list so the detail page can show connect state without a second round-trip.
export interface PersonaRecommendation {
  kind: string; // "connector" | "mcp" | …
  ref: string; // connector id (e.g. "github") or mcp/server name
  reason: string;
  tier: string; // "core" | "optional"
  connected: boolean;
}

// A persona-default connection (the middle of the §4 hierarchy): for a connected connector, whether
// new sessions of this persona get it enabled by default.
export interface PersonaDefaultConnection {
  connector: string; // connector id
  enabled: boolean; // persona-default on/off
  connected: boolean; // is the account actually connected (else the toggle is disabled)
}

export interface PersonaDetail {
  id: string;
  name: string;
  icon: string;
  /** Decides the fallback glyph when `icon` names something the client can't resolve. */
  family: string;
  tagline: string;
  description: string;
  enabled: boolean; // persona on/off (shown in the picker)
  tools: string[];
  /** This assistant's work degrades badly on a small model — see `needs_capable_model`
   *  in the manifest. Replaces a list of model ids nothing read or declared. */
  needs_capable_model?: boolean;
  default_permission_mode: string;
  workspace: string;
  recommends: PersonaRecommendation[];
  /** Worker types this assistant staffs when it delegates. Empty = whatever is installed. */
  subagents?: string[];
  /** Which other assistants this one may hand work to. Empty = none. */
  asks?: string[];
  /** The AG-UI address of the remote agent that does its turns, if one does. */
  remote?: string | null;
  /** Whether a token is saved for that agent. The token itself never comes back. */
  remote_token_set?: boolean;
  /** The procedures this assistant was given. Empty = the whole installed catalogue,
   *  which is right for a generalist and wrong for a specialist. The server has always
   *  sent this; the client just never declared it, so the detail page showed an
   *  assistant's capabilities and not its skills. */
  skills?: string[];
  /** Optional in the type because the server has not always sent it, and a required
   *  array that arrives undefined is a crash rather than a gap. */
  default_connections?: PersonaDefaultConnection[];
}

/** Does this address answer as an AG-UI agent? Tests what is typed, before it is saved.
 *  With no token given, the saved one is sent only to the saved address's host. */
export async function testRemoteAgent(
  id: string,
  endpoint: string,
  token?: string,
): Promise<{ ok: boolean; message?: string; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/personas/${encodeURIComponent(id)}/remote/test`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ endpoint, token: token || undefined }),
  });
  const out = await jsonOf(res);
  return { ok: out.ok === true, message: out.message, error: out.error };
}

export async function getPersonaDetail(id: string): Promise<PersonaDetail> {
  const res = await fetch(`${httpBase()}/v1/personas/${encodeURIComponent(id)}`);
  return res.json();
}

/** Set a persona-default connection (new sessions of this persona get it on/off by default). */
export async function setPersonaConnection(
  id: string,
  connector: string,
  enabled: boolean,
): Promise<{ ok: boolean; default_connections?: PersonaDefaultConnection[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/personas/${encodeURIComponent(id)}/connections`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ connector, enabled }),
  });
  return res.json();
}

/** Enable/disable the persona (whether it surfaces in the new-session picker). */
export async function setPersonaEnabled(
  id: string,
  enabled: boolean,
): Promise<{ ok: boolean; personas?: Persona[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/personas/${encodeURIComponent(id)}/enable`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ enabled }),
  });
  const out = await jsonOf(res);
  if (out.ok) announcePersonasChanged();
  return { ...out, ok: out.ok === true };
}
