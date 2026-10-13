import type { ApprovalDecision, Attachment } from "../../types";
import { resolveLastApproval, resolveLastPlan, resolveLastDirReq, resolveLastQuestion, newId, GENERAL_ASSISTANT } from "../appHelpers";
import { TRACE, trace } from "../../trace";
import { emptyUsage } from "../../usage";
import { useState, useEffect } from "react";
import { connectEvents, announceAutomationsChanged } from "../../api";
import { clock } from "../../when";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useSessionSocket } from "./useSessionSocket";
import type { useFollowScroll } from "./useFollowScroll";
import type { useVoiceAndSend } from "./useVoiceAndSend";

/** Answers to the agent's requests, interrupt/retry, mode and model changes, new sessions. */
export function useTurnActions(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> & ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend>) {
  const { setSessionInbox, setItems, sessionRef, setMode, setComposerPrefill, setRunning, running,
      model, sessionId, setModel, agent, setSurface, setUsage, setStreaming, setTodo, setAgent,
      gatesWorkspace, setWorkspace, setBranch, openGate, workspace, branch, setShowGate,
      setSessionId } = prev;
  // Resolving a LIVE prompt also resolves its parked Inbox mirror server-side, but the polled
  // `sessionInbox` copy stays "pending" for up to a poll cycle — long enough for the docked
  // answer-in-context card to flash the SAME request again right after the user answered it
  // (tester catch 2026-07-12: a Slack send "asked twice"). Drop the mirror optimistically;
  // the 4s poll restores anything genuinely still pending.
  const dropSessionInbox = (kind: string) =>
    setSessionInbox((cur) => cur.filter((it) => it.kind !== kind));
  const approve = (decision: ApprovalDecision) => {
    setItems((p) => resolveLastApproval(p, decision));
    dropSessionInbox("approval");
    sessionRef.current?.approve(decision);
  };
  const respondPlan = (approved: boolean, mode?: string, feedback?: string) => {
    setItems((p) => resolveLastPlan(p, approved ? "approved" : "rejected"));
    dropSessionInbox("plan");
    sessionRef.current?.respondPlan(approved, mode, feedback);
    if (approved && mode) setMode(mode); // the server flips the live engine to this mode
  };
  const respondDirectory = (granted: boolean, path?: string, writable?: boolean) => {
    setItems((p) => resolveLastDirReq(p, granted ? "granted" : "denied"));
    dropSessionInbox("directory");
    sessionRef.current?.respondDirectory(granted, path, writable);
  };
  const answerQuestion = (answers: (string | null)[]) => {
    setItems((p) => resolveLastQuestion(p, JSON.stringify(answers)));
    dropSessionInbox("question");
    sessionRef.current?.respondQuestion(answers);
  };
  const answerForm = (answers: Record<string, string>) => {
    setItems((p) => {
      const out = [...p];
      for (let i = out.length - 1; i >= 0; i--) {
        const it = out[i];
        if (it.kind === "form" && !it.resolved) {
          out[i] = { ...it, resolved: JSON.stringify(answers) };
          break;
        }
      }
      return out;
    });
    sessionRef.current?.respondForm(answers);
  };
  const prefillComposer = (text: string, attachments?: Attachment[]) =>
    setComposerPrefill((p) => ({ text, attachments, nonce: (p?.nonce ?? 0) + 1 }));
  const interrupt = () => sessionRef.current?.interrupt();
  const retry = () => {
    // Optimistic running: turn_start confirms; a rejected retry still ends in turn_done.
    setRunning(true);
    sessionRef.current?.retry();
  };
  const changeMode = (m: string) => {
    setMode(m);
    sessionRef.current?.setMode(m);
  };
  const changeModel = (m: string) => {
    if (running) return; // the server refuses mid-turn rebinds — don't let the header lie
    TRACE && trace("chat", "model picked", { from: model, to: m, session: sessionId });
    setModel(m);
    sessionRef.current?.setModel(m);
  };

  const startNewSession = (forAgent?: string) => {
    const target = forAgent || agent;
    setSurface("session"); // return to the conversation view if we were on a sub-view
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);
    // "New session" under a browsed persona switches to it (expand≠switch: the header alone
    // doesn't switch; this explicit action does).
    if (target !== agent) {
      setAgent(target);
      if (gatesWorkspace(target)) {
        // Never inherit the previous persona's folder — it may be a scratch dir. Clearing it
        // also blocks the connection effect, so nothing can chat behind the open gate.
        // `agent`/`workspace`/`sessionId` still hold the PREVIOUS values here, which is
        // exactly what cancelling has to return to.
        setWorkspace(null);
        setBranch(null);
        openGate({ agent, workspace, sessionId, branch });
      } else setShowGate(false);
    }
    // Knowledge family: a new conversation starts fresh (orphan) — clear the workspace so the
    // server provisions a NEW scratch dir for the new session id. Code keeps its repo.
    if (!gatesWorkspace(target)) setWorkspace(null);
    setSessionId(newId());
  };
  // Inbox → session: the item carries its session's workspace/agent, so open it directly.
  // UX-026: 5s top-right toast when a SCHEDULED automation run starts (never for
  // manual Run-now — the user is already watching). Rides the app-wide /ws/events
  // stream; View run opens the run's live session.
  const [runToast, setRunToast] = useState<{
    title: string; sessionId: string; workspace: string; agent: string; time: string;
  } | null>(null);
  useEffect(() => {
    const stop = connectEvents((msg) => {
      if (msg.type !== "automation_run_started") return;
      const d = (msg.data ?? {}) as Record<string, string>;
      setRunToast({
        title: d.task_title || "Automation",
        sessionId: d.session_id || "",
        workspace: d.workspace || "",
        agent: d.agent || GENERAL_ASSISTANT,
        time: clock(new Date()),
      });
      announceAutomationsChanged(); // the Scheduled band's badge is now stale
    });
    return stop;
  }, []);
  useEffect(() => {
    if (!runToast) return;
    const t = window.setTimeout(() => setRunToast(null), 5000);
    return () => window.clearTimeout(t);
  }, [runToast]);

  return { approve, respondPlan, respondDirectory, answerQuestion, answerForm, prefillComposer, interrupt, retry, changeMode, changeModel, startNewSession, runToast, setRunToast };
}
