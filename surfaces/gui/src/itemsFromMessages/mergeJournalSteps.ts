import type { ToolCall } from "../components/ActivityTab";
import type { Item } from "../types";

/** Index of the last thing the user sent (a message or a connector message), or -1. */
function lastAskIndex(items: Item[]): number {
  for (let at = items.length - 1; at >= 0; at--) {
    if (items[at].kind === "user" || items[at].kind === "connector") return at;
  }
  return -1;
}

/** Puts the last turn's steps back into a reopened conversation.
 *
 *  The session store keeps the answer, not the work, so a conversation you came back to read
 *  as if the model had simply known: "The files in the persistent working directory are:
 *  alpha.md" under a request to run a shell command, with nothing in between. The work did
 *  survive, since the run journal keeps every frame of the latest turn (`core::runlog`), but
 *  only the side panel read it.
 *
 *  The journal's calls go where they happened: after the user's last message, before the reply
 *  and any notice after it. Only the last turn, because that is all the journal holds; older
 *  turns don't borrow steps that weren't theirs. A turn that already has steps (saved charts,
 *  or older histories with `tool_calls`) is left as it is, so nothing shows twice.
 *
 *  A call the journal never saw finish belongs to a run that is over (only an idle journal is
 *  merged), so it shows as stopped rather than spinning. */
export function mergeJournalSteps(items: Item[], calls: ToolCall[]): Item[] {
  if (calls.length === 0) return items;
  const ask = lastAskIndex(items);
  const after = items.slice(ask + 1);
  if (after.some((it) => it.kind === "tool")) return items;
  // Nothing after the question means a turn that never answered or is still answering; either
  // way there's no reply for these steps to stand in front of.
  if (ask >= 0 && after.length === 0) return items;
  const steps = calls.map((call): Item => {
    const step: Extract<Item, { kind: "tool" }> = {
      kind: "tool",
      id: `journal-${call.seq}`,
      name: call.name,
      args: call.args,
      status: call.pending || call.stopped ? "stopped" : call.status || "ok",
    };
    if (call.result) step.preview = call.result;
    return step;
  });
  return [...items.slice(0, ask + 1), ...steps, ...after];
}
