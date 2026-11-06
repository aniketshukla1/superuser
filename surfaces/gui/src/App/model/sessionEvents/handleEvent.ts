import type { EventType, Item, WsEvent } from "../../../types";
import type { useSessionSocket } from "../useSessionSocket";
import { onRequestEvent } from "./requests";
import { onStateEvent } from "./state";
import { onTurnEvent } from "./turn";

/** What the event handlers read: the socket hook's inputs plus its voice refs. */
export type SocketCtx = Parameters<typeof useSessionSocket>[0] & { handsFreeRef: { current: boolean }; spokenBufRef: { current: string } };

// Which module handles which event.
const TURN_EVENTS = new Set<EventType>(["ready", "turn_start", "assistant_delta", "reasoning_delta", "assistant_message"]);
const REQUEST_EVENTS = new Set<EventType>([
  "tool_proposed", "tool_finished", "permission_required", "directory_requested", "plan_proposed", "question_requested",
  "form_requested", "computer_frame", "computer_help", "computer_secret",
]);
const STATE_EVENTS = new Set<EventType>([
  "usage", "turn_end", "turn_done", "model_changed", "mode_changed", "compacting", "compacted", "steered", "interrupted",
  "error", "input_rejected",
]);

/** Applies one engine event to the app's state. It is made per connection, so it reads the
 *  values the socket was opened with, just as the inline handler it replaced did. */
export function eventHandler(c: SocketCtx) {
  const { reasoningRef, setCompacting, setItems, setReasoningStream, setStreaming, streamingRef } = c;

  // A turn that was interrupted or failed never sends assistant_message, so what it had
  // streamed would live only in the passing buffer until the next turn_start cleared it
  // (owner, 2026-07-22). It becomes a lasting transcript item instead; the engine saves the
  // same text, so the live view and a reload of the session agree.
  const keepPartial = () => {
    const text = streamingRef.current;
    const reasoning = reasoningRef.current;
    if (!text && !reasoning) return;
    setStreaming("");
    setReasoningStream("");
    const kept: Extract<Item, { kind: "assistant" }> = { kind: "assistant", text, ts: Date.now() / 1000 };
    if (reasoning) kept.reasoning = reasoning;
    setItems((before) => [...before, kept]);
  };

  return (event: WsEvent) => {
    const data = event.data || {};
    // Whatever the engine sends after `compacting` means the summary is over (done, nothing to
    // do, or failed), so the indicator must not outlast it.
    if (event.type !== "compacting") setCompacting(false);
    if (TURN_EVENTS.has(event.type)) return onTurnEvent(c, event, data);
    if (REQUEST_EVENTS.has(event.type)) return onRequestEvent(c, event, data, keepPartial);
    if (STATE_EVENTS.has(event.type)) return onStateEvent(c, event, data, keepPartial);
  };
}
