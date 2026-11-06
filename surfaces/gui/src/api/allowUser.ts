import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

// Automation runs, and who may reach the app through a connector.

const enc = encodeURIComponent;
const automationUrl = (id: string, rest = "") => `${httpBase()}/v1/automations/${enc(id)}${rest}`;
const connectorUrl = (name: string, rest: string) => `${httpBase()}/v1/connectors/${enc(name)}${rest}`;
const slackTeamUrl = (teamId: string, rest: string) => connectorUrl("slack", `/workspaces/${enc(teamId)}${rest}`);
const call = async (url: string, init?: RequestInit) => (await fetch(url, init)).json();
const postEmpty = (url: string) => call(url, { method: "POST" });

export function updateAutomation(id: string, changes: Record<string, any>) {
  return call(automationUrl(id), jsonBody("PATCH", changes));
}

export function deleteAutomation(id: string) {
  return call(automationUrl(id), { method: "DELETE" });
}

export interface PreparedRun {
  ok: boolean;
  error?: string;
  run_id: string;
  session_id: string;
  agent: string;
  workspace: string;
  prompt: string;
}

/** Set up a manual run to watch live: the session to open and the first prompt to send. */
export function runAutomation(id: string): Promise<PreparedRun> {
  return postEmpty(automationUrl(id, "/run"));
}

/** Mark a manual run finished once its first turn is done. */
export function finalizeAutomationRun(id: string, runId: string) {
  return postEmpty(automationUrl(id, `/runs/${enc(runId)}/finalize`));
}

export function allowUser(name: string, userId: string, teamId?: string | null, displayName?: string) {
  const body: Record<string, string> = { user_id: userId };
  if (teamId) body.team_id = teamId;
  // A pick from the directory brings its display name, so the chip reads well straight away.
  if (displayName) body.name = displayName;
  return call(connectorUrl(name, "/allow"), jsonBody("POST", body));
}

export function disallowUser(name: string, userId: string, teamId?: string | null) {
  const body: Record<string, string> = { user_id: userId };
  if (teamId) body.team_id = teamId;
  return call(connectorUrl(name, "/disallow"), jsonBody("POST", body));
}

/** A member of the workspace, for the people picker (users:read, cached on this machine). */
export interface SlackMember {
  id: string;
  name: string;
  handle: string;
  guest: boolean;
}

/** A channel of the workspace. Slack lists private channels only where the bot is a member;
 *  `is_member: false` brings up the "invite @Superuser" hint. */
export interface SlackChannelEntry {
  id: string;
  name: string;
  is_member: boolean;
  is_private: boolean;
}

/** The workspace's members for the people picker. teamId "default" means manual Socket Mode. */
export function getSlackDirectory(teamId: string, q = ""): Promise<{ ok: boolean; error?: string; members?: SlackMember[] }> {
  return call(slackTeamUrl(teamId, `/directory?q=${enc(q)}`));
}

/** The workspace's channels for the channel typeahead, which turns a name into an id. */
export function getSlackChannels(teamId: string, q = ""): Promise<{ ok: boolean; error?: string; channels?: SlackChannelEntry[] }> {
  return call(slackTeamUrl(teamId, `/channels?q=${enc(q)}`));
}

/** Settle a parked message from someone not yet allowed (§19): dismiss it, allow the sender,
 *  or allow them and deliver it. */
export function resolveUnauthorized(name: string, itemId: string, action: "dismiss" | "allow" | "allow_deliver"): Promise<{ ok: boolean; error?: string }> {
  return call(connectorUrl(name, `/unauthorized/${enc(itemId)}`), jsonBody("POST", { action }));
}

export function addSlackApprovalOwner(userId: string, displayName?: string): Promise<{ ok: boolean; error?: string }> {
  const body: Record<string, string> = { user_id: userId };
  if (displayName) body.name = displayName;
  return call(connectorUrl("slack", "/approval-owners/add"), jsonBody("POST", body));
}

export function removeSlackApprovalOwner(userId: string): Promise<{ ok: boolean; error?: string }> {
  return call(connectorUrl("slack", "/approval-owners/remove"), jsonBody("POST", { user_id: userId }));
}

/** Stop relaying one managed Slack workspace; the app stays installed on Slack's side. */
export function disconnectSlackWorkspace(teamId: string): Promise<{ ok: boolean; error?: string; remaining_workspaces?: number }> {
  return postEmpty(slackTeamUrl(teamId, "/disconnect"));
}

// Google accounts: each connector keeps one per email, and one of them is the default.
const googleAccountUrl = (connector: "gmail" | "google_calendar", email: string, action: string) =>
  connectorUrl(connector, `/accounts/${enc(email)}/${action}`);

/** Remove ONE Gmail mailbox; the default moves to the next account. */
export function disconnectGmailAccount(email: string): Promise<{ ok: boolean; error?: string; remaining_accounts?: number }> {
  return postEmpty(googleAccountUrl("gmail", email, "disconnect"));
}

export function setGmailDefaultAccount(email: string): Promise<{ ok: boolean; error?: string }> {
  return postEmpty(googleAccountUrl("gmail", email, "default"));
}

/** Remove ONE Google Calendar account; the default moves to the next one. */
export function disconnectGcalAccount(email: string): Promise<{ ok: boolean; error?: string; remaining_accounts?: number }> {
  return postEmpty(googleAccountUrl("google_calendar", email, "disconnect"));
}

export function setGcalDefaultAccount(email: string): Promise<{ ok: boolean; error?: string }> {
  return postEmpty(googleAccountUrl("google_calendar", email, "default"));
}
