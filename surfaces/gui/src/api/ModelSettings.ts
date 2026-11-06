import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

// App settings: the model and its key, onboarding, layout, approvals and the rest.

export interface SurfaceVisibility {
  /** Always true. */
  cowork: boolean;
  chat: boolean;
  code: boolean;
}

/** Every field after the first block is optional, so the GUI copes with a server older than
 *  the field: an absent value means the control hides or shows its default. */
export interface ModelSettings {
  provider: string;
  model: string;
  models: string[];
  has_key: boolean;
  /** Whether the default model's provider can actually run, whichever provider it is. */
  model_ready: boolean;
  source: "env" | "store" | null;
  onboarded: boolean;
  surfaces: SurfaceVisibility;
  scratch_base: string;
  /** Where the server keeps saved credentials, in the system's own path form. */
  secrets_path: string;

  /** The local model for work nobody is waiting on; empty means background work uses the chat model. */
  small_model?: string;
  /** What memory search embeds with, and the installed models that can do it. Missing on an
   *  older server, where the card hides rather than showing an empty picker. */
  embed_model?: string;
  embed_models?: string[];
  /** Display names from the curated list ({full id → "GLM-5.2 · via Together"}); custom models aren't in it. */
  model_labels?: Record<string, string>;
  /** Context window in tokens for verified models only; it drives the composer's fill meter,
   *  which hides for a model that isn't listed. */
  model_context_windows?: Record<string, number>;
  /** Which leading `x:` on a model id names a provider rather than being part of the name.
   *  It comes from the server, because the client guessing is how "gemma4:12b" became "12b". */
  provider_prefixes?: string[];
  /** How hard to think, with the levels sent alongside: a scale fixed in the client would go
   *  stale the moment the server's changed. */
  effort?: string;
  effort_levels?: string[];

  /** Sidebar layout (§7): "flat" is the assistant accordions, "grouped" the bounded cards per
   *  assistant. A missing value means flat. */
  nav_layout?: "flat" | "grouped";
  /** The number of sessions a sidebar group lists before its "Show more" (1–50, default 5). */
  sessions_peek?: number;

  /** Let a second model call pass actions that are plainly inside what was asked. */
  auto_approve?: boolean;
  /** Let clearly read-only commands (ls, cat, git log) run without a card. */
  auto_allow_read_only_commands?: boolean;
  /** What every assistant is told on every turn, in the user's own words. */
  standing_instructions?: string;
  standing_instructions_max?: number;
  /** Seconds a cloud or local model may stay silent before its turn ends; 0 turns it off. */
  stall_seconds?: number;
  stall_seconds_local?: number;
  /** Days of activity trail to keep; 0 keeps all of it. */
  audit_retention_days?: number;
  /** The assistants' own browser (Settings ▸ Browser). */
  browser_enabled?: boolean;
  browser_allow_loopback?: boolean;
  browser_headed?: boolean;
  /** Whether assistants may draw a view of their own in the conversation (Settings ▸ General). */
  generative_ui?: boolean;

  /** PDF attachments, to save tokens: how models without native PDF reading get them, and the
   *  limits checked when attaching (pages 1–100, default 20; size 1–10 MB, default 10). */
  pdf_fallback?: "text" | "images";
  pdf_max_pages?: number;
  pdf_max_mb?: number;
  /** Compacting long histories (OPE-27). It starts at the smaller of threshold × the context
   *  window (0.10–0.95, default 0.8) and the token cap (default 250,000). The model runs the
   *  summary; "" means the session's own model. */
  compaction_threshold_pct?: number;
  compaction_cap_tokens?: number;
  compaction_model?: string;
}

export interface PdfSettings {
  pdf_fallback: "text" | "images";
  pdf_max_pages: number;
  pdf_max_mb: number;
}

export interface CompactionSettings {
  compaction_threshold_pct: number;
  compaction_cap_tokens: number;
  compaction_model: string;
}

/** POST one settings change and return the server's reply. */
async function saveSetting<T>(path: string, body: unknown): Promise<T> {
  return (await fetch(`${httpBase()}/v1/settings/${path}`, jsonBody("POST", body))).json();
}

/** The PDF settings: fallback and attach limits. */
export function setPdfSettings(patch: Partial<PdfSettings>) {
  return saveSetting<{ ok: boolean; error?: string } & Partial<PdfSettings>>("pdf", patch);
}

/** The compaction overrides: threshold, token cap and summary model. */
export function setCompactionSettings(patch: Partial<CompactionSettings>) {
  return saveSetting<{ ok: boolean; error?: string }>("compaction", patch);
}

/** Pages and size of a PDF data URL, read on this machine, for the composer's attach check. */
export async function inspectPdf(dataUrl: string): Promise<{ ok: boolean; pages?: number; bytes?: number; error?: string }> {
  return (await fetch(`${httpBase()}/v1/attachments/inspect-pdf`, jsonBody("POST", { data_url: dataUrl }))).json();
}

/** The two approval switches. Send only what changed; a missing field stays as it is. */
export function setApprovalSettings(next: { auto_approve?: boolean; auto_allow_read_only_commands?: boolean }) {
  return saveSetting<{ ok: boolean; auto_approve?: boolean; auto_allow_read_only_commands?: boolean; error?: string }>("approvals", next);
}

/** The number of sessions a sidebar group lists before its "Show more". */
export function setSessionsPeek(n: number) {
  return saveSetting<{ ok: boolean; sessions_peek?: number; error?: string }>("sessions-peek", { sessions_peek: n });
}

export function setScratchBase(path: string) {
  return saveSetting<{ ok: boolean; error?: string; scratch_base?: string }>("scratch-base", { path });
}

export function setSurfaces(flags: { chat?: boolean; code?: boolean }) {
  return saveSetting<{ ok: boolean; surfaces: SurfaceVisibility }>("surfaces", flags);
}

/** The sidebar layout, flat or grouped by assistant; getSettings reads it back. */
export function setNavLayout(layout: "flat" | "grouped") {
  return saveSetting<{ ok: boolean; nav_layout?: "flat" | "grouped"; error?: string }>("nav-layout", { nav_layout: layout });
}

/** Sent when a cloud sign-in or sign-out finishes, so the account row (§26) updates without
 *  waiting for the window to regain focus. */
export const CLOUD_CHANGED = "assistant:cloud-changed";
export function announceCloudChanged() {
  window.dispatchEvent(new CustomEvent(CLOUD_CHANGED));
}

/** Sent the first time anything uses the Inbox: an item is parked, or a session switches to
 *  Unattended. The inbox chip on the account row appears then and stays (§26). */
export const INBOX_UNLOCK = "assistant:inbox-unlock";
