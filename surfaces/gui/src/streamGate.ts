// §33 refinement #3 (owner, 2026-07-14 — v2's gate only covered turn START; mid-turn
// narration still painted as a floating paragraph): ONE rule for all streamed text.
//
//   mode = "hold"   turn-start, under the threshold — show nothing but the spinner; if a
//                   tool call arrives the text was narration and renders inside the group.
//   mode = "quiet"  mid-turn, under the threshold — the text belongs to the LIVE turn
//                   group: on its collapsed header ("Running 12 steps… · checking the
//                   historical pages…") or as the small quiet line when expanded. Never a
//                   floating paragraph.
//   mode = "answer" crossed the threshold with no tool call — it's the answer; render the
//                   streaming bubble from that point.
//   mode = "none"   nothing streaming (or session idle).
//
// Turns also start COLLAPSED while running (owner call): the header's live line is the
// pulse; expanding is opt-in.

import type { Item } from "./types";

// Owner call: 40 words (~1-2s of stream) — "people can wait 1-2 seconds longer".
export const STREAM_PROMOTE_WORDS = 40;

export type StreamMode = "none" | "hold" | "quiet" | "answer";

// The turn already has activity behind it when the last real item is a tool, an approval,
// or a narration line; a stream right after the user's message is turn-start.
export function midTurn(items: Item[], running: boolean): boolean {
  if (!running) return false;
  for (let i = items.length - 1; i >= 0; i--) {
    const item = items[i];
    if (item.kind === "notice") continue;
    return item.kind === "tool" || item.kind === "approval" || item.kind === "assistant";
  }
  return false;
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
export function waitingLabel(items: Item[], contended = false): string | null {
  for (let i = items.length - 1; i >= 0; i--) {
    const item = items[i];
    if (item.kind === "notice") continue;
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
  return waitingFirst(contended);
}

const waitingFirst = (contended: boolean) =>
  contended ? "Waiting its turn — another chat is using this model" : "Waiting for agent…";

export function streamMode(streaming: string, items: Item[], running: boolean): StreamMode {
  if (!streaming) return "none";
  const words = streaming.trim().split(/\s+/).filter(Boolean).length;
  if (words >= STREAM_PROMOTE_WORDS || !running) return "answer";
  return midTurn(items, running) ? "quiet" : "hold";
}
