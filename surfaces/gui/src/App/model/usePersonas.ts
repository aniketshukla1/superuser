import { useCallback, useEffect, useRef, useState } from "react";
import {
  announceInboxUnlock, getInbox, getPersonas, getRecentWorkspaces, getSessions, PERSONAS_CHANGED, resolveInboxItem,
  setUnattended, type InboxItem, type Persona, type Session,
} from "../../api";
import { reportIfFailed } from "../../notify";
import { agentSharesShell, isProjectScoped, showsBrowser, showsTerminal } from "../../personaScope";
import { stillOpen } from "../../sessionInbox";
import type { Attachment } from "../../types";
import { gatesWorkspaceFallback, needsWorkspaceFallback } from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout>;

const ROSTER_RETRIES = 5;

/** Calls `handler` whenever `event` is sent on the window, for the life of the component. */
function useWindowEvent(event: string, handler: () => void) {
  useEffect(() => {
    window.addEventListener(event, handler);
    return () => window.removeEventListener(event, handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}

/** The assistants, the open session's Inbox items, Unattended mode, and the refs the session
 *  machinery shares. */
export function usePersonas(prev: Deps) {
  const { openSettings, sessionId, setProjects, setSessions } = prev;
  const [onboarding, setOnboarding] = useState(false);
  /** Text and attachments the start panel wants put into the composer. */
  const [composerPrefill, setComposerPrefill] = useState<{ text: string; attachments?: Attachment[]; nonce: number }>();

  // An assistant's FAMILY decides how folders behave, not a list of ids: a code-family
  // assistant (DevOps, SecOps) asks for a folder as Code does, a knowledge one starts without.
  const [personas, setPersonas] = useState<Persona[] | null>(null);
  useEffect(() => {
    let stopped = false;
    // An EMPTY list can't be true: the built-in assistants are compiled into the server, so it
    // never really has none. But `listOf` turns every failure into `[]`, the 404 from a sidecar
    // still starting up included, and storing that makes a start-up race permanent: an empty
    // list is truthy, nothing falls back, and everything read from it quietly goes missing
    // (raw ids for names, no Terminal or Browser for a Coder, folder rules guessed from ids).
    // So an empty answer is retried, the way health is, since this runs at mount: the very
    // moment the sidecar may not be listening.
    const read = async (attempt: number) => {
      const roster = await getPersonas().catch((): Persona[] => []);
      if (stopped) return;
      if (roster.length > 0) setPersonas(roster);
      else if (attempt < ROSTER_RETRIES) setTimeout(() => read(attempt + 1), 250 * (attempt + 1));
    };
    // Read again whenever the assistants change. Read only once, the list went stale as soon
    // as someone created an assistant, and creating one opens a session with it straight away:
    // the top bar showed its raw slug ("modal-check") while the sidebar said "Modal Check".
    const reread = () => void read(0);
    void read(0);
    window.addEventListener(PERSONAS_CHANGED, reread);
    return () => {
      stopped = true;
      window.removeEventListener(PERSONAS_CHANGED, reread);
    };
  }, []);
  const personaOf = (id: string) => personas?.find((p) => p.id === id);

  /** The ACTIVE session's pending Inbox items, shown above the composer so an unattended
   *  session's blocking question or approval can be answered where it arose. It settles the
   *  same item the Inbox shows; whoever answers first wins. */
  const [sessionInbox, setSessionInbox] = useState<InboxItem[]>([]);
  // Whether the active session is Unattended. Then the agent's prompts go to the Inbox, so the
  // live cards stay hidden here. Kept in a ref as well, for socket handlers holding old state.
  const [unattended, setUnattendedState] = useState(false);
  const unattendedRef = useRef(false);
  const markUnattended = useCallback((on: boolean) => {
    unattendedRef.current = on;
    setUnattendedState(on);
  }, []);

  const sessionRef = useRef<Session | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  /** A prompt to send by itself when the next session connects ("Run now"). */
  const pendingPromptRef = useRef<string | null>(null);
  /** The manual run to finish after its first turn. */
  const activeRunRef = useRef<{ taskId: string; runId: string; sessionId: string } | null>(null);

  /** Every session and known project, so the sidebar can group them. */
  const refreshSessions = useCallback(() => {
    getSessions().then(setSessions).catch(() => setSessions([]));
    getRecentWorkspaces().then(setProjects).catch(() => setProjects([]));
  }, []);

  /** The Mode menu's "Send approvals to Inbox" switch (§22; it was the separate InboxControl). */
  const toggleUnattended = async (on: boolean) => {
    await setUnattended(sessionId, on);
    markUnattended(on);
    // The first time a session goes Unattended the Inbox is in use, and the account row's chip appears (§26).
    if (on) announceInboxUnlock();
  };

  const resolveSessionInbox = async (id: string, resolution: string) => {
    // "Clicking the options does nothing" was two bugs: the server handed back an item already
    // answered, and a failed answer here was swallowed in silence, which is what hid the first.
    // A click that quietly achieves nothing looks exactly like a dead button.
    const sent = await reportIfFailed(resolveInboxItem(id, resolution), "Couldn't send that answer.");
    getInbox(sessionId, "pending")
      .then((pending) => setSessionInbox(stillOpen(pending)))
      .catch(() => setSessionInbox([]));
    // The attention badge should drop at once.
    if (sent) refreshSessions();
  };

  /** Whether the assistant shows a working-folder chip and groups by project. */
  const needsWorkspace = (id: string) => personaOf(id)?.needs_workspace ?? needsWorkspaceFallback(id);
  /** Whether a folder MUST be chosen before starting: assistants tied to a project (Code to a
   *  repo, Ops to a project). The rest start without one; the server makes a scratch folder per
   *  conversation and reports it in the `ready` event. */
  const gatesWorkspace = (id: string) => {
    const persona = personaOf(id);
    return persona ? isProjectScoped(persona) : gatesWorkspaceFallback(id);
  };
  // Three different questions; `personaScope` explains each. The browser button needs `web`,
  // since without it nothing can fill that panel. The terminal is the user's and always
  // offered; `shell` only decides whether the AGENT types into it too, which its label has to
  // say. (The old guard was `agent !== "chat"`, from when "chat" was the one assistant without
  // a shell; with no "chat" left it was true for all, and knowledge assistants got a Terminal
  // button opening onto nothing.)
  const hasShell = (id: string) => showsTerminal(personaOf(id));
  const sharesShell = (id: string) => agentSharesShell(personaOf(id));
  const hasBrowser = (id: string) => showsBrowser(personaOf(id));

  // The tray's "Settings" item sends this on the window.
  useWindowEvent("assistant:open-settings", () => openSettings("appearance"));
  // "Run setup again" in Settings opens the wizard again.
  useWindowEvent("assistant:open-onboarding", () => setOnboarding(true));

  return {
    onboarding, setOnboarding, composerPrefill, setComposerPrefill, personas, personaOf, sessionInbox, setSessionInbox,
    unattended, unattendedRef, markUnattended, toggleUnattended, resolveSessionInbox, needsWorkspace, gatesWorkspace,
    hasShell, sharesShell, hasBrowser, sessionRef, scrollRef, pendingPromptRef, activeRunRef, refreshSessions,
  };
}
