import { fetch, httpBase, listOf } from "./neverRejectingJson";
import { jsonPost, skillUrl, type SessionSkillRow, type SkillRow } from "./skills";

// -- composer controls ---------------------------------------------------------

/** How hard the model is asked to think. The names come from the server, not from here:
 *  a scale baked into the client goes stale the moment the server's changes. */
export interface EffortState {
  effort: string;
  levels: string[];
}

export async function setEffort(effort: string): Promise<{ ok: boolean; effort?: string; thinking_budget?: number | null; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/settings/effort`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ effort }),
  });
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
  const res = await fetch(`${httpBase()}/v1/voice/device`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(patch),
  });
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

export async function listSkills(workspace?: string): Promise<SkillRow[]> {
  const qs = workspace ? `?workspace=${encodeURIComponent(workspace)}` : "";
  const res = await fetch(skillUrl(qs));
  return listOf(res, "skills");
}

/** The Skills page's switch: on or off everywhere. The server changes nothing else about a
 *  skill here — creating, editing, uploading and revealing one were never served. */
export async function updateSkill(
  name: string,
  patch: { enabled: boolean },
): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(skillUrl(`/${encodeURIComponent(name)}`), jsonPost(patch, "PATCH"));
  return res.json();
}

export async function deleteSkill(
  name: string,
  workspace?: string,
): Promise<{ ok: boolean; error?: string }> {
  const qs = workspace ? `?workspace=${encodeURIComponent(workspace)}` : "";
  const res = await fetch(skillUrl(`/${encodeURIComponent(name)}${qs}`), { method: "DELETE" });
  return res.json();
}

export async function sessionSkills(
  sessionId: string,
  workspace?: string,
  /** Which assistant this conversation is for. A session reaches the server only on
   *  its first turn, so without this the menu for a brand-new conversation could not
   *  be narrowed to the assistant's own procedures and offered all of them. */
  agent?: string,
): Promise<SessionSkillRow[]> {
  const p = new URLSearchParams();
  if (workspace) p.set("workspace", workspace);
  if (agent) p.set("agent", agent);
  const qs = p.toString() ? `?${p}` : "";
  const res = await fetch(
    `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/skills${qs}`,
  );
  return listOf(res, "skills");
}

export async function setSessionSkill(
  sessionId: string,
  skill: string,
  enabled: boolean,
  opts: { clear?: boolean; workspace?: string } = {},
): Promise<{ skills?: SessionSkillRow[]; ok?: boolean; error?: string }> {
  const res = await fetch(
    `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/skills`,
    jsonPost({
      skill,
      enabled,
      ...(opts.clear ? { clear: true } : {}),
      ...(opts.workspace ? { workspace: opts.workspace } : {}),
    }),
  );
  return res.json();
}
