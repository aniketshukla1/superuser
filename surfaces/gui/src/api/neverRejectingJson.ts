// The client's line to the local server: where the server is, how a request is signed, and
// the promise that reading a reply never throws. Every helper in api/ calls `fetch` below,
// so the token header is added in exactly one place.

export declare const __SUPERUSER_DEV_TOKEN__: string;

/** A setting from the first place that has it: a global the desktop shell injects (it picks
 *  the sidecar's port at launch), then the Vite environment, then the local default. That
 *  keeps one codebase: `npm run dev` in a browser talks to 8765, the desktop app to its
 *  sidecar. */
function setting(global: string, env: string, fallback: string): string {
  return (globalThis as any)[global] || (import.meta as any).env?.[env] || fallback;
}

export const httpBase = (): string => setting("__SUPERUSER_HTTP__", "VITE_SUPERUSER_HTTP", "http://127.0.0.1:8765");
export const wsBase = (): string => setting("__SUPERUSER_WS__", "VITE_SUPERUSER_WS", "ws://127.0.0.1:8765");
export const apiToken = (): string =>
  setting(
    "__SUPERUSER_API_TOKEN__",
    "VITE_SUPERUSER_API_TOKEN",
    typeof __SUPERUSER_DEV_TOKEN__ === "string" ? __SUPERUSER_DEV_TOKEN__ : "",
  );

/** The request options with the launch token added. */
function signed(init: RequestInit): RequestInit {
  const headers = new Headers(init.headers);
  const token = apiToken();
  if (token !== "") headers.set("X-Superuser-Token", token);
  return { ...init, headers };
}

/** Every local call goes through here: signed with the launch token, and with a reply whose
 *  `json()` cannot throw. */
export async function fetch(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  const response = await globalThis.fetch(input, signed(init));
  return neverRejectingJson(response);
}

/** Request options that send `body` as JSON. */
export function jsonBody(method: string, body: unknown): RequestInit {
  return { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) };
}

// Over a hundred helpers end in `return res.json()`, and a parse that rejects in any of them
// shows up as a page error with no stack in whichever caller left out a `.catch`
// (UX-REPORT §9). So the one place they all pass through returns a Response whose `json()`
// gives `{}` for a cut-off or non-JSON body. Callers then find the field missing and take the
// "not ok" path they already have for an error reply; the console keeps the detail.
export function neverRejectingJson(res: Response): Response {
  const original = res.json.bind(res);
  const safe = async (): Promise<unknown> => {
    try {
      return await original();
    } catch (error) {
      console.warn(`unparseable response from ${res.url}:`, error);
      return {};
    }
  };
  (res as { json: () => Promise<unknown> }).json = safe;
  return res;
}

/** The body as an object, always. A cut-off or non-JSON reply reads as `{}`, so `.ok` is
 *  falsy and the caller takes its failure branch rather than throwing. */
export async function jsonOf(res: Response): Promise<Record<string, any>> {
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    // Read as no body at all.
  }
  return body !== null && typeof body === "object" ? (body as Record<string, any>) : {};
}

/** One list field of the body, or [] when it isn't a list. A list endpoint that answers with
 *  an error, a 500 or a cut-off body must never pass `undefined` to a component about to
 *  `.map` it: with no error boundary above, that one throw takes the window down
 *  (UX-REPORT §1). */
export async function listOf<T>(res: Response, key: string): Promise<T[]> {
  const value = (await jsonOf(res))[key];
  return Array.isArray(value) ? (value as T[]) : [];
}

export function openWebSocket(url: string): WebSocket {
  const token = apiToken();
  return token === "" ? new WebSocket(url) : new WebSocket(url, ["superuser", token]);
}

export interface Health {
  model: string;
  status: string;
  default_workspace: string | null;
  /** How saved credentials are kept: "encrypted", "empty", "plain", or "locked: <why>". */
  secrets?: string;
}

/** Move a saved-keys file that won't open out of the way: renamed, never deleted. */
export async function setAsideLockedSecrets(): Promise<{ ok: boolean; kept_at?: string; error?: string }> {
  const reply = await jsonOf(await fetch(`${httpBase()}/v1/secrets/set-aside`, { method: "POST" }));
  return { ok: reply.ok === true, kept_at: reply.kept_at, error: reply.error };
}

export interface RecentWorkspace {
  name: string;
  path: string;
  exists: boolean;
}

export interface WorkspaceCommandTrust {
  workspace: string;
  trusted: boolean;
  required: boolean;
  requested_commands: string[];
  exists?: boolean;
}

export async function getHealth(): Promise<Health> {
  const body = await jsonOf(await fetch(`${httpBase()}/v1/health`));
  // `/v1/health` is the only route that needs no token, so it answers a bare liveness check
  // with `{ok, service, status}` and nothing more: it must not give the workspace path to any
  // website probing the port. The full answer needs the token, which the shell injects a
  // moment after the server starts, so a cold start can get here first. The short answer
  // therefore means "not ready", and the caller's retry loop comes back once the token is in.
  if (!("model" in body)) throw new Error("sidecar is up but not yet authenticated");
  return body as Health;
}

export async function getRecentWorkspaces(): Promise<RecentWorkspace[]> {
  return listOf(await fetch(`${httpBase()}/v1/workspaces/recent`), "workspaces");
}

/** Ask the local sidecar to show the system folder picker, since a browser can't learn an
 *  absolute path from a web file dialog. Waits for the choice; null if cancelled or unavailable. */
export async function pickFolderViaServer(): Promise<string | null> {
  try {
    const reply = await jsonOf(await fetch(`${httpBase()}/v1/workspaces/pick`, { method: "POST" }));
    return reply.ok && reply.path ? reply.path : null;
  } catch {
    return null;
  }
}

export type OpenedWorkspace = {
  ok: boolean;
  path: string;
  error?: string;
  git_branch?: string | null;
  command_trust?: WorkspaceCommandTrust;
};

export async function openWorkspace(path: string, create = false): Promise<OpenedWorkspace> {
  const res = await fetch(`${httpBase()}/v1/workspaces/open`, jsonBody("POST", { path, create }));
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
