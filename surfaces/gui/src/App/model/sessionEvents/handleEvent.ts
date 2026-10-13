import type { WsEvent } from "../../../types";
import type { useSessionSocket } from "../useSessionSocket";
import { onTurnEvent } from "./turn";
import { onRequestEvent } from "./requests";
import { onStateEvent } from "./state";

/** Everything an event handler reads: the socket hook's inputs and its voice refs. */
export type SocketCtx = Parameters<typeof useSessionSocket>[0] & { handsFreeRef: { current: boolean }; spokenBufRef: { current: string } };

/** Applies one engine event to the app's state. Built per connection, so it reads the
 * values the socket was opened with — as the inline handler it replaces did. */
export function eventHandler(c: SocketCtx) {
  const { streamingRef, reasoningRef, setStreaming, setReasoningStream, setItems, setCompacting } = c;
  return (ev: WsEvent) => {
    const d = ev.data || {};
    // An interrupted/errored turn never emits assistant_message, so its streamed partial
    // would otherwise live only in the ephemeral buffer until the next turn_start wipes it
    // (owner-hit 2026-07-22). Promote it to a durable transcript item — the engine persists
    // the same text server-side, so the live view and a session reload now agree.
    const flushPartialStream = () => {
      const partial = streamingRef.current;
      const thinking = reasoningRef.current;
      if (!partial && !thinking) return;
      setStreaming("");
      setReasoningStream("");
      setItems((p) => [
        ...p,
        {
          kind: "assistant",
          text: partial,
          ts: Date.now() / 1000,
          ...(thinking ? { reasoning: thinking } : {}),
        },
      ]);
    };
    // Any engine event after `compacting` means the summarizer finished (compacted /
    // silent no-op / failure prompt) — the transient must never outlive it.
    if (ev.type !== "compacting") setCompacting(false);
    switch (ev.type) {
      case "ready": case "turn_start": case "assistant_delta": case "reasoning_delta":
      case "assistant_message":
        return onTurnEvent(c, ev, d);
      case "tool_proposed": case "permission_required": case "directory_requested":
      case "plan_proposed": case "question_requested": case "computer_frame":
      case "computer_help": case "computer_secret": case "form_requested": case "tool_finished":
        return onRequestEvent(c, ev, d, flushPartialStream);
      case "usage": case "turn_end": case "model_changed": case "mode_changed": case "compacting":
      case "compacted": case "steered": case "interrupted": case "error": case "input_rejected":
      case "turn_done":
        return onStateEvent(c, ev, d, flushPartialStream);
    }
  };
}
