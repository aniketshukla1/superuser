import { useEffect, useState } from "react";
import { getLedger, type LedgerState } from "../../api";

/**
 * What has changed in the vault since each source was last read.
 *
 * The ledger has existed since the vault became a git repository — every sync is
 * a commit, so "3 new from GitHub" is a diff rather than a guess — and it was
 * readable only over HTTP. That is the same criticism this whole plan opened with
 * about memory living in a settings tab: a record nobody can see is one nobody can
 * trust.
 *
 * Reading this never acknowledges anything. Looking at a page is not the same as
 * having read the items, and consuming the delta here would take it out from under
 * the background loop that was about to evaluate it.
 */
export function LedgerStrip() {
  const [ledger, setLedger] = useState<LedgerState | null>(null);

  useEffect(() => {
    getLedger().then(setLedger).catch(() => setLedger(null));
  }, []);

  // Nothing has ever synced: the ledger creates itself on the first one, so this
  // is the ordinary state of a fresh vault rather than something being wrong.
  if (!ledger?.initialized) return null;

  const changed = ledger.sources.filter((s) => s.added + s.modified + s.removed > 0);
  return (
    <div className="mem-count" data-testid="ledger-strip">
      {ledger.snapshots} snapshot{ledger.snapshots === 1 ? "" : "s"}
      {changed.length === 0
        ? " · nothing new since you last looked"
        : ` · ${changed.map((s) => s.summary.replace(/\.$/, "")).join(" · ")}`}
    </div>
  );
}
