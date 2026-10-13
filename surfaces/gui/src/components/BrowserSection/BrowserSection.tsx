import { useCallback, useEffect, useState } from "react";
import { getComputers, setComputerSettings, setPersonaBrowser, type ComputersState, type ComputerStatus } from "../../api";
import { PanelHead } from "../IntegrationsView";
import { LiveView } from "../LiveView";
import { notifyError } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL, SEC_H } from "../../ui";
import { BTN_BORDERED } from "./BTN_ACCENT";
import { BrowserRow } from "./BrowserRow";
import { ResetConfirm } from "./ResetConfirm";

export function BrowserSection() {
  const [state, setState] = useState<ComputersState | null>(null);
  const [failed, setFailed] = useState(false);
  const [watching, setWatching] = useState<{ persona: string; name?: string } | null>(null);
  const [signIn, setSignIn] = useState<{ persona: string; url: string } | null>(null);
  const [confirmReset, setConfirmReset] = useState<ComputerStatus | null>(null);
  const [busy, setBusy] = useState("");

  const load = useCallback(async () => {
    const s = await getComputers().catch(() => null);
    if (s) {
      setState(s);
      setFailed(false);
    } else setFailed(true);
  }, []);

  useEffect(() => {
    void load();
    // Status moves on its own — a page opens, a request for help arrives — so the list
    // keeps up while someone is looking at it.
    const t = window.setInterval(() => void load(), 3000);
    return () => clearInterval(t);
  }, [load]);

  const save = async (patch: { browser_enabled?: boolean; browser_allow_loopback?: boolean; browser_headed?: boolean }) => {
    const res = await setComputerSettings(patch).catch(() => null);
    if (!res?.ok) {
      notifyError("Couldn't save that setting.", res?.error);
      return;
    }
    await load();
  };

  const run = async (key: string, work: () => Promise<{ ok: boolean; error?: string } | null>, failure: string) => {
    setBusy(key);
    const res = await work().catch(() => null);
    setBusy("");
    if (!res?.ok) notifyError(failure, res?.error);
    await load();
    return !!res?.ok;
  };

  if (!state)
    return (
      <section>
        <PanelHead title="Browser" sub="A real browser for each assistant to work websites in." />
        <p className="text-sm text-muted">{failed ? "Couldn't load the browser settings." : "Loading…"}</p>
      </section>
    );

  const granted = state.computers.filter((c) => c.granted);
  const others = state.computers.filter((c) => !c.granted);

  return (
    <section data-testid="browser-section">
      <PanelHead
        title="Browser"
        sub="A real browser for each assistant to work websites in — signed in where you sign it in, watchable, and handed to you at a sign-in."
      />

      <div className={CARD + " p-4 mb-4"}>
        <label className="flex items-start gap-3">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={state.enabled}
            onChange={(e) => void save({ browser_enabled: e.target.checked })}
            data-testid="browser-enabled"
          />
          <span>
            <span className={"block " + FIELD_LABEL}>Let assistants use their own browser</span>
            <span className={"block " + FIELD_HELP}>
              Each assistant you give a browser gets its own Chrome, kept apart from yours and from each other&rsquo;s. It
              can read pages that need JavaScript or a sign-in, and click and type in them — asking first, as your
              approval settings say. It can&rsquo;t reach this Mac&rsquo;s network or private addresses, apart from the
              staging site you name for Bug Reproduction under General. Switching this
              off closes every one of them; their sign-ins are kept.
            </span>
          </span>
        </label>
        {!state.chrome_found && (
          <p className="text-sm text-danger mt-3" data-testid="browser-no-chrome">
            No Chrome-family browser was found on this Mac. Install Google Chrome, Chromium, Brave or Microsoft Edge.
          </p>
        )}
        {state.enabled && state.local_only && (
          <p className={FIELD_HELP}>
            Local-only mode is on, so the browsers can only open this Mac&rsquo;s own dev servers, and only if that is
            allowed below.
          </p>
        )}
      </div>

      {state.enabled && (
        <div className={CARD + " p-4 mb-4"}>
          <label className="flex items-start gap-3 py-1">
            <input
              type="checkbox"
              className="mt-0.5"
              checked={state.allow_loopback}
              onChange={(e) => void save({ browser_allow_loopback: e.target.checked })}
              data-testid="browser-loopback"
            />
            <span>
              <span className={"block " + FIELD_LABEL}>Allow this Mac&rsquo;s own dev servers</span>
              <span className={"block " + FIELD_HELP}>
                Lets the browsers open localhost addresses, so an assistant can check the app you are building. Anything
                else listening on this Mac becomes reachable too.
              </span>
            </span>
          </label>
          <label className="flex items-start gap-3 py-1 mt-2">
            <input
              type="checkbox"
              className="mt-0.5"
              checked={state.headed}
              onChange={(e) => void save({ browser_headed: e.target.checked })}
              data-testid="browser-headed"
            />
            <span>
              <span className={"block " + FIELD_LABEL}>Show each browser in its own window</span>
              <span className={"block " + FIELD_HELP}>
                For sites that refuse a browser with no window. Open browsers restart to show the change.
              </span>
            </span>
          </label>
        </div>
      )}

      <div className={SEC_H + " mb-2 mt-6"}>Assistants with a browser</div>
      {granted.length === 0 ? (
        <p className="text-sm text-muted mb-4">
          None yet. Give one to an assistant below.
        </p>
      ) : (
        <div className={CARD + " divide-y divide-line mb-4"}>
          {granted.map((c) => (
            <BrowserRow
              key={c.persona}
              c={c}
              enabled={state.enabled}
              busy={busy}
              signIn={signIn}
              setSignIn={setSignIn}
              setWatching={setWatching}
              setConfirmReset={setConfirmReset}
              run={run}
            />
          ))}
        </div>
      )}

      {others.length > 0 && (
        <>
          <div className={SEC_H + " mb-2 mt-6"}>Without one</div>
          <div className={CARD + " divide-y divide-line"}>
            {others.map((c) => (
              <div className="flex items-center gap-2.5 p-3" key={c.persona}>
                <span className="min-w-0 flex-1 text-md text-ink truncate">{c.name || c.persona}</span>
                <button
                  className={BTN_BORDERED}
                  disabled={busy === `grant-${c.persona}`}
                  onClick={() =>
                    void run(`grant-${c.persona}`, () => setPersonaBrowser(c.persona, true), `Couldn't give ${c.name || c.persona} a browser.`)
                  }
                  data-testid={`browser-grant-${c.persona}`}
                >
                  Give it a browser
                </button>
              </div>
            ))}
          </div>
        </>
      )}

      {confirmReset && <ResetConfirm c={confirmReset} busy={busy} run={run} onClose={() => setConfirmReset(null)} />}

      {watching && <LiveView persona={watching.persona} name={watching.name} onClose={() => setWatching(null)} />}
    </section>
  );
}
