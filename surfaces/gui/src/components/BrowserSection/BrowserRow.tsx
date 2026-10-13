import type { ComputerStatus } from "../../api";
import { openInComputer, setPersonaBrowser, stopComputer } from "../../api";
import { Icon, ICON } from "../Icon";
import { BTN_ACCENT, BTN_BORDERED, INPUT, stateLine } from "./BTN_ACCENT";

type Run = (key: string, work: () => Promise<{ ok: boolean; error?: string } | null>, failure: string) => Promise<boolean>;

/** One assistant that has a browser: its state, and watch / sign in / stop / reset / revoke. */
export function BrowserRow({
  c,
  enabled,
  busy,
  signIn,
  setSignIn,
  setWatching,
  setConfirmReset,
  run,
}: {
  c: ComputerStatus;
  enabled: boolean;
  busy: string;
  signIn: { persona: string; url: string } | null;
  setSignIn: (s: { persona: string; url: string } | null) => void;
  setWatching: (w: { persona: string; name?: string } | null) => void;
  setConfirmReset: (c: ComputerStatus | null) => void;
  run: Run;
}) {
  const line = stateLine(c);
  const who = c.name || c.persona;
  const opening = signIn?.persona === c.persona;
  return (
    <div className="p-3" data-testid={`browser-row-${c.persona}`}>
      <div className="flex items-center gap-2.5">
        <Icon name="globe" size={ICON.lg} className="text-faint shrink-0" />
        <div className="min-w-0 flex-1">
          <div className="text-md text-ink truncate">{who}</div>
          <div
            className={
              "text-sm truncate " +
              (line.tone === "accent" ? "text-accent" : line.tone === "muted" ? "text-muted" : "text-ink")
            }
          >
            {line.text}
          </div>
        </div>
      </div>
      {/* Its own line, wrapping: five actions beside a name leave no room for the
          name, and the state line is the part worth reading. */}
      <div className="flex flex-wrap items-center gap-2 mt-2 pl-8">
        <button
          className={BTN_BORDERED}
          disabled={!enabled}
          onClick={() => setWatching({ persona: c.persona, name: c.name })}
          data-testid={`browser-watch-${c.persona}`}
        >
          Watch
        </button>
        <button
          className={BTN_BORDERED}
          disabled={!enabled}
          onClick={() => setSignIn(opening ? null : { persona: c.persona, url: "" })}
          data-testid={`browser-signin-${c.persona}`}
        >
          Sign in to a site
        </button>
        {c.running && (
          <button
            className={BTN_BORDERED}
            disabled={busy === `stop-${c.persona}`}
            onClick={() => void run(`stop-${c.persona}`, () => stopComputer(c.persona), `Couldn't stop ${who}'s browser.`)}
            data-testid={`browser-stop-${c.persona}`}
          >
            Stop
          </button>
        )}
        {c.has_profile && (
          <button
            className={BTN_BORDERED}
            onClick={() => setConfirmReset(c)}
            data-testid={`browser-reset-${c.persona}`}
          >
            Sign out of everything
          </button>
        )}
        <button
          className={BTN_BORDERED}
          disabled={busy === `grant-${c.persona}`}
          title="Its sign-ins are kept, in case you give it back"
          onClick={() =>
            void run(`grant-${c.persona}`, () => setPersonaBrowser(c.persona, false), `Couldn't take ${who}'s browser away.`)
          }
          data-testid={`browser-revoke-${c.persona}`}
        >
          Take it away
        </button>
      </div>
      {opening && (
        <form
          className="flex items-center gap-2 mt-2.5 pl-8"
          onSubmit={async (e) => {
            e.preventDefault();
            if (!signIn?.url.trim()) return;
            const ok = await run(
              `open-${c.persona}`,
              () => openInComputer(c.persona, signIn.url.trim()),
              `Couldn't open that in ${who}'s browser.`,
            );
            if (ok) {
              setSignIn(null);
              setWatching({ persona: c.persona, name: c.name });
            }
          }}
        >
          <input
            autoFocus
            className={INPUT}
            placeholder="The site to sign in to, e.g. mail.google.com"
            value={signIn?.url ?? ""}
            onChange={(e) => setSignIn({ persona: c.persona, url: e.target.value })}
            data-testid={`browser-signin-url-${c.persona}`}
          />
          <button className={BTN_ACCENT} type="submit" disabled={busy === `open-${c.persona}` || !signIn?.url.trim()}>
            Open it for me
          </button>
        </form>
      )}
    </div>
  );
}
