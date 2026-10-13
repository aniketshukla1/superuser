export declare const __COWORKER_DEV_TOKEN__: string;

// Endpoint resolution order: runtime-injected globals (Tauri sets `window.__COWORKER_HTTP__`
// for its dynamically-chosen sidecar port) → Vite env → the 127.0.0.1:8765 dev default. This
// keeps a single codebase: browser `npm run dev` hits 8765; the desktop shell hits its sidecar.
export const httpBase = (): string =>
  (globalThis as any).__COWORKER_HTTP__ ||
  (import.meta as any).env?.VITE_COWORKER_HTTP ||
  "http://127.0.0.1:8765";
export const wsBase = (): string =>
  (globalThis as any).__COWORKER_WS__ ||
  (import.meta as any).env?.VITE_COWORKER_WS ||
  "ws://127.0.0.1:8765";
export const apiToken = (): string =>
  (globalThis as any).__COWORKER_API_TOKEN__ ||
  (import.meta as any).env?.VITE_COWORKER_API_TOKEN ||
  (typeof __COWORKER_DEV_TOKEN__ === "string" ? __COWORKER_DEV_TOKEN__ : "");

// All local REST calls pass through this module, so a module-local wrapper applies launch
// authentication without asking every endpoint helper to remember the security header.
export const fetch = async (
  input: RequestInfo | URL,
  init: RequestInit = {},
): Promise<Response> => {
  const headers = new Headers(init.headers);
  const token = apiToken();
  if (token) headers.set("X-Superuser-Token", token);
  return neverRejectingJson(await globalThis.fetch(input, { ...init, headers }));
};

// Over a hundred helpers in this module end in `return res.json()`, and a rejected
// parse from any of them lands as a stackless page error in whichever caller forgot a
// `.catch` (UX-REPORT §9). Rather than ask every one of them to remember, the single
// chokepoint they already share hands back a Response whose `json()` resolves to `{}`
// on a truncated or non-JSON body. Readers then see an absent field and take their
// existing "not ok" path, which is what they do for an error envelope anyway. The
// console keeps the detail for a bug report.
export function neverRejectingJson(res: Response): Response {
  const parse = res.json.bind(res);
  (res as { json: () => Promise<unknown> }).json = async () => {
    try {
      return await parse();
    } catch (e) {
      console.warn(`unparseable response from ${res.url}:`, e);
      return {};
    }
  };
  return res;
}

// A list endpoint that answers with an error body, a 500, or truncated JSON must not
// hand `undefined` to a component that is about to call `.map` on it — with no error
// boundary in the tree, one such throw unmounts the whole window (UX-REPORT §1). The
// `?? []` most unwrappers already carry does not cover a `res.json()` that *throws*,
// so the parse lives here too.
export async function listOf<T>(res: Response, key: string): Promise<T[]> {
  const list = (await jsonOf(res))[key];
  return Array.isArray(list) ? (list as T[]) : [];
}

/** The same guarantee for a non-list body: an object, always. A truncated or
 *  non-JSON response reads as `{}`, so `.ok` is falsy and callers take their
 *  failure branch instead of rejecting into whoever forgot a `.catch`. */
export async function jsonOf(res: Response): Promise<Record<string, any>> {
  const body = await res.json().catch(() => null);
  return body && typeof body === "object" ? body : {};
}

export const openWebSocket = (url: string): WebSocket => {
  const token = apiToken();
  return token
    ? new WebSocket(url, ["superuser", token])
    : new WebSocket(url);
};

export interface Health {
  status: string;
  default_workspace: string | null;
  model: string;
  /** How saved credentials are kept: "encrypted", "empty", "plain", or "locked: <why>". */
  secrets?: string;
}

/** Move a saved-keys file that won't open out of the way. Renamed, never deleted. */
export async function setAsideLockedSecrets(): Promise<{ ok: boolean; kept_at?: string; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/secrets/set-aside`, { method: "POST" });
  const out = await jsonOf(res);
  return { ok: out.ok === true, kept_at: out.kept_at, error: out.error };
}

export interface RecentWorkspace {
  path: string;
  name: string;
  exists: boolean;
}

export interface WorkspaceCommandTrust {
  workspace: string;
  requested_commands: string[];
  trusted: boolean;
  required: boolean;
  exists?: boolean;
}

export async function getHealth(): Promise<Health> {
  const res = await fetch(`${httpBase()}/v1/health`);
  const body = await jsonOf(res);
  // `/v1/health` is the one unauthenticated route, so it answers a bare liveness
  // check with `{ok, service, status}` and nothing else — it must not hand the
  // workspace path to any website that pokes at the port. The fuller answer needs
  // the token, which the shell injects into the window a moment after the server
  // is up, so on a cold start we can beat it here. Treat the short answer as
  // "not ready" and let the caller's retry loop come back once the token lands.
  if (!("model" in body)) throw new Error("sidecar is up but not yet authenticated");
  return body as Health;
}

export async function getRecentWorkspaces(): Promise<RecentWorkspace[]> {
  const res = await fetch(`${httpBase()}/v1/workspaces/recent`);
  return listOf(res, "workspaces");
}

/** Ask the LOCAL sidecar to open the OS folder picker — the browser GUI can't obtain absolute
 * paths from web file dialogs. Blocks until the user picks or cancels; null on cancel/unavailable. */
export async function pickFolderViaServer(): Promise<string | null> {
  try {
    const res = await fetch(`${httpBase()}/v1/workspaces/pick`, { method: "POST" });
    const d = await jsonOf(res);
    return d.ok && d.path ? d.path : null;
  } catch {
    return null;
  }
}

export async function openWorkspace(
  path: string,
  create = false,
): Promise<{
  path: string;
  ok: boolean;
  error?: string;
  git_branch?: string | null;
  command_trust?: WorkspaceCommandTrust;
}> {
  const res = await fetch(`${httpBase()}/v1/workspaces/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, create }),
  });
  return res.json();
}

export interface WorkerType {
  name: string;
  description: string;
  builtin: boolean;
  /** null = inherits the parent assistant's tools; a list = exactly these. */
  tools: string[] | null;
  instructions: string;
}

export interface WorkerRoster {
  id: string;
  name: string;
  subagents: string[];
}

/** The worker types an assistant can delegate to, and which assistants staff a named team. */
export async function getWorkerTypes(): Promise<{ types: WorkerType[]; rosters: WorkerRoster[] }> {
  const res = await fetch(`${httpBase()}/v1/subagents`);
  const j = await jsonOf(res);
  return { types: j.types ?? [], rosters: j.rosters ?? [] };
}

export interface BoardReview {
  by: string;
  pass: boolean;
  evidence: string;
}

export interface BoardItem {
  id: string;
  title: string;
  detail: string;
  /** What has to be true for this to be done. Empty means nobody wrote one. */
  acceptance: string;
  state: "open" | "in_progress" | "blocked" | "review" | "done" | "canceled";
  assignee: string | null;
  /** Who put it up for review — the one actor barred from passing it. */
  builder: string | null;
  notes: string[];
  reviews: BoardReview[];
}
