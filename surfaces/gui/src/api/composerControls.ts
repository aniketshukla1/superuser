import { fetch, httpBase, jsonBody, listOf } from "./neverRejectingJson";
import { jsonPost, skillUrl, type SessionSkillRow, type SkillRow } from "./skills";

// What the composer's controls read and change: effort, the microphone, usage, and skills.

/** How hard the model is asked to think. The names come from the server, not from here:
 *  a scale baked into the client goes stale the moment the server's changes. */
export interface EffortState {
  effort: string;
  levels: string[];
}

export async function setEffort(effort: string): Promise<{ ok: boolean; effort?: string; thinking_budget?: number | null; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/settings/effort`, jsonBody("POST", { effort }));
  return res.json();
}

export interface InputDevice {
  name: string;
  is_default: boolean;
  /** Enumerates but will not open — a virtual device whose host app is closed. */
  usable: boolean;
}

export interface VoiceDevices {
  devices: InputDevice[];
  /** Empty = follow the system default. */
  selected: string;
  /** False when the chosen mic is unplugged, so the menu can say the default is standing in. */
  selected_present: boolean;
  hold_to_record: boolean;
}

export async function getVoiceDevices(): Promise<VoiceDevices> {
  const res = await fetch(`${httpBase()}/v1/voice/devices`);
  const v = (await res.json()) as Partial<VoiceDevices>;
  return {
    devices: v.devices ?? [],
    selected: v.selected ?? "",
    selected_present: v.selected_present ?? true,
    hold_to_record: v.hold_to_record ?? false,
  };
}

export async function setVoiceDevice(
  patch: { device?: string; hold_to_record?: boolean },
): Promise<{ ok: boolean }> {
  const res = await fetch(`${httpBase()}/v1/voice/device`, jsonBody("POST", patch));
  return res.json();
}

/** One window of spend, from this app's own history — NOT a subscription meter.
 *  `uncounted` is the number of turns in the window that reported no usage, which
 *  makes `tokens` a floor rather than a total. Read it before presenting the number. */
export interface UsageWindow {
  input: number;
  output: number;
  tokens: number;
  counted: number;
  uncounted: number;
  /** Unix seconds. */
  resets_at: number;
}

export interface UsageRollup {
  today: UsageWindow;
  week: UsageWindow;
  now: number;
}

export const emptyWindow = (): UsageWindow => ({
  input: 0, output: 0, tokens: 0, counted: 0, uncounted: 0, resets_at: 0,
});

export async function getUsageRollup(): Promise<UsageRollup> {
  const res = await fetch(`${httpBase()}/v1/usage/rollup`);
  const v = (await res.json()) as Partial<UsageRollup>;
  return { today: v.today ?? emptyWindow(), week: v.week ?? emptyWindow(), now: v.now ?? 0 };
}

const workspaceQuery = (workspace?: string) => (workspace ? `?workspace=${encodeURIComponent(workspace)}` : "");
const sessionSkillsUrl = (sessionId: string) => `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/skills`;

export async function listSkills(workspace?: string): Promise<SkillRow[]> {
  return listOf(await fetch(skillUrl(workspaceQuery(workspace))), "skills");
}

/** The Skills page's switch: on or off everywhere. Nothing else about a skill changes here;
 *  creating, editing, uploading and revealing one were never served. */
export async function updateSkill(name: string, patch: { enabled: boolean }): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(skillUrl(`/${encodeURIComponent(name)}`), jsonPost(patch, "PATCH"))).json();
}

export async function deleteSkill(name: string, workspace?: string): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(skillUrl(`/${encodeURIComponent(name)}${workspaceQuery(workspace)}`), { method: "DELETE" })).json();
}

/** The skills on offer in one conversation. `agent` names the assistant it is for: a session
 *  only reaches the server with its first turn, and without this a brand-new conversation's
 *  menu couldn't be narrowed to that assistant's own procedures, so it would offer them all. */
export async function sessionSkills(sessionId: string, workspace?: string, agent?: string): Promise<SessionSkillRow[]> {
  const filters = new URLSearchParams();
  if (workspace) filters.set("workspace", workspace);
  if (agent) filters.set("agent", agent);
  const query = filters.toString();
  return listOf(await fetch(sessionSkillsUrl(sessionId) + (query ? `?${query}` : "")), "skills");
}

/** Mute or unmute a skill for one conversation; `clear` drops the override altogether. */
export async function setSessionSkill(
  sessionId: string,
  skill: string,
  enabled: boolean,
  opts: { clear?: boolean; workspace?: string } = {},
): Promise<{ skills?: SessionSkillRow[]; ok?: boolean; error?: string }> {
  const body: Record<string, unknown> = { skill, enabled };
  if (opts.clear) body.clear = true;
  if (opts.workspace) body.workspace = opts.workspace;
  return (await fetch(sessionSkillsUrl(sessionId), jsonPost(body))).json();
}
