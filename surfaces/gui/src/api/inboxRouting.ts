import type { RecentChannel } from "./channelSubscriptions";
import type { ModelSettings } from "./ModelSettings";
import { fetch, httpBase, jsonBody, listOf } from "./neverRejectingJson";

// Where Unattended approvals and questions are copied to, channel subscriptions, and the
// model settings the Models tab writes.

const v1 = (path: string) => `${httpBase()}/v1/${path}`;
const send = async (path: string, body: unknown) => (await fetch(v1(path), jsonBody("POST", body))).json();

export interface InboxBinding {
  name: string;
  /** The platform, e.g. "slack"; null keeps it to the in-app Inbox. */
  channel: string | null;
  /** The chat_id there, e.g. "C0BEJNCQQ8Y". */
  target: string;
}

export async function getInboxRouting(): Promise<InboxBinding[]> {
  return listOf(await fetch(v1("inbox/routing")), "bindings");
}

export function setInboxBinding(name: string, channel: string | null, target: string): Promise<{ ok: boolean; bindings?: InboxBinding[]; error?: string }> {
  return send("inbox/routing/binding", { name, channel, target });
}

/** A message that arrived with nowhere to go. */
export interface UnroutedItem {
  ts: number;
  source: string;
  sender: string;
  text: string;
  reason: string;
}

export async function getUnrouted(): Promise<UnroutedItem[]> {
  return listOf(await fetch(v1("unrouted")), "items");
}

export async function getRecentChannels(): Promise<RecentChannel[]> {
  return listOf(await fetch(v1("channels/recent")), "channels");
}

export function subscribeChannel(sessionId: string, channel: string): Promise<{ ok: boolean; channel?: string; error?: string }> {
  return send("subscriptions", { session_id: sessionId, channel });
}

export function unsubscribeChannel(sessionId: string, channel: string): Promise<{ ok: boolean; removed?: boolean }> {
  return send("subscriptions/remove", { session_id: sessionId, channel });
}

const unattendedPath = (sessionId: string) => `sessions/${encodeURIComponent(sessionId)}/unattended`;

export async function getUnattended(sessionId: string): Promise<boolean> {
  let body: any = null;
  try {
    body = await (await fetch(v1(unattendedPath(sessionId)))).json();
  } catch {
    // An unreadable answer counts as "attended".
  }
  return body?.unattended === true;
}

export function setUnattended(sessionId: string, unattended: boolean): Promise<{ ok: boolean; unattended: boolean }> {
  return send(unattendedPath(sessionId), { unattended });
}

export async function getSettings(): Promise<ModelSettings> {
  return (await fetch(v1("settings"))).json();
}

type ModelReply = { ok: boolean; error?: string; model?: string };

export function setModelKey(apiKey: string): Promise<{ ok: boolean; error?: string; has_key?: boolean; source?: string }> {
  return send("settings/model-key", { api_key: apiKey });
}

/** The local model for background work. "" clears it, and everything then uses the chat
 *  model, which is also what an install that never sets one does. */
export function setSmallModel(model: string): Promise<ModelReply> {
  return send("settings/small-model", { model });
}

/** The model memory search embeds with. The server refuses one that can't embed, and says so,
 *  instead of saving it and quietly getting worse. */
export function setEmbedModel(model: string): Promise<ModelReply> {
  return send("settings/embed-model", { model });
}

export function setDefaultModel(model: string): Promise<ModelReply> {
  return send("settings/default-model", { model });
}

export function addModel(model: string): Promise<ModelSettings & { ok: boolean; error?: string }> {
  return send("settings/models/add", { model });
}

export function removeModel(model: string): Promise<ModelSettings & { ok: boolean }> {
  return send("settings/models/remove", { model });
}

export function setOnboarded(value: boolean): Promise<{ ok: boolean; onboarded: boolean }> {
  return send("settings/onboarded", { value });
}
