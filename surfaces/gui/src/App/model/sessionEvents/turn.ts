import type { MessageSource } from "../../../api";
import { speak } from "../../../tauri";
import { TRACE, trace } from "../../../trace";
import type { Item, WsEvent } from "../../../types";
import { addTurnUsage } from "../../../usage";
import type { SocketCtx } from "./handleEvent";

type Data = WsEvent["data"];

/** The first complete sentence of `text` and how much of `text` it used, if there is one yet. */
function firstSentence(text: string): { sentence: string; used: number } | null {
  const match = /^([\s\S]*?[.!?])(\s|$)/.exec(text);
  return match ? { sentence: match[1].trim(), used: match[0].length } : null;
}

/** Add `item` unless the transcript already ends with the same message (`same` decides). */
function appendOnce(c: SocketCtx, item: Item, same: (last: Item) => boolean) {
  c.setItems((before) => {
    const last = before[before.length - 1];
    return last && same(last) ? before : [...before, item];
  });
}

function onReady(c: SocketCtx, d: Data) {
  c.setConnected(true);
  TRACE && trace("chat", "ready", { session: d.session_id, agent: d.agent, model: d.model, mode: d.mode, shared: d.model_shared });
  // `model` is one value shared by every conversation, so whatever `ready` leaves alone is
  // inherited from the chat shown before. `if (d.model)` was therefore a leak: a session with
  // no explicit choice reported "", kept the previous chat's model, and picking a model in one
  // chat moved every chat that never chose onto it (2026-09-01: nemotron and gemma both ended
  // up gemma). The server now sends the model the turn would really run on and never an empty
  // one, so it is taken as given; a missing field would be a server bug, not a default.
  if (typeof d.model === "string" && d.model) c.setModel(d.model);
  // Whether every chat shares one endpoint for this model. The server answers that
  // (`providers::runs_locally`), since it takes telling an Ollama tag from a provider prefix.
  c.setModelShared(d.model_shared === true);
  if (d.mode) c.setMode(d.mode);
  if (d.command_trust?.required) c.setWorkspaceTrustRequest(d.command_trust);
  // Cowork: take the scratch folder the server made, unless there is a folder already.
  if (d.workspace) c.setWorkspace((current) => current || d.workspace);
}

function onTurnStart(c: SocketCtx, d: Data) {
  c.setRunning(true);
  c.setStreaming("");
  c.setReasoningStream("");
  // Coming back to a chat that is still working is not a new turn: the server replays
  // `turn_start` and sends `started_ms` so the clock keeps the turn's real age. Restarting it
  // here reset the elapsed time on every visit, so with two chats running neither showed its
  // true age.
  TRACE && trace("chat", "turn_start", { resumed: !!d.resumed, started_ms: d.started_ms });
  const resumedAt = d.resumed && typeof d.started_ms === "number" && d.started_ms > 0 ? d.started_ms : null;
  c.setTurnStartedAt(resumedAt ?? Date.now());
  // The replayed backlog sends this turn's `usage` frames again, so zero is only a starting
  // point that the replay corrects.
  c.setTurnTokens(0);
  // A turn that arrived in the background (a channel message, a self-wake, a resumed run) had
  // no local send(), so the message that started it isn't in `items` yet. A connector message
  // carries a structured `source` (§3.1) and gets the rich card; anything else is a plain user
  // item. A turn started from the composer already added its message, so a repeat is skipped.
  if (d.source?.connector) {
    const source = d.source as MessageSource;
    appendOnce(c, { kind: "connector", source }, (last) => last.kind === "connector" && last.source.ts === source.ts && last.source.text === source.text);
    return;
  }
  if (typeof d.input !== "string" || !d.input) return;
  // For a forced skill, `display` is the "/name …" line the user typed and `input` the framed
  // text for the model. What the user sees is what is shown and compared.
  const shown = (typeof d.display === "string" && d.display) || (d.input as string);
  appendOnce(c, { kind: "user", text: shown, ts: Date.now() / 1000 }, (last) => last.kind === "user" && last.text === shown);
}

function onDelta(c: SocketCtx, d: Data) {
  const piece = d.text || "";
  // Hands-free: each finished sentence goes to the voice as it arrives, so the first words are
  // spoken while the rest is still being written. The Rust side queues them, so they line up
  // instead of interrupting each other.
  if (c.handsFreeRef.current) {
    c.spokenBufRef.current += piece;
    const next = firstSentence(c.spokenBufRef.current);
    if (next) {
      c.spokenBufRef.current = c.spokenBufRef.current.slice(next.used);
      if (next.sentence) void speak(next.sentence, { persona: c.agent }).catch(() => {});
    }
  }
  c.setStreaming((sofar) => sofar + piece);
}

function onMessage(c: SocketCtx, d: Data) {
  if (d.usage) c.setUsage((totals) => addTurnUsage(totals, d.usage));
  // The event's own reasoning wins (it covers turns delivered in the background); the local
  // buffer is for servers that don't send it.
  const reasoning = d.reasoning || c.reasoningRef.current;
  if (d.text || reasoning) {
    const said: Extract<Item, { kind: "assistant" }> = { kind: "assistant", text: d.text || "", ts: Date.now() / 1000 };
    if (reasoning) said.reasoning = reasoning;
    if (d.persona) said.persona = d.persona;
    c.setItems((before) => [...before, said]);
  }
  // The text is in `items` now, or the turn only used tools.
  c.setStreaming("");
  c.setReasoningStream("");
}

/** The turn itself: connecting, starting, streamed text and reasoning, the final message. */
export function onTurnEvent(c: SocketCtx, ev: WsEvent, d: Data) {
  switch (ev.type) {
    case "ready":
      return onReady(c, d);
    case "turn_start":
      return onTurnStart(c, d);
    case "assistant_delta":
      return onDelta(c, d);
    case "reasoning_delta":
      return c.setReasoningStream(c.reasoningRef.current + (d.text || ""));
    case "assistant_message":
      return onMessage(c, d);
  }
}
