import { getRecentWorkspaces, getSessions } from "../../api";
import type { GateReturn } from "../../gateCancel";
import { loadTranscript } from "../../sessionTranscript";
import { TRACE, trace } from "../../trace";
import { emptyUsage, usageFromMessages } from "../../usage";
import { fallbackWorkspace, newId, rememberLastSession, resumeTargetForAgent } from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useFollowScroll } from "./useFollowScroll";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";
import type { useSessionSocket } from "./useSessionSocket";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useTurnActions } from "./useTurnActions";
import type { useVoiceAndSend } from "./useVoiceAndSend";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> &
  ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend> &
  ReturnType<typeof useTurnActions>;

/** Opening, leaving and switching sessions, assistants and folders. */
export function useSessionSwitching(prev: Deps) {
  const {
    agent, branch, gateReturn, gatesWorkspace, leaveConversation, needsWorkspace, openGate, projects, sessionId,
    sessions, setAgent, setBranch, setGateCreate, setItems, setProjects, setRunning, setSessionId, setShowGate,
    setStreaming, setSurface, setTodo, setUsage, setWorkspace, workspace,
  } = prev;

  /** Show session `id`'s saved transcript and usage; empty when it can't be read. */
  const showTranscript = async (id: string) => {
    try {
      const { messages, items } = await loadTranscript(id);
      setItems(items);
      setUsage(usageFromMessages(messages));
    } catch {
      setItems([]);
      setUsage(emptyUsage());
    }
  };

  /** Move to another folder, dropping the branch, which belonged to the old one. */
  const moveTo = (folder: string, keepBranch: string | null = null) => {
    setWorkspace(folder);
    setBranch(keepBranch);
  };

  // `br` comes only from a caller RESTORING a session whose branch it knows (cancelling the
  // folder gate). Everyone else means some other session's folder, where the old branch isn't
  // theirs to keep, so it starts as none.
  const selectSession = async (id: string, ws: string, ag: string, br?: string | null) => {
    // Choosing a conversation always comes back to the conversation view.
    setSurface("session");
    leaveConversation("sidebar");
    TRACE && trace("chat", "select session", { to: id, agent: ag, workspace: ws || null });
    if (ag) setAgent(ag);
    if (!gatesWorkspace(ag)) setShowGate(false);
    if (ws && ws !== workspace) moveTo(ws, br ?? null);
    setSessionId(id);
    await showTranscript(id);
  };

  const openSessionFromInbox = (id: string, ws: string, ag: string) => selectSession(id, ws, ag);

  const switchAgent = async (name: string) => {
    setSurface("session");
    if (name === agent) return;
    rememberLastSession(agent, sessionId, workspace);
    const knownSessions = sessions.length > 0 ? sessions : await getSessions().catch(() => []);
    const knownProjects = projects.length > 0 ? projects : await getRecentWorkspaces().catch(() => []);
    const resume = resumeTargetForAgent(name, knownSessions);
    // Read before the setters below replace this render's values: if the switch ends at the
    // folder gate, cancelling must put these back. Switching to a workbench with no folder to
    // offer is the same move the New menu makes, only from an assistant's header, so it owes
    // the same undo.
    const returnTo: GateReturn = { agent, workspace, sessionId, branch };

    setAgent(name);
    setItems([]);
    setUsage(emptyUsage());
    setStreaming("");
    setTodo([]);
    setRunning(false);

    // The current folder may only be passed on to an assistant that needs one if it came from
    // another such assistant: a knowledge assistant's folder is a scratch folder, and a code
    // session must never take one. (`agent` is still the previous assistant here.)
    const passable = gatesWorkspace(agent) ? workspace : null;
    const needsGate = gatesWorkspace(name);

    if (resume) {
      // Code falls back to a recent folder; Cowork picks up its scratch folder or starts with
      // none ("" lets the server make one); Chat has no folder at all.
      let folder = "";
      if (needsGate) folder = resume.workspace || fallbackWorkspace(passable, knownProjects);
      else if (needsWorkspace(name)) folder = resume.workspace || "";
      // No folder for Cowork means the next `ready` brings a fresh scratch folder.
      if (!folder) setWorkspace(null);
      else if (folder !== workspace) moveTo(folder);
      if (!needsGate || folder) setShowGate(false);
      else openGate(returnTo);
      setSessionId(resume.sessionId);
      await showTranscript(resume.sessionId);
      return;
    }

    const freshId = newId();
    const folder = needsGate ? fallbackWorkspace(passable, knownProjects) : "";
    if (folder && folder !== workspace) moveTo(folder);
    // Cowork without a folder: the server makes a fresh scratch folder on connect.
    else if (!folder && needsWorkspace(name)) setWorkspace(null);
    setSessionId(freshId);
    rememberLastSession(name, freshId, folder);
    if (!needsGate || folder) setShowGate(false);
    else openGate(returnTo);
  };

  const chooseWorkspace = (path: string, b?: string | null) => {
    // The choice is made; there's nothing to undo any more.
    gateReturn.current = null;
    moveTo(path, b ?? null);
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
