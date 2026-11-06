// One rule for all streamed text (§33, third refinement, owner 2026-07-14: the second
// version only gated the start of a turn, so narration in the middle of a turn still showed
// up as a loose paragraph). A stream is in one of four modes:
//
//   "hold"    the turn has just started and the text is under the threshold: only the
//             spinner shows. If a tool call follows, the text was narration and moves into
//             the step group.
//   "quiet"   mid-turn and under the threshold: the text belongs to the running step group.
//             A collapsed group carries it in its header ("Running 12 steps… · checking
//             the historical pages…"); an open one shows it as a small, quiet line.
//   "answer"  past the threshold with no tool call: this is the answer, so the streaming
//             bubble shows from here on.
//   "none"    nothing is streaming, or the session is idle.
//
// A running turn also starts collapsed (owner call): the header's live line is the sign of
// life, and opening the group is the person's choice.

import type { Item } from "./types";

// Text becomes the answer at 40 words, a second or two of streaming: a wait that short is fine.
export const STREAM_PROMOTE_WORDS = 40;

export type StreamMode = "none" | "hold" | "quiet" | "answer";

/** The newest item that isn't a notice; notices never say where a turn has got to. */
function lastReal(items: Item[]): Item | undefined {
  return [...items].reverse().find((item) => item.kind !== "notice");
}

// A turn is under way once its newest real item is a tool call, an approval or narration;
// text that streams right after the user's message is the turn's opening.
export function midTurn(items: Item[], running: boolean): boolean {
  const last = running ? lastReal(items) : undefined;
  return last !== undefined && ["tool", "approval", "assistant"].includes(last.kind);
}

// What the waiting row should say — or `null` to not render it at all.
//
// It used to be one fixed string, so a turn that had already run steps and was waiting on
// the model's next decision still read "Waiting for agent…" — as if nothing had happened
// yet, directly under a step group saying "Running 1 step…". The row can only honestly
// describe what we're waiting FOR, which is one of three things:
//
//   null                 a tool step is in flight — the step group already shows that
//                        state (its live spinner, or "Running N steps…" when collapsed),
//                        and a second, vaguer line about it is just noise.
//   "Thinking…"          steps have run and all of them settled: we're waiting on the
//                        model to decide the next one.
//   "Waiting for agent…" nothing has happened in this turn yet — the original wording,
//                        now only used when it's actually true.
//
// `contended` adds a fourth answer, and only where it is the true one. A local model
// serves a fixed number of sequences at once — Ollama starts its backend with `-np N`,
// 1 unless `OLLAMA_NUM_PARALLEL` says otherwise — so a second chat on the same model
// really is queued behind the first. Before this, both chats sat on "Waiting for
// agent…" for as long as the other one took, with nothing to say the app was fine and
// the model was busy (owner-hit 2026-09-01: two searches started back to back, each
// taking twice as long as it would alone).
//
// It replaces ONLY "Waiting for agent…" — the case where nothing has happened yet,
// which is exactly the wait contention explains. Once this turn has run a step, its
// own work is what it is waiting on and the queue is no longer the story.
const waitingFirst = (contended: boolean) =>
  contended ? "Waiting its turn — another chat is using this model" : "Waiting for agent…";

export function waitingLabel(items: Item[], contended = false): string | null {
  const item = lastReal(items);
  if (!item) return waitingFirst(contended);
  if (item.kind === "tool") return item.status === "…" ? null : "Thinking…";
  // An unresolved approval means the turn is not waiting on the model at all — it is
  // waiting on the person reading this. Saying "Thinking…" over a card that is
  // blocking on them describes the wrong party (UX-REPORT §7); the card itself is
  // the status, so the row yields to it.
  if (item.kind === "approval") return item.resolved ? "Thinking…" : null;
  // A narration line means the turn is underway; anything else (the user's own
  // message) means it has only just started.
  return item.kind === "assistant" ? "Thinking…" : waitingFirst(contended);
}

const wordCount = (text: string) => text.split(/\s+/).filter((word) => word.length > 0).length;

export function streamMode(streaming: string, items: Item[], running: boolean): StreamMode {
  if (streaming === "" || streaming == null) return "none";
  const promoted = !running || wordCount(streaming) >= STREAM_PROMOTE_WORDS;
  if (promoted) return "answer";
  return midTurn(items, running) ? "quiet" : "hold";
}
