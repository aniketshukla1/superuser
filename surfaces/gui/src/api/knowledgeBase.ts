import { fetch, httpBase, jsonOf, listOf } from "./neverRejectingJson";
import { announcePersonasChanged, type Persona, type PersonaConsent } from "./personas";

// ---- Knowledge base (the topic axis) -------------------------------------------
// Notes record what arrived and the journal records when; a page is a subject the
// agent chose to write up and keeps editing.

export interface KnowledgePage {
  slug: string;
  title: string;
  category: string;
  body: string;
  updated_at: string;
  /** `[[Other page]]` references found in the body. */
  links: string[];
}

export async function getKnowledge(q?: string): Promise<KnowledgePage[]> {
  const qs = q?.trim() ? `?q=${encodeURIComponent(q.trim())}` : "";
  const res = await fetch(`${httpBase()}/v1/knowledge${qs}`);
  return res.ok ? (await jsonOf(res)).pages ?? [] : [];
}

export async function deleteKnowledgePage(slug: string): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/knowledge/${encodeURIComponent(slug)}`, { method: "DELETE" });
  return res.ok && (await jsonOf(res)).ok === true;
}

/** Put back whatever the last self-evolution pass changed. */
export async function undoEvolution(): Promise<{ ok: boolean; restored?: number; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/journal/undo`, { method: "POST" });
  if (!res.ok) return { ok: false, error: `HTTP ${res.status}` };
  return res.json();
}

/** Correct the core memory by hand — it's a markdown file the user owns. */
export async function setCoreMemory(body: string): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/journal/core`, {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ body }),
  });
  return res.ok && (await jsonOf(res)).ok === true;
}

export async function getPersonas(): Promise<Persona[]> {
  const res = await fetch(`${httpBase()}/v1/personas`);
  return listOf<Persona>(res, "personas");
}

export async function updatePersona(
  id: string,
  body: {
    enabled?: boolean;
    surfaced?: boolean;
    default?: boolean;
    /** How far this assistant may act without asking. */
    default_permission_mode?: string;
    /** Worker types it staffs when it delegates. Empty = whatever is installed. */
    subagents?: string[];
    /** Which other assistants it may hand work to. Empty = none. */
    asks?: string[];
    /** Whether it has a browser of its own (Settings ▸ Browser). */
    browser?: boolean;
    /** The AG-UI address of a remote agent that does its turns. Empty = runs here. */
    remote?: string;
    /** The token that remote agent issued. Empty clears it. Never read back. */
    remote_token?: string;
  },
): Promise<{ ok: boolean; personas?: Persona[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/personas/${encodeURIComponent(id)}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const out = await jsonOf(res);
  if (out.ok !== false) announcePersonasChanged();
  return { ...out, ok: out.ok === true };
}

/** Uninstall a non-builtin persona (its snapshot + state). Local; works signed out. */
export async function deletePersona(
  id: string,
): Promise<{ ok: boolean; personas?: Persona[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/personas/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
  const out = await jsonOf(res);
  if (out.ok) announcePersonasChanged();
  return { ...out, ok: out.ok === true };
}

// A curated persona card from the cloud gallery (metadata only — the manifest
// is fetched server-side at install and runs through the normal consent flow).
export interface GalleryPersona {
  slug: string;
  version: number;
  name: string;
  icon: string;
  tagline: string;
  description: string;
  family: string;
  workspace: string;
  publisher: string;
  recommended_connectors: string[];
  risk_summary: string;
  featured?: boolean; // publisher-flagged for the gallery's featured carousel
}

export async function getCloudGallery(): Promise<{
  ok: boolean;
  personas: GalleryPersona[];
  error?: string;
}> {
  const res = await fetch(`${httpBase()}/v1/cloud/gallery`);
  return res.json();
}

// Solo page for one gallery assistant. `capabilities` is the desktop's own
// consent summary derived from the manifest (same parser as install), so the
// page shows exactly what installing would ask the user to approve.
export interface GalleryDetail {
  ok: boolean;
  error?: string;
  card?: GalleryPersona & { pitch_markdown: string };
  capabilities?: {
    tools: string[];
    risk: string[];
    connectors: boolean;
    mcp: string[];
    messaging: boolean;
    recommended_mode: string;
    recommended_models: string[];
  };
  recommends?: { kind: string; ref: string; reason: string; tier: string }[];
}

export async function getCloudGalleryDetail(slug: string): Promise<GalleryDetail> {
  const res = await fetch(`${httpBase()}/v1/cloud/gallery/${encodeURIComponent(slug)}`);
  return res.json();
}

export async function installPersona(
  body: { dir?: string; git_url?: string; gallery_slug?: string },
): Promise<{ ok: boolean; consent?: PersonaConsent[]; personas?: Persona[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/personas/install`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const out = await jsonOf(res);
  if (out.ok) announcePersonasChanged();
  return { ...out, ok: out.ok === true };
}
