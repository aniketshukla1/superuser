import { fetch, httpBase, jsonOf } from "./neverRejectingJson";

// -- The ledger: what changed in the vault since you last looked --------------
// The vault is a git repository and every sync is a commit, so "what's new from
// GitHub" is a diff rather than a guess — see `core/ledger.rs`.

export interface LedgerSource {
  source: string;
  added: number;
  modified: number;
  removed: number;
  /** Files this source has in the vault right now, changed or not. */
  total: number;
  summary: string;
}

export interface LedgerState {
  /** False until the first sync creates the repository. */
  initialized: boolean;
  snapshots: number;
  sources: LedgerSource[];
}

/** Reading this page never marks anything as read — the marker only moves when
 *  something actually consumes the delta. */
export async function getLedger(): Promise<LedgerState> {
  const res = await fetch(`${httpBase()}/v1/ledger`);
  if (!res.ok) return { initialized: false, snapshots: 0, sources: [] };
  const j = await jsonOf(res);
  return { initialized: j.initialized ?? false, snapshots: j.snapshots ?? 0, sources: j.sources ?? [] };
}
