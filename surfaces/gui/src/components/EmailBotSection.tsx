import { useEffect, useState } from "react";
import { getEmailBot, setEmailBot, type EmailBotSetup } from "../api";
import { CARD } from "../ui";

// The email bot's settings.
//
// New email from the senders you list gets a reply drafted by the Email Assistant, and every
// draft waits in the Inbox: nothing is sent until you approve it. Off by default and gated on a
// list, like Telegram, because anyone can write to you. When it's switched on but can't run,
// the reason is shown rather than a switch that silently does nothing.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";

export function EmailBotSection() {
  const [bot, setBot] = useState<EmailBotSetup | null>(null);
  const [allowed, setAllowed] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = () =>
    getEmailBot().then((b) => {
      setBot(b);
      if (b) setAllowed(b.allowed);
    });
  useEffect(() => {
    load();
  }, []);

  const save = async (extra: Partial<{ email_inbound: boolean }> = {}) => {
    setBusy(true);
    setNotice(null);
    const res = await setEmailBot({ email_allowed: allowed.trim(), ...extra });
    setBusy(false);
    if (res.ok === false) {
      setNotice(res.error ?? "Couldn't save that.");
      return;
    }
    await load();
    setNotice(res.live ? "Saved. New email from these senders will get a drafted reply." : "Saved.");
  };

  if (!bot) return null;

  return (
    <section className={`${CARD} p-4 mt-6`} data-testid="email-bot">
      <div className="text-md font-medium">Answer email</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        New email from the people you list gets a reply drafted by the Email Assistant. Every
        draft waits in your Inbox, and nothing is sent until you approve it. Only email that
        arrives after you switch this on is read.
      </p>

      {!bot.connected ? (
        <div className="text-sm text-muted">
          Connect your email first: sidebar ▸ More ▸ Connectors ▸ Email (IMAP).
        </div>
      ) : (
        <>
          <label className="text-sm font-medium text-ink" htmlFor="email-bot-allowed">
            Senders to answer
          </label>
          <textarea
            id="email-bot-allowed"
            className={`${INPUT} mt-1 mb-1 min-h-[64px]`}
            placeholder="ada@example.com, @yourcompany.com"
            value={allowed}
            onChange={(e) => setAllowed(e.target.value)}
          />
          <div className="text-xs text-muted mb-3">
            Addresses, or @domain for everyone at one. Everyone else, mailing lists, automatic
            replies and no-reply senders are left alone.
          </div>

          <label className="flex items-start gap-2 text-sm mb-1">
            <input
              type="checkbox"
              role="switch"
              className="mt-0.5"
              aria-label="Draft replies to new email"
              checked={bot.inbound}
              disabled={busy}
              onChange={(e) => save({ email_inbound: e.target.checked })}
            />
            <span>
              <span className="font-medium">Draft replies to new email</span>
              <span className="block text-xs text-muted leading-prose">
                For {bot.address}. The Email Assistant can read, not act — an email is text
                anyone can write, so it drafts and you decide.
              </span>
            </span>
          </label>

          {bot.inbound && bot.problem ? (
            <div className="text-xs text-warn mb-2" role="alert">
              Paused: {bot.problem}.
            </div>
          ) : bot.inbound && bot.allowed_count === 0 ? (
            <div className="text-xs text-warn mb-2" role="alert">
              On, but no one is listed, so nothing is being answered yet.
            </div>
          ) : bot.live ? (
            <div className="text-xs text-muted mb-2">
              {bot.drafted_today} of {bot.daily_cap} drafts today.
            </div>
          ) : null}

          <div className="flex items-center gap-2 mt-3">
            <button className={BTN} disabled={busy} onClick={() => save()}>
              {busy ? "Saving…" : "Save"}
            </button>
            {notice ? (
              <span className="text-xs text-muted" role="status">
                {notice}
              </span>
            ) : null}
          </div>
        </>
      )}
    </section>
  );
}
