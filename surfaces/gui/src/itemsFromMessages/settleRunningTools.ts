import type { Item } from "../types";

/** Settle every step still on its spinner, for when a turn reaches a terminal state.
 *
 *  A turn is over when the server says it's over, so nothing may still be "running"
 *  underneath it. TurnGroup derives its own running flag from the steps
 *  (`tools.some(t => t.status === "…")`), so one unsettled step kept a finished turn
 *  reading "Running 3 steps…" with live spinners under a printed answer. That happens
 *  whenever a step's completion never lands: a dropped frame, a turn cancelled
 *  mid-call, or a server that doesn't report one.
 *
 *  Returns the original array when there's nothing to settle, so React can skip the
 *  re-render. */
export function settleRunningTools(items: Item[], status: string): Item[] {
  let touched = false;
  const settled = items.map((it) => {
    if (it.kind === "tool" && it.status === "…") {
      touched = true;
      return { ...it, status };
    }
    return it;
  });
  return touched ? settled : items;
}
