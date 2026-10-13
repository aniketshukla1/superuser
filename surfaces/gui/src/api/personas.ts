import { fetch, httpBase } from "./neverRejectingJson";

// -- Personas -----------------------------------------------------------------

// Fired after any persona mutation (enable/disable/install/delete) so always-mounted
// consumers (the sidebar's new-session picker) refetch instead of going stale.
export const PERSONAS_CHANGED = "assistant:personas-changed";
export function announcePersonasChanged() {
  window.dispatchEvent(new CustomEvent(PERSONAS_CHANGED));
}

export interface Persona {
  id: string;
  name: string;
  icon: string;
  tagline: string;
  needs_workspace: boolean;
  builtin: boolean;
  family: string;
  workspace: string; // "git" | "project" | "deliverable" | "none" — drives project-scoping
  /** "assistant" | "workbench". Workbenches can run shell and are bound to a place
   *  (a repo, a machine); assistants work across memory and connected apps. */
  kind?: string;
  tools: string[];
  enabled: boolean;
  surfaced: boolean;
  default: boolean;
  // First-person greeting + one-click opening prompts for a fresh session, from the
  // persona manifest. Empty falls back to the generic hero copy.
  intro?: string;
  starters?: string[];
}

/** One card in "Installed N — review and enable below", from
 *  `POST /v1/personas/install`. What you just took on: what it may do, and what
 *  that costs. `risk` is DERIVED server-side from the capability list rather than
 *  declared by the manifest — an assistant that got to describe its own risk would
 *  describe it as none. */
export interface PersonaConsent {
  id: string;
  name: string;
  description: string;
  tools: string[];
  /** "read" | "write_local" | "exec" | "external" — the app's own vocabulary,
   *  shared with the risk-overrides card. Empty means read-only. */
  risk: string[];
  connectors: boolean;
  mcp: string[];
  messaging: boolean;
  recommended_mode: string;
  /** Replaces `recommended_models`, which named model ids nothing declared and no
   *  local-first user could act on. */
  needs_capable_model: boolean;
  source: string | null;
  builtin: boolean;
}

/** Create an assistant from the New-assistant modal. The backend writes a manifest
 *  under data_dir/personas/<id>.md — a plain file the user can edit or delete. */
export async function createPersona(payload: {
  name: string;
  responsibilities?: string;
  icon?: string;
  starters?: string[];
  permission_mode?: string;
  tools?: string[];
  /** Reference files, stored as text beside the manifest so the assistant can read them. */
  files?: { name: string; content: string }[];
}): Promise<{ ok: boolean; error?: string; persona?: { id: string; name: string } }> {
  const res = await fetch(`${httpBase()}/v1/personas`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  return res.json();
}
