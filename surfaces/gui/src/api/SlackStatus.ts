import type { WsEvent } from "../types";
import type { GmailFilters } from "./connectors";
import { fetch, httpBase, jsonBody } from "./neverRejectingJson";

// Accounts, installations and health for the connectors that have more than one of each.

const connectorUrl = (connector: string, rest: string) => `${httpBase()}/v1/connectors/${encodeURIComponent(connector)}${rest}`;
const post = async (url: string) => (await fetch(url, { method: "POST" })).json();

/** Remove ONE account of a multi-account connector (notion, attio, posthog, …); the default
 *  moves to the next account. */
export function disconnectAccount(connector: string, accountId: string): Promise<{ ok: boolean; error?: string; remaining_accounts?: number }> {
  return post(connectorUrl(connector, `/accounts/${encodeURIComponent(accountId)}/disconnect`));
}

export function setDefaultAccount(connector: string, accountId: string): Promise<{ ok: boolean; error?: string }> {
  return post(connectorUrl(connector, `/accounts/${encodeURIComponent(accountId)}/default`));
}

/** Replace the "Never show agents" lists of senders and labels; leave one out to keep it. */
export async function setGmailFilters(filters: { senders?: string[]; labels?: string[] }): Promise<{ ok: boolean; filters?: GmailFilters; error?: string }> {
  return (await fetch(connectorUrl("gmail", "/filters"), jsonBody("PATCH", filters))).json();
}

/** GitHub's relay health, in the same three layers as Slack's: the shared relay socket, the
 *  cloud sign-in, and each installation's token, plus counts of missed events. */
export interface GithubStatus {
  ok: boolean;
  mode: string;
  signed_in: boolean;
  relay: { state: string; reconnects: number; last_event_at: number | null; last_error: string };
  installs: Record<string, { token_ok: boolean }>;
  missed: Record<string, number>;
}

export async function getGithubStatus(): Promise<GithubStatus> {
  return (await fetch(connectorUrl("github", "/status"))).json();
}

/** Stop relaying ONE GitHub App installation to this computer. */
export function disconnectGithubInstallation(installationId: string): Promise<{ ok: boolean; error?: string; remaining_installs?: number }> {
  return post(connectorUrl("github", `/installations/${encodeURIComponent(installationId)}/disconnect`));
}

/** Remove ONE HubSpot portal; the default moves to the next one. */
export function disconnectHubSpotPortal(hubId: string): Promise<{ ok: boolean; error?: string; remaining_portals?: number }> {
  return post(connectorUrl("hubspot", `/portals/${encodeURIComponent(hubId)}/disconnect`));
}

export function setHubSpotDefaultPortal(hubId: string): Promise<{ ok: boolean; error?: string }> {
  return post(connectorUrl("hubspot", `/portals/${encodeURIComponent(hubId)}/default`));
}

/** Replace the list of properties kept out of what agents read. */
export async function setHubSpotHiddenFields(fields: string[]): Promise<{ ok: boolean; hidden_fields?: string[]; error?: string }> {
  return (await fetch(connectorUrl("hubspot", "/hidden-fields"), jsonBody("PATCH", { hidden_fields: fields }))).json();
}

/** Slack's health in three plain layers: the relay socket, the cloud sign-in, each team's token. */
export interface SlackStatus {
  /** "relay", or "" when set up by hand or off. */
  mode: string;
  signed_in: boolean;
  relay: {
    state: "live" | "reconnecting" | "offline";
    reconnects: number;
    last_event_at: number | null;
    last_error: string;
  };
  teams: Record<string, { token_ok: boolean }>;
}

export async function getSlackStatus(): Promise<SlackStatus> {
  return (await fetch(connectorUrl("slack", "/status"))).json();
}

export type Handlers = {
  onEvent: (event: WsEvent) => void;
  onOpen?: () => void;
  onClose?: () => void;
};
