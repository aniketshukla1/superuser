import { fetch, httpBase, jsonOf } from "./neverRejectingJson";
import { announcePersonasChanged, type Persona, type PersonaConsent } from "./personas";

// -- Importing an agent package ------------------------------------------------
// Two steps: read the folder into a plan (nothing written), then import what the plan showed.
// The digest ties the two together — a folder that changed in between is refused.
export interface PlannedSkill {
  name: string;
  title: string;
  description: string;
  instructions: string;
  tools: string[];
  unmapped: string[];
  problems: string[];
}

export interface PlannedPersona {
  id: string;
  name: string;
  tagline: string;
  instructions: string;
  tools: string[];
  skills: string[];
  remote?: string | null;
  problems: string[];
  notes: string[];
}

export interface PackagePlan {
  digest: string;
  personas: PlannedPersona[];
  skills: PlannedSkill[];
  left_behind: string[];
}

export async function previewAgentPackage(dir: string): Promise<{ ok: boolean; plan?: PackagePlan; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/import/package/preview`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ dir }),
  });
  const out = await jsonOf(res);
  return { ok: out.ok === true, plan: out.plan, error: out.error };
}

export async function importAgentPackage(
  dir: string,
  digest: string,
): Promise<{ ok: boolean; skills?: string[]; consent?: PersonaConsent[]; personas?: Persona[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/import/package`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ dir, digest }),
  });
  const out = await jsonOf(res);
  if (out.ok) announcePersonasChanged();
  return { ...out, ok: out.ok === true };
}
