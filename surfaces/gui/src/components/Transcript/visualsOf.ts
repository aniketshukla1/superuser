import type { ApprovalDecision, Item } from "../../types";
import { asChartSpec, type ChartSpec } from "../Chart";
import { asTableSpec, type TableSpec } from "../DataTable";
import { asCardSpec, type CardSpec } from "../FactCard";
import type { TurnItem } from "./ReplyMeta";

export interface Props {
  /** Open a fan-out sub-thread from its link in the transcript. */
  onOpenThread?: (sessionId: string) => void;
  /** The conversation these items belong to — needed to file a verdict against one. */
  sessionId?: string;
  items: Item[];
  onApprove: (decision: ApprovalDecision) => void;
  // The session's live flag. While true, the FINAL run's trailing assistant text is still
  // narration (status), not the answer — promoting it early made each line flash as a full
  // ASSISTANT bubble and then vanish into the group when the next tool call arrived
  // (owner report 2026-07-13). The answer bubble appears once, when the turn ends.
  running?: boolean;
  // Sub-threshold streamed text (streamGate mode "quiet") — handed to the live turn group.
  streamingText?: string;
  // Re-run the failed turn (no new user message). Offered only on a retriable notice that
  // is the transcript tail of an idle session — anywhere else the error is history.
  onRetry?: () => void;
  /** Open an assistant's browser live, from a screen its turn kept. */
  onWatch?: (persona: string) => void;
  /** Go to where a failure's fix lives (a missing key, a rejected model). */
  onOpenSettings?: (tab: "models") => void;
  /** An assistant's name from its id, for a reply another one gave (an `@` mention). */
  personaName?: (id: string) => string;
}

// The transcript index whose notice gets the Retry button: the tail error notice, looking
// through info notices after it (model switches must not consume the retry — switching
// models and THEN retrying is the intended recovery path). -1 when the tail is anything else.
export function retryAnchor(items: Item[]): number {
  for (let i = items.length - 1; i >= 0; i--) {
    const it = items[i];
    if (it.kind !== "notice") return -1;
    if (it.retriable) return i;
    if (it.tone !== "info") return -1;
  }
  return -1;
}

/** The charts a turn drew, in the order it drew them.
 *
 *  The spec is already here: `chart` calls arrive with their arguments, and the panel's
 *  `.chart.json` file is written from the same object. So drawing it in the conversation
 *  costs one read of what the step already carries — no extra event, no fetch, nothing to
 *  keep in sync with the file on disk.
 *
 *  Only successful calls. A refused chart (see core::gates) has a spec too, and drawing it
 *  would render exactly the fabricated picture the gate just stopped. */
export type Visual =
  | { chart: ChartSpec }
  | { table: TableSpec }
  | { card: CardSpec }
  | { shot: ComputerItem }
  | { view: { title: string; html: string; source?: string } };

/** A screen the assistant's browser kept — drawn beside the charts, for the same reason:
 *  a turn renders collapsed, and a page nobody expands to see was never shown. */
export type ComputerItem = Extract<Item, { kind: "computer" }>;

/** The visuals a turn produced, IN THE ORDER IT PRODUCED THEM.
 *
 *  One pass over the steps rather than one pass per kind, so a turn that draws a
 *  chart, then a table, then another chart reads in that order. Collecting by kind
 *  would group them and quietly rearrange the argument the turn was making. */
export function visualsOf(turn: TurnItem[]): Visual[] {
  const out: Visual[] = [];
  for (const it of turn) {
    if (it.kind !== "tool" || it.status !== "ok") continue;
    const source = it.args?.source ? String(it.args.source) : undefined;
    if (it.name === "chart") {
      const spec = asChartSpec(it.args?.spec);
      if (spec) out.push({ chart: source ? { ...spec, source } : spec });
    } else if (it.name === "table") {
      const spec = asTableSpec(it.args?.spec);
      if (spec) out.push({ table: source ? { ...spec, source } : spec });
    } else if (it.name === "card") {
      const spec = asCardSpec(it.args?.spec);
      if (spec) out.push({ card: source ? { ...spec, source } : spec });
    } else if (it.name === "ui_component") {
      // A view the model laid out itself. Drawn beside the charts, and sealed the same
      // way wherever it is drawn — see `GeneratedView`.
      const title = typeof it.args?.title === "string" ? it.args.title : "";
      const html = typeof it.args?.html === "string" ? it.args.html : "";
      if (title && html) out.push({ view: { title, html, ...(source ? { source } : {}) } });
    }
  }
  return out;
}
