import type { Connector } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { AVATAR } from "../Icon";
import { Spinner, type QuickTemplate } from "./TEMPLATES";

type Flow = { name: string; phase: "opening" | "waiting" } | null;

/** Each connection the template needs: connected, connecting (with the browser handoff
 * strip), or a Connect button. */
export function ConnectionRows({
  conns,
  connState,
  connFlow,
  setConnFlow,
  startConnect,
}: {
  conns: QuickTemplate["conns"];
  connState: (name: string) => Connector | undefined;
  connFlow: Flow;
  setConnFlow: (f: Flow) => void;
  startConnect: (name: string) => void;
}) {
  return (
    <>
      {conns.map(({ name, why }) => {
        const c = connState(name);
        const flow = connFlow?.name === name ? connFlow : null;
        return (
          <div key={name} className="border-b border-line last:border-b-0">
            <div className="flex items-center gap-3 py-2.5">
              {c && <ConnectorBadge connector={c} size={AVATAR.md} title={c.title} />}
              <span className="min-w-0 flex-1">
                <span className="block text-md font-medium">{c?.title || name}</span>
                <span className="block text-xs text-faint">{why}</span>
              </span>
              {c?.connected ? (
                <span className="text-sm text-ok">✓ Connected</span>
              ) : flow ? (
                <span className="inline-flex items-center gap-2 text-sm text-muted">
                  <Spinner />
                  {flow.phase === "opening"
                    ? "Opening browser…"
                    : `Waiting for ${c?.title || name}…`}
                </span>
              ) : (
                <button
                  className="px-3.5 py-1 rounded-full border border-line text-sm hover:bg-paper"
                  onClick={() => startConnect(name)}
                  data-testid={`ob-connect-${name}`}
                >
                  Connect
                </button>
              )}
            </div>
            {/* §30 handoff strip: the flow finishes out-of-band in the browser — say so,
                and let Cancel clear the LOCAL state (the browser tab is the user's). */}
            {flow?.phase === "waiting" && (
              <div
                className="flex items-start gap-2 bg-accentSoft/50 rounded-lg px-3 py-2 mb-2.5 text-sm text-muted"
                data-testid="ob-connect-wait"
              >
                <span>↗</span>
                <span className="flex-1 min-w-0">
                  <b className="text-ink font-medium">
                    Finish connecting {c?.title || name} in your browser.
                  </b>{" "}
                  Approve it there, then come back — this page updates by itself.
                </span>
                <button
                  className="text-faint underline hover:text-muted shrink-0"
                  onClick={() => setConnFlow(null)}
                  data-testid="ob-connect-cancel"
                >
                  Cancel
                </button>
              </div>
            )}
          </div>
        );
      })}
    </>
  );
}
