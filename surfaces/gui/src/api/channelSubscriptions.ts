import { fetch, httpBase, listOf } from "./neverRejectingJson";

// Which sessions listen to which channels. Read-only here.

export interface Subscription {
  channel: string;
  /** The channel's display name ("team-test"); the address stays the id. */
  channel_name?: string | null;
  session_id: string;
  session_title: string;
  agent: string;
  routing_target: string | null;
  /** True when a session listens on the same channel the Inbox sends to. */
  collision: boolean;
}

export interface RecentChannel {
  channel: string;
  /** Display name such as "team-test"; falls back to the address. */
  name?: string | null;
  last_text: string | null;
  last_from: string | null;
}

export async function getSubscriptions(): Promise<Subscription[]> {
  return listOf(await fetch(`${httpBase()}/v1/subscriptions`), "subscriptions");
}
