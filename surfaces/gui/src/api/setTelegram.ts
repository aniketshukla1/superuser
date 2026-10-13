import { fetch, httpBase } from "./neverRejectingJson";

/** Save the setup. `token` is written to the keychain and never read back. */
export async function setTelegram(body: {
  token?: string;
  telegram_chat_id?: string;
  telegram_allowed?: string;
  telegram_inbound?: boolean;
}): Promise<{ ok: boolean; error?: string; connected?: boolean; inbound_live?: boolean }> {
  const res = await fetch(`${httpBase()}/v1/telegram`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return res.json();
}
