import type { WsEvent } from "../../../types";
import { TRACE, trace } from "../../../trace";
import type { MessageSource } from "../../../api";
import { speak } from "../../../tauri";
import { addTurnUsage } from "../../../usage";
import type { SocketCtx } from "./handleEvent";

/** The turn itself: connect, start, streamed text and reasoning, the final message. */
export function onTurnEvent(c: SocketCtx, ev: WsEvent, d: WsEvent["data"]) {
  const { agent, reasoningRef, setStreaming, setReasoningStream, setItems, setConnected, setModel,
      setModelShared, setMode, setWorkspaceTrustRequest, setWorkspace, setRunning,
      setTurnStartedAt, setTurnTokens, setUsage, handsFreeRef, spokenBufRef } = c;
  switch (ev.type) {
    case "ready":
      setConnected(true);
      // `model` is one piece of App state shared by every conversation, so
      // whatever `ready` does NOT overwrite is inherited from the chat you were
      // looking at before. That made `if (d.model)` a bleed: a session with no
      // explicit pick reported "", the model silently stayed on the previous
      // chat's, and choosing a model in one chat moved every never-configured
      // chat onto it (reported 2026-09-01 — nemotron and gemma, both ended up
      // gemma). The server now resolves this to the model the turn would
      // actually run on and never sends an empty one, so it is taken at face
      // value; a missing field would be a server bug, not a default.
      TRACE && trace("chat", "ready", {
        session: d.session_id,
        agent: d.agent,
        model: d.model,
        mode: d.mode,
        shared: d.model_shared,
      });
      if (typeof d.model === "string" && d.model) setModel(d.model);
      // Whether every chat shares one endpoint for this model — the server owns
      // that question (`providers::runs_locally`), because getting it right means
      // knowing an Ollama tag from a provider prefix.
      setModelShared(d.model_shared === true);
      if (d.mode) setMode(d.mode);
      if (d.command_trust?.required) setWorkspaceTrustRequest(d.command_trust);
      // Cowork: adopt the server-provisioned scratch dir (only when we don't already have one).
      if (d.workspace) setWorkspace((cur) => cur || d.workspace);
      break;
    case "turn_start":
      setRunning(true);
      setStreaming("");
      setReasoningStream("");
      // A REATTACH is not a new turn. The server replays `turn_start` when you
      // come back to a chat that is still working, and starting the clock here
      // meant the elapsed time reset to zero on every visit — with two chats both
      // running, switching between them showed neither turn's real age. It sends
      // `started_ms` for exactly this; nothing read it.
      TRACE && trace("chat", "turn_start", { resumed: !!d.resumed, started_ms: d.started_ms });
      setTurnStartedAt(
        d.resumed && typeof d.started_ms === "number" && d.started_ms > 0
          ? d.started_ms
          : Date.now(),
      );
      // The replayed backlog re-sends this turn's `usage` frames, so zeroing here
      // is a starting point the replay corrects — not a number the user is left
      // looking at.
      setTurnTokens(0);
      // Background-delivered turns (channel message, self-wake, durable resume) have no local
      // send(), so the triggering message isn't in `items` yet — surface it. A connector message
      // carries a structured `source` (§3.1) → render the rich card; otherwise a plain user item.
      // Foreground turns already appended it in send(); skip the duplicate.
      if (d.source?.connector) {
        const src = d.source as MessageSource;
        setItems((p) => {
          const last = p[p.length - 1];
          return last && last.kind === "connector" && last.source.ts === src.ts && last.source.text === src.text
            ? p
            : [...p, { kind: "connector", source: src }];
        });
      } else if (typeof d.input === "string" && d.input) {
        // `display` (force-run) is the user's literal "/name …" line; the framed
        // `input` is model-facing. Surface/dedupe on what the user actually sees.
        const shown = (typeof d.display === "string" && d.display) || (d.input as string);
        setItems((p) => {
          const last = p[p.length - 1];
          return last && last.kind === "user" && last.text === shown
            ? p
            : [...p, { kind: "user", text: shown, ts: Date.now() / 1000 }];
        });
      }
      break;
    case "assistant_delta":
      // Hands-free: hand each finished sentence to the voice as it arrives, so
      // the first words are spoken while the rest is still being written. The
      // Rust side queues them, so these calls accumulate rather than cutting
      // each other off.
      if (handsFreeRef.current) {
        spokenBufRef.current += d.text || "";
        const ready = /^([\s\S]*?[.!?])(\s|$)/.exec(spokenBufRef.current);
        if (ready) {
          const sentence = ready[1].trim();
          spokenBufRef.current = spokenBufRef.current.slice(ready[0].length);
          if (sentence) void speak(sentence, { persona: agent }).catch(() => {});
        }
      }
      setStreaming((s) => s + (d.text || ""));
      break;
    case "reasoning_delta":
      setReasoningStream(reasoningRef.current + (d.text || ""));
      break;
    case "assistant_message": {
      if (d.usage) setUsage((u) => addTurnUsage(u, d.usage));
      // The event's reasoning is authoritative (covers background-delivered turns);
      // the local buffer is the fallback for older servers.
      const reasoning = d.reasoning || reasoningRef.current;
      if (d.text || reasoning)
        setItems((p) => [
          ...p,
          {
            kind: "assistant",
            text: d.text || "",
            ts: Date.now() / 1000,
            ...(reasoning ? { reasoning } : {}),
            ...(d.persona ? { persona: d.persona } : {}),
          },
        ]);
      setStreaming(""); // finalized into items (or empty tool-only turn)
      setReasoningStream("");
      break;
    }
  }
}
