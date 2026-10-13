// Where an assistant's turns run — here, or on a remote AG-UI agent.
//
// Its errors stay in this section. A refused address is something to fix in the field,
// and the page-level error screen would take the whole assistant away to say it.

import { useState } from "react";
import { testRemoteAgent, updatePersona } from "../api";
import { SEC_H } from "../ui";

const INPUT = "w-full px-3 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

type Status = { tone: "ok" | "bad" | "busy"; text: string } | null;

export function RemoteAgentSection({
  personaId,
  name,
  remote,
  tokenSet,
  onSaved,
}: {
  personaId: string;
  name: string;
  remote: string | null | undefined;
  tokenSet: boolean;
  onSaved: (remote: string | null, tokenSet: boolean) => void;
}) {
  const [open, setOpen] = useState(!!remote);
  const [endpoint, setEndpoint] = useState(remote ?? "");
  const [token, setToken] = useState("");
  const [status, setStatus] = useState<Status>(null);
  const [busy, setBusy] = useState(false);

  const test = async () => {
    setBusy(true);
    setStatus({ tone: "busy", text: "Asking it to say hello…" });
    try {
      const r = await testRemoteAgent(personaId, endpoint.trim(), token.trim());
      setStatus(r.ok ? { tone: "ok", text: r.message ?? "It answered." } : { tone: "bad", text: r.error ?? "It didn't answer." });
    } catch {
      setStatus({ tone: "bad", text: "Couldn't reach this app's server to run the test." });
    } finally {
      setBusy(false);
    }
  };

  const save = async (address: string) => {
    setBusy(true);
    try {
      const body: { remote: string; remote_token?: string } = { remote: address };
      if (address && token.trim()) body.remote_token = token.trim();
      const r = await updatePersona(personaId, body);
      if (!r.ok) {
        setStatus({ tone: "bad", text: r.error ?? "Not saved." });
        return;
      }
      setToken("");
      if (!address) {
        setEndpoint("");
        setOpen(false);
        setStatus({ tone: "ok", text: `${name} runs here again.` });
        onSaved(null, false);
      } else {
        setStatus({ tone: "ok", text: "Saved. Its next turn goes to that agent." });
        onSaved(address, tokenSet || !!body.remote_token);
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <section data-testid="persona-remote">
      <div className={`${SEC_H} mb-2`}>Where it runs</div>
      {!open ? (
        <div className="flex items-center gap-3">
          <p className="text-sm text-muted">Here, on this machine, with the tools and rules on this page.</p>
          <button className={BTN_BORDERED} onClick={() => setOpen(true)} data-testid="remote-open">
            Use a remote agent…
          </button>
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-muted leading-prose">
            {remote ? `${name}'s turns go to a remote agent. ` : `Send ${name}'s turns to an agent that speaks AG-UI. `}
            That agent's tools run on its own machine, under its own rules: the approvals, boundaries and
            browser on this page don't reach it, and its answers count as outside content — like a web page.
          </p>
          <label className="text-sm font-medium text-ink" htmlFor="remote-endpoint">
            Agent address
          </label>
          <input
            id="remote-endpoint"
            className={INPUT}
            placeholder="https://agent.example.com/run"
            value={endpoint}
            onChange={(e) => setEndpoint(e.target.value)}
            data-testid="remote-endpoint"
          />
          <label className="text-sm font-medium text-ink" htmlFor="remote-token">
            Token (optional)
          </label>
          <input
            id="remote-token"
            className={INPUT}
            type="password"
            placeholder={tokenSet ? "•••••••• (saved — type to replace)" : "Sent as a bearer token"}
            value={token}
            onChange={(e) => setToken(e.target.value)}
            data-testid="remote-token"
          />
          <div className="flex flex-wrap items-center gap-2 mt-1">
            <button className={BTN_BORDERED} disabled={busy || !endpoint.trim()} onClick={() => void test()} data-testid="remote-test">
              Test connection
            </button>
            <button
              className={BTN_BORDERED}
              disabled={busy || !endpoint.trim()}
              onClick={() => void save(endpoint.trim())}
              data-testid="remote-save"
            >
              Save
            </button>
            {remote ? (
              <button className={BTN_BORDERED} disabled={busy} onClick={() => void save("")} data-testid="remote-clear">
                Run here instead
              </button>
            ) : (
              <button
                className={BTN_BORDERED}
                disabled={busy}
                onClick={() => {
                  setOpen(false);
                  setStatus(null);
                }}
                data-testid="remote-cancel"
              >
                Cancel
              </button>
            )}
          </div>
          {status && (
            <p
              role={status.tone === "bad" ? "alert" : "status"}
              className={`text-sm ${status.tone === "bad" ? "text-danger" : status.tone === "ok" ? "text-ink" : "text-muted"}`}
              data-testid="remote-status"
            >
              {status.text}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
