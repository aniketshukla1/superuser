import { deleteSession, renameSession, runAutomation, setSessionFlags } from "../../api";
import type { GateReturn } from "../../gateCancel";
import { emptyUsage } from "../../usage";
import { newId } from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useFollowScroll } from "./useFollowScroll";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";
import type { useSessionSocket } from "./useSessionSocket";
import type { useSessionSwitching } from "./useSessionSwitching";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useTurnActions } from "./useTurnActions";
import type { useVoiceAndSend } from "./useVoiceAndSend";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> &
  ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend> &
  ReturnType<typeof useTurnActions> & ReturnType<typeof useSessionSwitching>;

/** Projects and conversations: new ones, renaming, pinning, archiving, deleting, Run now. */
export function useConversationActions(prev: Deps) {
  const {
    activeRunRef, agent, branch, openGate, pendingPromptRef, refreshSessions, selectSession, sessionId, setAgent,
    setBranch, setGateCreate, setItems, setRunContext, setRunning, setSessionId, setShowGate, setStreaming, setSurface,
    setTodo, setUsage, setWorkspace, workspace,
  } = prev;

  /** Empty the transcript and everything tied to it, so nothing carries into the next session. */
  const clearTranscript = () => {
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);
  };

  // "New project" sits under a project-bound assistant's accordion: switch to that assistant,
  // start a fresh session with no folder, and open the gate in create mode. The gate's guard
  // (surface "session" and an assistant that needs a folder) then holds even if the open
  // session belonged to Chat or Cowork.
  const newProject = (forAgent?: string) => {
    const target = forAgent || agent;
    setSurface("session");
    clearTranscript();
    const returnTo: GateReturn = { agent, workspace, sessionId, branch };
    if (target !== agent) setAgent(target);
    setWorkspace(null);
    setBranch(null);
    setSessionId(newId());
    setGateCreate(true);
    openGate(returnTo);
  };

  // A project's own "+": a new session in a folder already known. Not newProject, which opens
  // the gate to ask for the folder when the answer is already here. It resets like
  // chooseWorkspace, so the transcript, usage and todo belong to the new session.
  const newSessionInProject = (path: string, forAgent?: string) => {
    const target = forAgent || agent;
    setSurface("session");
    if (target !== agent) setAgent(target);
    setWorkspace(path);
    setBranch(null);
    clearTranscript();
    setSessionId(newId());
    setShowGate(false);
    setGateCreate(false);
  };

  /** Leaves the open conversation for a new one, when `id` is the one being archived or deleted. */
  const leaveIfOpen = (id: string) => {
    if (id !== sessionId) return;
    clearTranscript();
    setSessionId(newId());
  };

  const renameConversation = async (id: string, title: string) => {
    if ((await renameSession(id, title)).ok) refreshSessions();
  };
  const togglePinned = async (id: string, pinned: boolean) => {
    await setSessionFlags(id, { pinned });
    refreshSessions();
  };
  const toggleArchived = async (id: string, archived: boolean) => {
    await setSessionFlags(id, { archived });
    refreshSessions();
    // Archiving the chat that is open moves it to Archived and starts a fresh one.
    if (archived) leaveIfOpen(id);
  };
  const deleteConversation = async (id: string) => {
    if (!(await deleteSession(id)).ok) return;
    refreshSessions();
    leaveIfOpen(id);
  };

  // "Run now": set up a manual run, open its session and send the task by itself, so the agent
  // works LIVE in the main view; the run is marked done in history after its first turn.
  const openRunSession = (runSession: string, folder: string, runAgent: string, task?: { id: string; title: string }) => {
    setRunContext(task ?? null);
    setSurface("session");
    setShowGate(false);
    selectSession(runSession, folder, runAgent);
  };
  const runTaskNow = async (taskId: string, title?: string) => {
    const run = await runAutomation(taskId);
    if (!run?.ok) return;
    pendingPromptRef.current = run.prompt;
    activeRunRef.current = { taskId, runId: run.run_id, sessionId: run.session_id };
    openRunSession(run.session_id, run.workspace, run.agent, { id: taskId, title: title || "" });
  };

  return { newProject, newSessionInProject, renameConversation, togglePinned, toggleArchived, deleteConversation, openRunSession, runTaskNow };
}
