import { useEffect, useState } from "react";
import { getTelegram, setTelegram, type TelegramSetup } from "../api";
import { CARD } from "../ui";
import { Icon, ICON } from "./Icon";
import { Toggle } from "./Toggle";

/** Said once, where the decision is made. */
const TELEGRAM_INBOUND_WARNING =
  "Turning this on lets the chats listed above run commands, read and write files, and use your connected apps through this Mac. Only the ids you have listed can do it. Turn it on?";

// Reaching your assistant from your phone, and answering from either end.
//
// Outbound has always worked — the app can push you a reminder. Inbound is the
// half that makes it a conversation, and it's the half with teeth: anyone can
// message a bot once they know its handle, and this one answers with an agent that
// has file and shell access. So inbound is off by default and is gated on an
// allowlist; an empty allowlist serves nobody rather than everybody.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN =
  "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";

export function TelegramSection() {
  const [tg, setTg] = useState<TelegramSetup | null>(null);
  const [token, setToken] = useState("");
  const [chatId, setChatId] = useState("");
  const [allowed, setAllowed] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = () =>
    getTelegram().then((t) => {
      setTg(t);
      if (t) {
        setChatId(t.chat_id);
        setAllowed(t.allowed);
      }
    });
  useEffect(() => {
    load();
  }, []);

  const save = async (extra: Partial<{ telegram_inbound: boolean }> = {}) => {
    setBusy(true);
    setNotice(null);
    const res = await setTelegram({
      // Only send the token when one was typed — an empty box means "leave it
      // alone", not "disconnect".
      ...(token.trim() ? { token: token.trim() } : {}),
      telegram_chat_id: chatId.trim(),
      telegram_allowed: allowed.trim(),
      ...extra,
    });
    setBusy(false);
    setToken("");
    if (!res.ok) {
      setNotice(res.error ?? "Couldn't save that.");
      return;
    }
    await load();
    setNotice(
      res.inbound_live
        ? "Saved. Messages from the allowed chats will be answered."
        : "Saved.",
    );
  };

  if (!tg) return null;

  return (
    <section className={`${CARD} p-4 mt-6`}>
      <div className="text-md font-medium">Telegram</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        Reach your assistant from your phone. Reminders and results can go out to a chat, and
        with replies switched on you can answer from Telegram — the conversation shows up here
        too, so you never have to leave the app to keep it going.
      </p>

      <label className="text-sm font-medium text-ink">Bot token</label>
      <input
        className={`${INPUT} mt-1 mb-1`}
        type="password"
        placeholder={tg.connected ? "•••••••• (saved — type to replace)" : "123456:ABC-DEF…"}
        value={token}
        onChange={(e) => setToken(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        From @BotFather. Stored in your keychain, never shown again.
      </div>

      <label className="text-sm font-medium text-ink">Your chat id</label>
      <input
        className={`${INPUT} mt-1 mb-1`}
        placeholder="12345678"
        value={chatId}
        onChange={(e) => setChatId(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        Where notifications go, and the one chat allowed to send instructions.
      </div>

      <label className="text-sm font-medium text-ink">Also allow (optional)</label>
      <input
        className={`${INPUT} mt-1 mb-1`}
        placeholder="87654321, 11223344"
        value={allowed}
        onChange={(e) => setAllowed(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        Other chat ids that may talk to your assistant. Everyone else is ignored.
      </div>

      {/* A switch, framed as what it is: the one setting on this page that hands a remote
          chat the same reach the app has on this Mac. It was a plain checkbox in a row of
          plain checkboxes, and its consequence was in the fourth line of its own caption. */}
      <div className={"tg-inbound flex items-start gap-3 text-sm mb-1" + (tg.inbound ? " is-on" : "")}>
        <Toggle
          checked={tg.inbound}
          disabled={busy || !tg.connected}
          label="Answer messages sent to the bot"
          testId="telegram-inbound"
          onChange={(next) => {
            if (next && !window.confirm(TELEGRAM_INBOUND_WARNING)) return;
            save({ telegram_inbound: next });
          }}
        />
        <span>
          <span className="font-medium flex items-center gap-1.5">
            {tg.inbound && <Icon name="alert" size={ICON.sm} className="text-warnInk" />}
            Answer messages sent to the bot
          </span>
          <span className="block text-xs text-muted leading-prose">
            Off means outbound only — it posts, and reads nothing back. On, the chats listed
            above can ask your assistant to do things, with the same file, shell and app
            access it has here.
          </span>
        </span>
      </div>
      {tg.inbound && !tg.inbound_live ? (
        <div className="text-xs text-warn mb-2">
          Replies are on but no chat id is set, so nothing is being answered yet.
        </div>
      ) : null}

      <div className="flex items-center gap-2 mt-3">
        <button className={BTN} disabled={busy} onClick={() => save()}>
          {busy ? "Saving…" : "Save"}
        </button>
        {notice ? <span className="text-xs text-muted">{notice}</span> : null}
      </div>
    </section>
  );
}
