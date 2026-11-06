import { fetch, httpBase, jsonBody, jsonOf, listOf } from "./neverRejectingJson";
import { announcePersonasChanged, type Persona, type PersonaConsent } from "./personas";

// The knowledge base, memory corrections, and installing and managing assistants.
//
// Knowledge is the topic axis of memory: notes keep what came in and the journal keeps when,
// while a page is a subject the agent decided to write up and goes on editing.

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

/** Fix the core memory by hand; it is a markdown file that belongs to the user. */
export async function setCoreMemory(body: string): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/journal/core`, jsonBody("PUT", { body }));
  return res.ok && (await jsonOf(res)).ok === true;
}

const personaUrl = (id = "", rest = "") => `${httpBase()}/v1/personas${id ? `/${encodeURIComponent(id)}` : ""}${rest}`;
type PersonasReply = { ok: boolean; personas?: Persona[]; error?: string };

/** The server's reply with `ok` made strict, announcing the change when `changed` says so. */
async function settled(res: Response, changed: (reply: Record<string, any>) => boolean): Promise<any> {
  const reply = await jsonOf(res);
  if (changed(reply)) announcePersonasChanged();
  return { ...reply, ok: reply.ok === true };
}

export async function getPersonas(): Promise<Persona[]> {
  return listOf<Persona>(await fetch(personaUrl()), "personas");
}

export type PersonaUpdate = {
  enabled?: boolean;
  surfaced?: boolean;
  default?: boolean;
  /** How far the assistant may go without asking. */
  default_permission_mode?: string;
  /** Worker types it staffs when it delegates; empty means whatever is installed. */
  subagents?: string[];
  /** Other assistants it may hand work to; empty means none. */
  asks?: string[];
  /** Whether it gets a browser of its own (Settings ▸ Browser). */
  browser?: boolean;
  /** The AG-UI address of a remote agent that takes its turns; empty means it runs here. */
  remote?: string;
  /** The token that remote agent issued. Empty clears it; it is never sent back. */
  remote_token?: string;
};

export async function updatePersona(id: string, body: PersonaUpdate): Promise<PersonasReply> {
  // Anything but an explicit failure may have changed the list, so listeners refresh.
  return settled(await fetch(personaUrl(id), jsonBody("POST", body)), (reply) => reply.ok !== false);
}

/** Remove an assistant that didn't ship with the app (its snapshot and its state). Works on
 *  this machine alone, signed in or not. */
export async function deletePersona(id: string): Promise<PersonasReply> {
  return settled(await fetch(personaUrl(id), { method: "DELETE" }), (reply) => Boolean(reply.ok));
}

/** A card from the cloud gallery: metadata only. The manifest itself is fetched by the server
 *  at install time and goes through the usual consent step. */
export interface GalleryPersona {
  slug: string;
  version: number;
  publisher: string;
  name: string;
  tagline: string;
  description: string;
  icon: string;
  family: string;
  workspace: string;
  risk_summary: string;
  recommended_connectors: string[];
  /** Marked by the publisher for the gallery's featured row. */
  featured?: boolean;
}

export async function getCloudGallery(): Promise<{ ok: boolean; personas: GalleryPersona[]; error?: string }> {
  return (await fetch(`${httpBase()}/v1/cloud/gallery`)).json();
}

/** One gallery assistant's own page. `capabilities` is this app's consent summary, worked out
 *  from the manifest with the same parser as installing, so the page lists precisely what an
 *  install would ask the user to agree to. */
export interface GalleryDetail {
  ok: boolean;
  error?: string;
  card?: GalleryPersona & { pitch_markdown: string };
  capabilities?: {
    tools: string[];
    risk: string[];
    mcp: string[];
    connectors: boolean;
    messaging: boolean;
    recommended_mode: string;
    recommended_models: string[];
  };
  recommends?: { kind: string; ref: string; reason: string; tier: string }[];
}

export async function getCloudGalleryDetail(slug: string): Promise<GalleryDetail> {
  return (await fetch(`${httpBase()}/v1/cloud/gallery/${encodeURIComponent(slug)}`)).json();
}

export async function installPersona(body: { dir?: string; git_url?: string; gallery_slug?: string }): Promise<PersonasReply & { consent?: PersonaConsent[] }> {
  return settled(await fetch(personaUrl("", "/install"), jsonBody("POST", body)), (reply) => Boolean(reply.ok));
}
