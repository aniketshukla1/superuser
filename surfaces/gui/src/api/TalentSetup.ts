import { fetch, httpBase, jsonOf } from "./neverRejectingJson";
import { jsonPost } from "./skills";

/** Talent Scout: candidates checked against the person's ATS export and past shortlists, with
 *  outreach drafts nobody sends. */
export interface TalentSetup {
  /** Where the ATS export (a CSV) is; empty when none is set. */
  applicants: string;
  /** How many people it holds; null when none is set or it can't be read. */
  people: number | null;
  /** Why the export can't be read. */
  error: string | null;
  /** A message the person has sent, so drafts sound like them. */
  voice: string;
  shortlisted: number;
}

export async function getTalent(): Promise<TalentSetup | null> {
  const res = await fetch(`${httpBase()}/v1/talent`);
  const j = res.ok ? await jsonOf(res) : {};
  return typeof j.applicants === "string" ? (j as TalentSetup) : null;
}

export async function setTalent(body: {
  talent_applicants?: string;
  talent_voice?: string;
}): Promise<Partial<TalentSetup> & { ok?: boolean; error?: string | null }> {
  const res = await fetch(`${httpBase()}/v1/talent`, jsonPost(body));
  return res.json();
}

/** The system's file picker; `path` is null when it was cancelled. */
export async function pickTalentExport(): Promise<{ path: string | null }> {
  const res = await fetch(`${httpBase()}/v1/talent/pick`, jsonPost({}));
  return res.json();
}

export async function forgetTalentShortlist(): Promise<{ ok: boolean; forgotten?: number; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/talent/forget`, jsonPost({}));
  return res.json();
}
