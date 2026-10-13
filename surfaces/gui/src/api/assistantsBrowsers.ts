import { fetch, httpBase, jsonOf, openWebSocket, wsBase } from "./neverRejectingJson";

// -- The assistants' browsers ----------------------------------------------------------
//
// Not the browser in the side panel (`BrowserState` below), which is the user's. These are
// the assistants' own: one real Chrome per assistant, signed in to whatever it was signed
// into, watchable, and handed to the user whenever a site needs a person.

/** Who has the wheel, and whether the assistant is waiting on the user. Never a secret. */
export interface ComputerControl {
  holder: "assistant" | "user";
  since: number;
  /** Why the assistant asked for help — what the user is being handed. */
  reason?: string;
  /** The assistant asked for help and nobody has taken the wheel yet. */
  requested: boolean;
  /** The label of a value the assistant is waiting for, if any. */
  secret_wanted?: string;
}

export interface ComputerStatus {
  persona: string;
  name?: string;
  icon?: string;
  /** Whether the assistant was given a browser on its page. */
  granted?: boolean;
  running: boolean;
  url: string;
  title: string;
  control: ComputerControl;
  /** Sign-ins exist that "Sign out of everything" would delete. */
  has_profile: boolean;
  viewers: number;
  last_used: number;
}

export interface ComputersState {
  enabled: boolean;
  allow_loopback: boolean;
  headed: boolean;
  chrome_found: boolean;
  local_only: boolean;
  computers: ComputerStatus[];
}

export type Ok<T = {}> = { ok: boolean; error?: string } & Partial<T>;

export const postJSON = (path: string, body: unknown = {}) =>
  fetch(`${httpBase()}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

export const computerPath = (persona: string) => `/v1/computers/${encodeURIComponent(persona)}`;

export async function getComputers(): Promise<ComputersState | null> {
  const body = await jsonOf(await fetch(`${httpBase()}/v1/computers`));
  if (!Array.isArray(body.computers)) return null;
  return body as ComputersState;
}

export async function getComputer(persona: string): Promise<ComputerStatus | null> {
  const body = await jsonOf(await fetch(`${httpBase()}${computerPath(persona)}`));
  return typeof body.persona === "string" ? (body as ComputerStatus) : null;
}

export async function setComputerSettings(next: {
  browser_enabled?: boolean;
  browser_allow_loopback?: boolean;
  browser_headed?: boolean;
}): Promise<Ok<{ browser_enabled: boolean; browser_allow_loopback: boolean; browser_headed: boolean }>> {
  return jsonOf(await postJSON("/v1/settings/computer", next)) as Promise<
    Ok<{ browser_enabled: boolean; browser_allow_loopback: boolean; browser_headed: boolean }>
  >;
}

/** May assistants draw a view of their own in the conversation. Off by default. */
export async function setGenerativeUi(on: boolean): Promise<Ok<{ generative_ui: boolean }>> {
  return jsonOf(await postJSON("/v1/settings/generative-ui", { on })) as Promise<Ok<{ generative_ui: boolean }>>;
}

/** Give an assistant its own browser, or take it away (closing the one it has open). */
export async function setPersonaBrowser(persona: string, on: boolean): Promise<Ok> {
  return jsonOf(await postJSON(`/v1/personas/${encodeURIComponent(persona)}`, { browser: on })) as Promise<Ok>;
}

/** Close it and keep its sign-ins. */
export async function stopComputer(persona: string): Promise<Ok<{ was_running: boolean }>> {
  return jsonOf(await postJSON(`${computerPath(persona)}/stop`)) as Promise<Ok<{ was_running: boolean }>>;
}

/** Sign out of everything: deletes the profile and every kept screen. Confirm first. */
export async function resetComputer(persona: string): Promise<Ok> {
  return jsonOf(await postJSON(`${computerPath(persona)}/reset`)) as Promise<Ok>;
}

/** take / release the wheel, or decline ("not now") a request for help. */
export async function computerControl(
  persona: string,
  action: "take" | "release" | "decline",
): Promise<Ok<{ control: ComputerControl }>> {
  return jsonOf(await postJSON(`${computerPath(persona)}/control`, { action })) as Promise<Ok<{ control: ComputerControl }>>;
}

/** Open a site in the assistant's browser for the user to sign in on — the wheel theirs. */
export async function openInComputer(persona: string, url: string): Promise<Ok<{ url: string; title: string }>> {
  return jsonOf(await postJSON(`${computerPath(persona)}/open`, { url })) as Promise<Ok<{ url: string; title: string }>>;
}

/** Type the value the assistant asked for straight into its field. Returns only a count. */
export async function supplyComputerSecret(persona: string, value: string): Promise<Ok<{ chars: number }>> {
  return jsonOf(await postJSON(`${computerPath(persona)}/secret`, { value })) as Promise<Ok<{ chars: number }>>;
}

export async function declineComputerSecret(persona: string): Promise<Ok> {
  return jsonOf(await postJSON(`${computerPath(persona)}/secret`, { decline: true })) as Promise<Ok>;
}

/** A screen a finished turn kept, as a data URL; null once it is no longer kept. */
export async function getComputerFrame(persona: string, id: string): Promise<string | null> {
  const body = await jsonOf(await fetch(`${httpBase()}${computerPath(persona)}/frames/${encodeURIComponent(id)}`));
  return typeof body.data_url === "string" ? body.data_url : null;
}

/** The live screen: `frame` / `stopped` / `control` / `error` messages out; input events in. */
export function openComputerSocket(persona: string): WebSocket {
  return openWebSocket(`${wsBase()}/ws/computer/${encodeURIComponent(persona)}`);
}

export interface BrowserState {
  open: boolean;
  url: string;
  title: string;
  status: string;
  last_action: string;
  last_result: string;
  last_error: string;
  screenshot_data_url: string;
  updated_at: string | null;
  controls: any[];
}

export async function getBrowserState(): Promise<BrowserState> {
  const res = await fetch(`${httpBase()}/v1/browser/state`);
  return res.json();
}

export async function takeBrowserScreenshot(): Promise<BrowserState & { ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/browser/screenshot`, { method: "POST" });
  return res.json();
}

export async function closeBrowser(): Promise<{ ok?: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/browser/close`, { method: "POST" });
  return res.json();
}
