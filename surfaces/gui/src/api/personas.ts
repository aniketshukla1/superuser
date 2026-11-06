import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

/** Sent after any change to the assistants (switched on or off, installed, removed), so parts
 *  that stay mounted, like the sidebar's new-session picker, fetch again instead of going stale. */
export const PERSONAS_CHANGED = "assistant:personas-changed";
export function announcePersonasChanged() {
  window.dispatchEvent(new CustomEvent(PERSONAS_CHANGED));
}

export interface Persona {
  id: string;
  name: string;
  tagline: string;
  icon: string;
  family: string;
  /** "assistant" or "workbench". A workbench can use a shell and is tied to a place (a repo,
   *  a machine); an assistant works across memory and connected apps. */
  kind?: string;
  /** "git", "project", "deliverable" or "none"; decides whether it is tied to a project. */
  workspace: string;
  needs_workspace: boolean;
  tools: string[];
  builtin: boolean;
  enabled: boolean;
  surfaced: boolean;
  default: boolean;
  /** From the manifest: a first-person greeting and one-click opening prompts for a fresh
   *  session. Without them the generic welcome is shown. */
  intro?: string;
  starters?: string[];
}

/** One card under "Installed N — review and enable below", from `POST /v1/personas/install`:
 *  what was just taken on, what it may do and what that costs. The server works `risk` out
 *  from the capabilities instead of taking the manifest's word for it; an assistant allowed to
 *  describe its own risk would call it none. */
export interface PersonaConsent {
  id: string;
  name: string;
  description: string;
  source: string | null;
  builtin: boolean;
  tools: string[];
  /** "read", "write_local", "exec" or "external": the app's own words, shared with the risk
   *  overrides card. Empty means it only reads. */
  risk: string[];
  mcp: string[];
  connectors: boolean;
  messaging: boolean;
  recommended_mode: string;
  /** Took the place of `recommended_models`, which listed model ids nothing declared and a
   *  local-first user couldn't act on. */
  needs_capable_model: boolean;
}

export type NewPersona = {
  name: string;
  responsibilities?: string;
  icon?: string;
  starters?: string[];
  permission_mode?: string;
  tools?: string[];
  /** Reference files, saved as text next to the manifest so the assistant can read them. */
  files?: { name: string; content: string }[];
};

/** Create an assistant from the New-assistant dialog. The server writes its manifest to
 *  data_dir/personas/<id>.md, a plain file the user can edit or delete. */
export async function createPersona(payload: NewPersona): Promise<{ ok: boolean; error?: string; persona?: { id: string; name: string } }> {
  return (await fetch(`${httpBase()}/v1/personas`, jsonBody("POST", payload))).json();
}
