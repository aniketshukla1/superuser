import { finalizeAutomationRun } from "../../../api";
import { endBrowserAsks } from "../../../browserAsks";
import { settleRunningTools } from "../../../itemsFromMessages";
import { speak } from "../../../tauri";
import type { Item, WsEvent } from "../../../types";
import type { SocketCtx } from "./handleEvent";

type Data = WsEvent["data"];
type Notice = Extract<Item, { kind: "notice" }>;

const notice = (tone: Notice["tone"], text: string, retriable?: boolean): Notice =>
  retriable ? { kind: "notice", tone, text, retriable } : { kind: "notice", tone, text };

function append(c: SocketCtx, ...added: Item[]) {
  c.setItems((before) => [...before, ...added]);
}

/** A turn that stops early (interrupted or failed) sends no turn_done, so the composer is
 *  released here; otherwise Stop never went away and the turn looked hung. */
function stopEarly(c: SocketCtx, keepPartial: () => void, toolStatus: string, said: Notice) {
  keepPartial();
  c.setRunning(false);
  c.setItems((before) => [...settleRunningTools(before, toolStatus), said]);
}

function finishTurn(c: SocketCtx) {
  c.setRunning(false);
  // Hands-free: what's left had no closing full stop (a list item, a trailing clause), so it is
  // spoken now rather than dropped.
  if (c.handsFreeRef.current) {
    const rest = c.spokenBufRef.current.trim();
    c.spokenBufRef.current = "";
    if (rest) void speak(rest, { persona: c.agent }).catch(() => {});
  }
  // The turn succeeded, so a step that never reported back counts as done: a red step under a
  // good answer would be untrue.
  c.setItems((before) => endBrowserAsks(settleRunningTools(before, "ok")));
  c.refreshSessions();
  // Files written by a shell command, or by a brand-new session whose record only exists after
  // its first save, show up once the turn is over.
  c.setBrowserRefreshKey((key) => key + 1);
  // A manual run is marked done after its first turn.
  const run = c.activeRunRef.current;
  if (run && run.sessionId === c.sessionId) {
    c.activeRunRef.current = null;
    finalizeAutomationRun(run.taskId, run.runId).catch(() => {});
  }
}

/** What the engine reports about the session: usage, model and mode, compaction, steering,
 *  errors, and the end of a turn. */
export function onStateEvent(c: SocketCtx, ev: WsEvent, d: Data, keepPartial: () => void) {
  switch (ev.type) {
    case "usage":
      // The turn's running token total, sent once per model call.
      if (typeof d.tokens === "number" && d.tokens > 0) c.setTurnTokens(d.tokens);
      return;
    case "turn_end":
      if (d.status === "max_iterations_exceeded") append(c, notice("warn", "Stopped: max iterations reached."));
      return;
    case "model_changed":
      // Switched mid-session by the server: the header updates, and the saved marker appears in
      // the live transcript (a replay draws it from history).
      if (d.model) c.setModel(d.model);
      append(c, notice("info", d.text || "Model switched"));
      return;
    case "mode_changed":
      // The mode changed, not the model. Both once came as `model_changed`, which wrote "Model
      // switched" into the transcript when only Interactive became Plan: a lasting, false
      // record. The composer already shows the mode, so this only updates state.
      if (d.model) c.setModel(d.model);
      if (d.mode) c.setMode(d.mode);
      return;
    case "compacting":
      c.setCompacting(true);
      return;
    case "compacted":
      // OPE-27: only what is sent to the model is summarised. The transcript stays whole; this
      // divider marks where the model's memory was condensed.
      append(c, notice("info", d.text || "Context compacted"));
      return;
    case "steered":
      // Sent while the agent was working. The server already has it in the transcript; it is
      // echoed here so it doesn't seem to vanish, with a note that it hasn't arrived yet: the
      // model gets it at the next step, which can be seconds into a long tool call.
      append(c, { kind: "user", text: d.text || "" }, notice("info", "Queued — the agent picks this up at its next step."));
      return;
    case "interrupted":
      return stopEarly(c, keepPartial, "stopped", notice("warn", "Interrupted."));
    case "error":
      return stopEarly(c, keepPartial, "error", notice("warn", "Error: " + (d.error || d.message || "unknown"), true));
    case "input_rejected":
      append(c, notice("warn", d.error || "That message was rejected."));
      return;
    case "turn_done":
      return finishTurn(c);
  }
}
