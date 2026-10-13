import { useState } from "react";
import { resolveInboxItem, type InboxItem } from "../../api";

/** Approvals nobody answered for a day, with a bulk deny that asks first: it answers
 * several agents at once. */
export function StaleApprovals({ stale, onDone }: { stale: InboxItem[]; onDone: () => void }) {
  const [confirmStale, setConfirmStale] = useState(false);
  const denyStale = async () => {
    setConfirmStale(false);
    await Promise.all(stale.map((it) => resolveInboxItem(it.id, "deny").catch(() => null)));
    onDone();
  };

  if (stale.length === 0) return null;
  return (
    <div className="inbox-stale" data-testid="inbox-stale">
      {confirmStale ? (
        <>
          <span>
            Deny {stale.length} request{stale.length === 1 ? "" : "s"} waiting more than a day? The
            agents that asked will carry on without them.
          </span>
          <button className="btn sm danger" onClick={denyStale} data-testid="inbox-stale-confirm">
            Deny {stale.length}
          </button>
          <button className="btn sm ghost" onClick={() => setConfirmStale(false)}>
            Cancel
          </button>
        </>
      ) : (
        <>
          <span>
            {stale.length} approval{stale.length === 1 ? " has" : "s have"} been waiting more than a day.
          </span>
          <button className="btn sm" onClick={() => setConfirmStale(true)}>
            Deny {stale.length === 1 ? "it" : "them"}…
          </button>
        </>
      )}
    </div>
  );
}
