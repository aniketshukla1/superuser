import type { WsEvent } from "../../../types";
import { finalizeAutomationRun } from "../../../api";
import { speak } from "../../../tauri";
import { settleRunningTools } from "../../../itemsFromMessages";
import { endBrowserAsks } from "../../../browserAsks";
import type { SocketCtx } from "./handleEvent";

/** Session state the engine reports: usage, model and mode, compaction, steering, errors, turn end. */
export function onStateEvent(c: SocketCtx, ev: WsEvent, d: WsEvent["data"], flushPartialStream: () => void) {
  const { agent, sessionId, setItems, setCompacting, setModel, setMode, setRunning, setTurnTokens,
      setBrowserRefreshKey, refreshSessions, activeRunRef, handsFreeRef, spokenBufRef } = c;
  switch (ev.type) {
    // Running token total for the turn, one message per model call.
    case "usage":
      if (typeof d.tokens === "number" && d.tokens > 0) setTurnTokens(d.tokens);
      break;
    case "turn_end":
      if (d.status === "max_iterations_exceeded")
        setItems((p) => [...p, { kind: "notice", tone: "warn", text: "Stopped: max iterations reached." }]);
      break;
    case "model_changed":
      // Mid-session switch (server-applied): update the header fact and drop the
      // persisted marker into the live transcript (replay renders it from history).
      if (d.model) setModel(d.model);
      setItems((p) => [...p, { kind: "notice", tone: "info", text: d.text || "Model switched" }]);
      break;
    // The mode moved, not the model. Both used to arrive as `model_changed`, so
    // switching from Interactive to Plan wrote "Model switched" into the
    // transcript — a durable, false statement about what happened. The mode is
    // visible in the composer where it was just set, so this only syncs state.
    case "mode_changed":
      if (d.model) setModel(d.model);
      if (d.mode) setMode(d.mode);
      break;
    case "compacting":
      setCompacting(true);
      break;
    case "compacted":
      // Auto-compaction marker (OPE-27): outbound-only — the transcript stays intact,
      // this divider just shows where the model's memory was summarized.
      setItems((p) => [...p, { kind: "notice", tone: "info", text: d.text || "Context compacted" }]);
      break;
    case "steered":
      // Sent while the agent was working. It's already in the transcript on the
      // server; echo it here so the message doesn't appear to vanish, and say
      // it hasn't landed yet — it reaches the model at the next step boundary,
      // which can be a few seconds into a long tool call.
      setItems((p) => [
        ...p,
        { kind: "user", text: d.text || "" },
        { kind: "notice", tone: "info", text: "Queued — the agent picks this up at its next step." },
      ]);
      break;
    case "interrupted":
      flushPartialStream();
      // Interrupted is terminal for the UI. Without this the composer stayed in
      // its running state forever — the Stop button never went away, which reads
      // as "Stop doesn't work" even though the backend had stopped.
      setRunning(false);
      setItems((p) => [
        ...settleRunningTools(p, "stopped"),
        { kind: "notice", tone: "warn", text: "Interrupted." },
      ]);
      break;
    case "error":
      flushPartialStream();
      // Terminal, like `interrupted`: a failed turn sends no turn_done, so without
      // this the composer stayed in its running state forever and the turn looked
      // hung rather than failed.
      setRunning(false);
      setItems((p) => [
        ...settleRunningTools(p, "error"),
        {
          kind: "notice",
          tone: "warn",
          text: "Error: " + (d.error || d.message || "unknown"),
          retriable: true,
        },
      ]);
      break;
    case "input_rejected":
      setItems((p) => [
        ...p,
        { kind: "notice", tone: "warn", text: d.error || "That message was rejected." },
      ]);
      break;
    case "turn_done":
      setRunning(false);
      // Whatever is left has no closing full stop (a list item, a trailing
      // clause); say it rather than dropping the end of the answer.
      if (handsFreeRef.current) {
        const tail = spokenBufRef.current.trim();
        spokenBufRef.current = "";
        if (tail) void speak(tail, { persona: agent }).catch(() => {});
      }
      // The turn succeeded, so a step that never reported is treated as done, not
      // failed — a red step under a good answer would be a lie.
      setItems((p) => endBrowserAsks(settleRunningTools(p, "ok")));
      refreshSessions();
      // Catch-all artifact refresh: files created via shell or on a brand-new session (whose
      // record only exists after the first save) appear once the turn completes.
      setBrowserRefreshKey((k) => k + 1);
      // Finalize a manual run after its first turn completes (mark it ok in history).
      {
        const ar = activeRunRef.current;
        if (ar && ar.sessionId === sessionId) {
          activeRunRef.current = null;
          finalizeAutomationRun(ar.taskId, ar.runId).catch(() => {});
        }
      }
      break;
  }
}
