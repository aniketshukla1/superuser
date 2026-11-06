import { fetch, httpBase, jsonBody, jsonOf } from "./neverRejectingJson";

// Which session answers direct messages to the bot.
const DM_ROUTE = () => `${httpBase()}/v1/messaging/dm-route`;

export async function getDmRoute(): Promise<string | null> {
  const reply = await jsonOf(await fetch(DM_ROUTE()));
  return reply.dm_session ?? null;
}

export async function setDmRoute(sessionId: string): Promise<{ ok: boolean; dm_session: string | null }> {
  const res = await fetch(DM_ROUTE(), jsonBody("POST", { session_id: sessionId }));
  return res.json();
}
