import { fetch, httpBase } from "./neverRejectingJson";

// -- Skills (SKILLS-SPEC §4) ----------------------------------------------------
// Scope = folder location: "global" (every session) or "project" (one workspace).
// The session endpoints resolve the effective menu (Settings disables + session mutes).

export interface SkillRow {
  name: string;
  description: string;
  instructions: string;
  scope: "global" | "project";
  source: string; // "local" | "uploaded"
  enabled: boolean;
  path: string;
}

export interface SessionSkillRow {
  name: string;
  description: string;
  scope: "global" | "project";
  enabled: boolean; // false = muted for this session only
}

/// A skill fetched from a link, before anything is written. Reviewing this IS the
/// consent step: a skill is instructions the worker will follow, so the actual text
/// goes on screen before it can steer anything.
export interface SkillLinkPreview {
  name: string;
  description: string;
  instructions: string;
  allowed_tools: string[];
  source: string;
  url: string;
  replaces_existing: boolean;
}

/** Fetch a skill from `owner/repo`, a GitHub link, or a URL to a SKILL.md. Writes nothing. */
export async function previewSkillLink(
  source: string,
): Promise<{ ok: boolean; error?: string; preview?: SkillLinkPreview }> {
  const res = await fetch(`${httpBase()}/v1/skills/preview`, jsonPost({ source }));
  return res.json();
}

/** Install the preview the user just read — not the source, so what lands is what they saw. */
export async function installSkillLink(
  preview: SkillLinkPreview,
): Promise<{ ok: boolean; error?: string; name?: string }> {
  const res = await fetch(`${httpBase()}/v1/skills/install`, jsonPost({ preview }));
  return res.json();
}

export const skillUrl = (path = "") => `${httpBase()}/v1/skills${path}`;
export const jsonPost = (body: unknown, method = "POST") => ({
  method,
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(body),
});
