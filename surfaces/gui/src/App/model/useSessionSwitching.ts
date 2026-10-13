import { TRACE, trace } from "../../trace";
import { loadTranscript } from "../../sessionTranscript";
import { usageFromMessages, emptyUsage } from "../../usage";
import { rememberLastSession, resumeTargetForAgent, fallbackWorkspace, newId } from "../appHelpers";
import { getSessions, getRecentWorkspaces } from "../../api";
import type { GateReturn } from "../../gateCancel";
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

/** Opening, leaving and switching sessions, assistants and workspaces. */
export function useSessionSwitching(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> & ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend> & ReturnType<typeof useTurnActions>) {
  const { setSurface, leaveConversation, setAgent, gatesWorkspace, setShowGate, workspace,
      setWorkspace, setBranch, setSessionId, setItems, setUsage, agent, sessionId, sessions,
      projects, branch, setStreaming, setTodo, setRunning, needsWorkspace, openGate, gateReturn,
      setGateCreate, setProjects } = prev;
  const openSessionFromInbox = (sid: string, ws: string, ag: string) => selectSession(sid, ws, ag);
  // `br` is only passed when the caller is RESTORING a session it knows the branch of
  // (cancelling the folder gate). Everyone else means "a different session's folder",
  // where the old branch is not theirs to keep, so it defaults to none.
  const selectSession = async (id: string, ws: string, ag: string, br?: string | null) => {
    setSurface("session"); // selecting a conversation always returns to the conversation view
    leaveConversation("sidebar");
    TRACE && trace("chat", "select session", { to: id, agent: ag, workspace: ws || null });
    if (ag) setAgent(ag);
    if (!gatesWorkspace(ag)) setShowGate(false);
    if (ws && ws !== workspace) {
      setWorkspace(ws); // switch project to the session's folder
      setBranch(br ?? null);
    }
    setSessionId(id);
    try {
      const { messages, items: loaded } = await loadTranscript(id);
      setItems(loaded);
      setUsage(usageFromMessages(messages));
    } catch {
      setItems([]);
      setUsage(emptyUsage());
    }
  };
  const switchAgent = async (name: string) => {
    setSurface("session");
    if (name === agent) return;
    rememberLastSession(agent, sessionId, workspace);
    const knownSessions = sessions.length ? sessions : await getSessions().catch(() => []);
    const knownProjects = projects.length ? projects : await getRecentWorkspaces().catch(() => []);
    const target = resumeTargetForAgent(name, knownSessions);
    // Read before the setters below overwrite the render's values: if this switch ends
    // at the folder gate, this is what cancelling it has to put back. Switching to a
    // workbench with no folder to offer is the SAME navigation the New menu makes —
    // it just arrives from a persona header instead — so it owes the same undo.
    const before: GateReturn = { agent, workspace, sessionId, branch };

    setAgent(name);
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);

    // The live workspace is only a valid fallback for a gated persona if it came from
    // another gated persona — a knowledge persona's workspace is a scratch dir, and a
    // code-family session must never adopt one. (`agent` is still the previous persona here.)
    const inheritable = gatesWorkspace(agent) ? workspace : null;

    if (target) {
      // Code falls back to a recent folder; Cowork resumes its scratch (target.workspace) or
      // starts orphan ("" → server provisions). Chat has no workspace.
      const targetWorkspace = gatesWorkspace(name)
        ? target.workspace || fallbackWorkspace(inheritable, knownProjects)
        : needsWorkspace(name)
          ? target.workspace || ""
          : "";
      if (targetWorkspace && targetWorkspace !== workspace) {
        setWorkspace(targetWorkspace);
        setBranch(null);
      } else if (!targetWorkspace) {
        setWorkspace(null); // orphan cowork: clear so the next `ready` adopts a fresh scratch
      }
      if (!gatesWorkspace(name)) setShowGate(false);
      else if (targetWorkspace) setShowGate(false);
      else openGate(before);
      setSessionId(target.sessionId);
      try {
        const { messages, items: loaded } = await loadTranscript(target.sessionId);
        setItems(loaded);
        setUsage(usageFromMessages(messages));
      } catch {
        setItems([]);
        setUsage(emptyUsage());
      }
      return;
    }

    const id = newId();
    const fallback = gatesWorkspace(name) ? fallbackWorkspace(inheritable, knownProjects) : "";
    if (fallback && fallback !== workspace) {
      setWorkspace(fallback);
      setBranch(null);
    } else if (!fallback && needsWorkspace(name)) {
      setWorkspace(null); // orphan cowork: server provisions a fresh scratch on connect
    }
    setSessionId(id);
    rememberLastSession(name, id, fallback);
    if (!gatesWorkspace(name) || fallback) setShowGate(false);
    else openGate(before);
  };
  const chooseWorkspace = (path: string, b?: string | null) => {
    gateReturn.current = null; // committed; there is nothing to undo
    setWorkspace(path);
    setBranch(b ?? null);
    setShowGate(false);
    setGateCreate(false);
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setSessionId(newId());
    getRecentWorkspaces().then(setProjects).catch(() => {});
  };

  return { openSessionFromInbox, selectSession, switchAgent, chooseWorkspace };
}
