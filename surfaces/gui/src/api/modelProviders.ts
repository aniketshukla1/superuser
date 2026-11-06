import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

// Model providers: OpenAI, Ollama and the rest.

export interface ProviderField {
  key: string;
  label: string;
  help: string;
  placeholder: string;
  secret: boolean;
  required: boolean;
  /** An editable starting value, such as an OpenAI-compatible vendor's endpoint. */
  default?: string;
  /** When present the field is a segmented choice, not a text box. `tag` is a small badge
   *  ("Easiest"), `desc` the line at the top of the method's panel, `command` a terminal
   *  command to copy. */
  choices?: { value: string; label: string; tag?: string; desc?: string; command?: string }[];
  /** Show the field only while these other fields hold these values. */
  show_when?: Record<string, string> | null;
}

export interface ProviderInfo {
  name: string;
  title: string;
  /** A short note shown below the title, e.g. "Uses X's OpenAI-compatible API…". */
  blurb?: string;
  fields: ProviderField[];
  needs_key: boolean;
  configured: boolean;
  /** Stored values that aren't secret (base_url, say), to pre-fill the form. */
  values: Record<string, string>;
  /** Bare model names offered when adding a model. */
  suggested_models: string[];
  /** The model pre-filled for this provider (qwen3-coder:30b, say). */
  recommended_model: string | null;
  /** When the key was last saved, as an ISO date; absent when it comes from the environment. */
  key_set_at?: string | null;
  /** When the provider last answered a completion, in seconds since the epoch. */
  last_used_at?: number | null;
}

const providersUrl = (rest = "") => `${httpBase()}/v1/providers${rest}`;

export async function getProviders(): Promise<ProviderInfo[]> {
  const res = await fetch(providersUrl());
  // Callers map and search this, and an error reply is no list: a 401 that arrived before the
  // launch token reached the window once brought the whole Models tab down.
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    // Not a list.
  }
  return Array.isArray(body) ? (body as ProviderInfo[]) : [];
}

export async function setProvider(
  name: string,
  fields: Record<string, string>,
): Promise<{ ok: boolean; error?: string; provider?: string; recommended_model?: string | null }> {
  const res = await fetch(providersUrl(), jsonBody("POST", { name, fields }));
  return res.json();
}

/** Forget what is stored for a provider (Settings ▸ Models, "Remove key…"). */
export async function removeProvider(name: string): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(providersUrl(`/${encodeURIComponent(name)}`), { method: "DELETE" });
  return res.json();
}

/** Check the credentials against the provider without saving them, when the user presses Test. */
export async function verifyProvider(name: string, fields: Record<string, string>): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(providersUrl("/verify"), jsonBody("POST", { name, fields }));
  return res.json();
}

/** Prefixes that give away who issued a key, checked in order (the server's detect_provider
 *  makes the same guess). `sk-` comes last because the two before it start with it too. */
const KEY_PREFIXES: [string, string][] = [
  ["sk-ant-", "anthropic"],
  ["sk-or-", "openrouter"],
  ["AIza", "gemini"],
  ["sk-", "openai"],
  ["sk_", "openai"],
];

/** A guess at the provider from the shape of an API key, before anything is sent. */
export function detectProvider(apiKey: string): string | null {
  const key = (apiKey ?? "").trim();
  if (key === "") return null;
  const hit = KEY_PREFIXES.find(([prefix]) => key.startsWith(prefix));
  return hit ? hit[1] : null;
}
