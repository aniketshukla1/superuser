import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

// Skills (SKILLS-SPEC §4). Where a skill's folder lives sets its scope: "global" applies to
// every session, "project" to one workspace. The session endpoints work out what a session
// actually gets, after Settings switches and per-session mutes.

export interface SkillRow {
  name: string;
  description: string;
  instructions: string;
  path: string;
  scope: "global" | "project";
  /** "local" or "uploaded". */
  source: string;
  enabled: boolean;
}

export interface SessionSkillRow {
  name: string;
  description: string;
  scope: "global" | "project";
  /** false when muted for this session only. */
  enabled: boolean;
}

/** A skill read from a link, before anything is written. Looking at it is the consent step:
 *  a skill is instructions the worker will follow, so its real text is shown before it can
 *  steer anything. */
export interface SkillLinkPreview {
  name: string;
  description: string;
  instructions: string;
  allowed_tools: string[];
  url: string;
  source: string;
  replaces_existing: boolean;
}

export const skillUrl = (path = "") => `${httpBase()}/v1/skills${path}`;
export const jsonPost = (body: unknown, method = "POST") => jsonBody(method, body);

/** Read a skill from `owner/repo`, a GitHub link or a URL to a SKILL.md, without writing. */
export async function previewSkillLink(source: string): Promise<{ ok: boolean; error?: string; preview?: SkillLinkPreview }> {
  const res = await fetch(skillUrl("/preview"), jsonPost({ source }));
  return res.json();
}

/** Install the preview the user just read, not the source again, so what lands is what they saw. */
export async function installSkillLink(preview: SkillLinkPreview): Promise<{ ok: boolean; error?: string; name?: string }> {
  const res = await fetch(skillUrl("/install"), jsonPost({ preview }));
  return res.json();
}
