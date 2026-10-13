import { useEffect, useState } from "react";
import { clearEgress, getEgress, setLocalOnly, type EgressHost, type EgressState } from "../../api";
import { CARD } from "../../ui";
import { exactTime, timeAgo } from "../../when";
import { Toggle } from "../Toggle";
import { RequestRow } from "./RequestRow";

// What left this machine.
//
// The page leads with hosts rather than requests, because the question people
// come here with is "who has heard from me", and a reverse-chronological list of
// three hundred requests answers that only if you read all of them. The requests
// are underneath, for when the rollup raises a question.
//
// Local-only lives here too. It's the control this page is *about*, and putting
// the switch next to the evidence of what it prevents is more use than filing it
// in Settings with the theme picker.
//
// The hosts were a column of identical cards: 3.3 MB of prompts sent to a model provider
// sat in the same box, at the same weight, as "0 B sent" to an encyclopedia. The page's
// whole job is to make that difference visible, so hosts are a table now, with the volume
// drawn as a bar and the hosts that receive your prompts marked as such.

export const bytes = (n: number) => {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
};

/** A host that answers model calls receives what you type, which is a different kind of
 *  hearing-from-you than a page fetch. */
export const receivesPrompts = (h: EgressHost) => h.purposes.includes("model");

/** Past this many hosts the table folds; the busiest are the ones worth reading first. */
export const HOSTS_SHOWN = 12;

export function NetworkSection() {
  const [state, setState] = useState<EgressState | null>(null);
  const [busy, setBusy] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const [allHosts, setAllHosts] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const refresh = () => getEgress().then(setState).catch(() => setState(null));
  useEffect(() => {
    refresh();
  }, []);

  const toggleLocalOnly = async () => {
    if (!state) return;
    setBusy(true);
    const next = await setLocalOnly(!state.local_only);
    setBusy(false);
    if (next !== null) setState({ ...state, local_only: next });
  };

  const forget = async () => {
    setBusy(true);
    await clearEgress();
    setBusy(false);
    setConfirmClear(false);
    refresh();
  };

  if (!state) return <div className={CARD + " p-4 text-md text-muted"}>Loading…</div>;

  const total = state.hosts.reduce((n, h) => n + h.requests, 0);
  const blocked = state.hosts.reduce((n, h) => n + h.blocked, 0);
  const sent = state.hosts.reduce((n, h) => n + h.sent, 0);
  const maxSent = Math.max(1, ...state.hosts.map((h) => h.sent));
  const hosts = allHosts ? state.hosts : state.hosts.slice(0, HOSTS_SHOWN);
  const shown = showAll ? state.requests : state.requests.slice(0, 40);

  return (
    <div data-testid="network-section">
      <div className={"net-switch " + (state.local_only ? "is-on" : "")}>
        <div className="min-w-0 flex-1">
          <div className="net-switch-title" id="local-only-label">
            Local-only mode is {state.local_only ? "on" : "off"}
          </div>
          <p className="net-switch-body">
            {state.local_only
              ? "On — nothing may leave this machine. Cloud models, connector syncs, web search and messaging are all refused, and each refusal is recorded below. Ollama and anything else on this Mac keep working."
              : "Cloud models, connectors and the web can be reached, and every request is recorded below. Turn it on to refuse them all at the point they're made, rather than trusting each feature to check."}
          </p>
        </div>
        <Toggle
          checked={state.local_only}
          onChange={toggleLocalOnly}
          disabled={busy}
          label="Local-only mode"
          testId="local-only-toggle"
        />
      </div>

      {state.hosts.length === 0 ? (
        <div className={CARD + " p-4 text-md text-muted"}>
          Nothing has left this machine
          {state.local_only ? " — and with local-only on, nothing will." : " yet."} Requests to
          Ollama and other addresses on this Mac aren't listed, because they never went anywhere.
        </div>
      ) : (
        <>
          <div className="net-summary">
            <span>
              {total} request{total === 1 ? "" : "s"} to {state.hosts.length} host
              {state.hosts.length === 1 ? "" : "s"} · {bytes(sent)} sent
            </span>
            {blocked > 0 && <span className="net-refused">{blocked} refused by local-only mode</span>}
          </div>
          <div className="net-table-wrap">
            <table className="net-table">
              <thead>
                <tr>
                  <th scope="col">Host</th>
                  <th scope="col">What for</th>
                  <th scope="col" className="num">Requests</th>
                  <th scope="col">Sent</th>
                  <th scope="col" className="num">Last</th>
                </tr>
              </thead>
              <tbody>
                {hosts.map((h) => (
                  <tr key={h.host} data-testid="egress-host">
                    <td>
                      <span className="net-host">{h.host}</span>
                      {receivesPrompts(h) && (
                        <span className="net-badge" title="This host answers model calls, so it receives what you write to the assistant.">
                          receives your prompts
                        </span>
                      )}
                    </td>
                    <td className="net-purpose">{h.purposes.join(", ")}</td>
                    <td className="num">
                      {h.requests}
                      {h.blocked > 0 && <span className="net-refused"> · {h.blocked} refused</span>}
                    </td>
                    <td>
                      <span className="net-bar" aria-hidden>
                        <span style={{ width: `${Math.max(2, Math.round((h.sent / maxSent) * 100))}%` }} />
                      </span>
                      <span className="net-bytes">{bytes(h.sent)}</span>
                    </td>
                    <td className="num" title={exactTime(h.last)}>
                      {timeAgo(h.last)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {state.hosts.length > HOSTS_SHOWN && (
            <button className="btn sm ghost mt-2" onClick={() => setAllHosts((v) => !v)}>
              {allHosts ? "Show fewer hosts" : `Show all ${state.hosts.length} hosts`}
            </button>
          )}
        </>
      )}

      {state.requests.length > 0 && (
        <div className="mt-6">
          <div className="flex items-center gap-3 mb-2">
            <h3 className="text-sm font-medium text-ink">Requests</h3>
            {confirmClear ? (
              <span className="ml-auto flex items-center gap-2 text-sm text-muted" role="group" aria-label="Clear the log">
                Clear {state.requests.length} recorded request{state.requests.length === 1 ? "" : "s"}? This can't be undone.
                <button className="btn sm danger" disabled={busy} onClick={forget} data-testid="egress-clear-confirm">
                  Clear log
                </button>
                <button className="btn sm ghost" onClick={() => setConfirmClear(false)}>
                  Cancel
                </button>
              </span>
            ) : (
              <button className="btn sm ghost ml-auto" disabled={busy} onClick={() => setConfirmClear(true)}>
                Clear log…
              </button>
            )}
          </div>
          <div className="net-requests">
            {shown.map((r, i) => (
              <RequestRow r={r} key={`${r.at}-${i}`} />
            ))}
          </div>
          {state.requests.length > shown.length && (
            <button className="btn sm ghost mt-2" onClick={() => setShowAll(true)}>
              Show all {state.requests.length}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
