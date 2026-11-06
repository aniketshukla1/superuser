import { useEffect, useState } from "react";
import { announceAutomationsChanged, connectEvents } from "../../api";
import { TRACE, trace } from "../../trace";
import type { ApprovalDecision, Attachment, Item } from "../../types";
import { emptyUsage } from "../../usage";
import { clock } from "../../when";
import {
  GENERAL_ASSISTANT, newId, resolveLastApproval, resolveLastDirReq, resolveLastForm, resolveLastPlan, resolveLastQuestion,
} from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useFollowScroll } from "./useFollowScroll";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";
import type { useSessionSocket } from "./useSessionSocket";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useVoiceAndSend } from "./useVoiceAndSend";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> &
  ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend>;

type RunToastState = { title: string; sessionId: string; workspace: string; agent: string; time: string };

const TOAST_MS = 5000;

/** Answering the agent's requests, stopping and retrying, changing mode or model, and starting
 *  new sessions. */
export function useTurnActions(prev: Deps) {
  const {
    agent, branch, model, running, sessionId, sessionRef, workspace, gatesWorkspace, openGate, setAgent, setBranch,
    setComposerPrefill, setItems, setMode, setModel, setRunning, setSessionId, setSessionInbox, setShowGate,
    setStreaming, setSurface, setTodo, setUsage, setWorkspace,
  } = prev;

  // Answering a LIVE prompt also settles its Inbox copy on the server, but the polled
  // `sessionInbox` keeps showing it as pending until the next poll: long enough for the docked
  // answer card to show the same request again right after it was answered (a tester saw a
  // Slack send "asked twice", 2026-07-12). So the local copy goes at once; the 4-second poll
  // brings back anything that really is still pending.
  const settle = (inboxKind: string | null, resolve: (items: Item[]) => Item[]) => {
    setItems(resolve);
    if (inboxKind) setSessionInbox((pending) => pending.filter((it) => it.kind !== inboxKind));
  };

  const approve = (decision: ApprovalDecision) => {
    settle("approval", (items) => resolveLastApproval(items, decision));
    sessionRef.current?.approve(decision);
  };
  const respondPlan = (approved: boolean, mode?: string, feedback?: string) => {
    settle("plan", (items) => resolveLastPlan(items, approved ? "approved" : "rejected"));
    sessionRef.current?.respondPlan(approved, mode, feedback);
    // The server switches the running engine to this mode as well.
    if (approved && mode) setMode(mode);
  };
  const respondDirectory = (granted: boolean, path?: string, writable?: boolean) => {
    settle("directory", (items) => resolveLastDirReq(items, granted ? "granted" : "denied"));
    sessionRef.current?.respondDirectory(granted, path, writable);
  };
  const answerQuestion = (answers: (string | null)[]) => {
    settle("question", (items) => resolveLastQuestion(items, JSON.stringify(answers)));
    sessionRef.current?.respondQuestion(answers);
  };
  const answerForm = (answers: Record<string, string>) => {
    settle(null, (items) => resolveLastForm(items, JSON.stringify(answers)));
    sessionRef.current?.respondForm(answers);
  };

  const prefillComposer = (text: string, attachments?: Attachment[]) =>
    setComposerPrefill((before) => ({ text, attachments, nonce: (before?.nonce ?? 0) + 1 }));

  const interrupt = () => sessionRef.current?.interrupt();
  const retry = () => {
    // Shown as running straight away: turn_start confirms it, and a refused retry still ends in turn_done.
    setRunning(true);
    sessionRef.current?.retry();
  };
  const changeMode = (next: string) => {
    setMode(next);
    sessionRef.current?.setMode(next);
  };
  const changeModel = (next: string) => {
    // The server won't switch models mid-turn, so the header mustn't pretend it did.
    if (running) return;
    TRACE && trace("chat", "model picked", { from: model, to: next, session: sessionId });
    setModel(next);
    sessionRef.current?.setModel(next);
  };

  const startNewSession = (forAgent?: string) => {
    const target = forAgent || agent;
    // Back to the conversation view from wherever we were.
    setSurface("session");
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);
    // "New session" under an assistant you were only browsing switches to it: expanding in
    // the header doesn't switch, this explicit action does.
    if (target !== agent) {
      setAgent(target);
      if (!gatesWorkspace(target)) setShowGate(false);
      else {
        // Never carry the previous assistant's folder over; it may be a scratch folder.
        // Clearing it also holds back the connection effect, so nothing can chat behind the
        // open gate. `agent`, `workspace` and `sessionId` still hold the PREVIOUS values here,
        // which is exactly where cancelling must return to.
        setWorkspace(null);
        setBranch(null);
        openGate({ agent, workspace, sessionId, branch });
      }
    }
    // Knowledge assistants start each conversation fresh: without a folder, the server makes a
    // NEW scratch folder for the new session id. Code keeps its repository.
    if (!gatesWorkspace(target)) setWorkspace(null);
    setSessionId(newId());
  };

  // UX-026: a five-second toast at the top right when a SCHEDULED run starts (never for Run
  // now, where the user is already watching). It comes over the app-wide /ws/events stream;
  // "View run" opens the run's live session.
  const [runToast, setRunToast] = useState<RunToastState | null>(null);
  useEffect(
    () =>
      connectEvents((event) => {
        if (event.type !== "automation_run_started") return;
        const run = (event.data ?? {}) as Record<string, string>;
        setRunToast({
          title: run.task_title || "Automation",
          sessionId: run.session_id || "",
          workspace: run.workspace || "",
          agent: run.agent || GENERAL_ASSISTANT,
          time: clock(new Date()),
        });
        // The Scheduled band's badge is out of date now.
        announceAutomationsChanged();
      }),
    [],
  );
  useEffect(() => {
    if (!runToast) return;
    const timer = window.setTimeout(() => setRunToast(null), TOAST_MS);
    return () => window.clearTimeout(timer);
  }, [runToast]);

  return {
    approve, respondPlan, respondDirectory, answerQuestion, answerForm, prefillComposer, interrupt, retry, changeMode,
    changeModel, startNewSession, runToast, setRunToast,
  };
}
