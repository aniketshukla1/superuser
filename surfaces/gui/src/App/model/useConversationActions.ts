import { emptyUsage } from "../../usage";
import type { GateReturn } from "../../gateCancel";
import { newId } from "../appHelpers";
import { renameSession, setSessionFlags, deleteSession, runAutomation } from "../../api";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useSessionSocket } from "./useSessionSocket";
import type { useFollowScroll } from "./useFollowScroll";
import type { useVoiceAndSend } from "./useVoiceAndSend";
import type { useTurnActions } from "./useTurnActions";
import type { useSessionSwitching } from "./useSessionSwitching";

/** Projects and conversation actions: new, rename, pin, archive, delete, run now. */
export function useConversationActions(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> & ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend> & ReturnType<typeof useTurnActions> & ReturnType<typeof useSessionSwitching>) {
  const { agent, setSurface, setItems, setUsage, setStreaming, setTodo, setRunning, workspace,
      sessionId, branch, setAgent, setWorkspace, setBranch, setSessionId, setGateCreate, openGate,
      setShowGate, refreshSessions, setRunContext, selectSession, pendingPromptRef, activeRunRef } = prev;
  // "New project" lives under a project-scoped persona's accordion. Switch to that persona, start a
  // fresh session with no folder yet, and open the gate in create mode — so the gate's
  // surface==="session" && gatesWorkspace(agent) guard passes even if the active session was Chat/Cowork.
  const newProject = (forAgent?: string) => {
    const target = forAgent || agent;
    setSurface("session");
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);
    const before: GateReturn = { agent, workspace, sessionId, branch };
    if (target !== agent) setAgent(target);
    setWorkspace(null);
    setBranch(null);
    setSessionId(newId());
    setGateCreate(true);
    openGate(before);
  };
  // The per-project `+`: a new session in a folder we already know. Deliberately NOT
  // newProject — that opens the gate to ask which folder, and the answer is already in
  // hand. Mirrors chooseWorkspace's reset so the transcript, usage and todo belong to
  // the new session rather than bleeding in from the old one.
  const newSessionInProject = (path: string, forAgent?: string) => {
    const target = forAgent || agent;
    setSurface("session");
    if (target !== agent) setAgent(target);
    setWorkspace(path);
    setBranch(null);
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);
    setSessionId(newId());
    setShowGate(false);
    setGateCreate(false);
  };
  const renameConversation = async (id: string, title: string) => {
    const res = await renameSession(id, title);
    if (res.ok) refreshSessions();
  };
  const togglePinned = async (id: string, pinned: boolean) => {
    await setSessionFlags(id, { pinned });
    refreshSessions();
  };
  const toggleArchived = async (id: string, archived: boolean) => {
    await setSessionFlags(id, { archived });
    refreshSessions();
    // Archiving the open chat: leave it and start fresh (it moves to the Archived section).
    if (archived && id === sessionId) {
      setItems([]);
      setUsage(emptyUsage());
      setStreaming("");
      setTodo([]);
      setRunning(false);
      setSessionId(newId());
    }
  };
  const deleteConversation = async (id: string) => {
    const res = await deleteSession(id);
    if (!res.ok) return;
    refreshSessions();
    if (id === sessionId) {
      setItems([]);
      setUsage(emptyUsage());
      setStreaming("");
      setTodo([]);
      setRunning(false);
      setSessionId(newId());
    }
  };

  // "Run now": prepare a manual run, open its session, and auto-send the task so the agent
  // runs LIVE in the main view; finalize it in history once the first turn finishes.
  const openRunSession = (
    sessionId: string,
    ws: string,
    ag: string,
    task?: { id: string; title: string },
  ) => {
    setRunContext(task ?? null);
    setSurface("session");
    setShowGate(false);
    selectSession(sessionId, ws, ag);
  };
  const runTaskNow = async (taskId: string, title?: string) => {
    const r = await runAutomation(taskId);
    if (!r || !r.ok) return;
    pendingPromptRef.current = r.prompt;
    activeRunRef.current = { taskId, runId: r.run_id, sessionId: r.session_id };
    openRunSession(r.session_id, r.workspace, r.agent, { id: taskId, title: title || "" });
  };

  return { newProject, newSessionInProject, renameConversation, togglePinned, toggleArchived, deleteConversation, openRunSession, runTaskNow };
}
