import { useState, useEffect, useRef, useCallback } from "react";
import type { Attachment } from "../../types";
import { type Persona, getPersonas, PERSONAS_CHANGED, type InboxItem, setUnattended, announceInboxUnlock, resolveInboxItem, getInbox, Session, getSessions, getRecentWorkspaces } from "../../api";
import { reportIfFailed } from "../../notify";
import { stillOpen } from "../../sessionInbox";
import { needsWorkspaceFallback, gatesWorkspaceFallback } from "../appHelpers";
import { isProjectScoped, showsTerminal, agentSharesShell, showsBrowser } from "../../personaScope";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";

/** The persona roster, per-session inboxes, unattended mode and session refs. */
export function usePersonas(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout>) {
  const { sessionId, openSettings, setSessions, setProjects } = prev;
  const [onboarding, setOnboarding] = useState(false);
  // A pending composer prefill (text + attachments) pushed from the session start panel.
  const [composerPrefill, setComposerPrefill] = useState<{ text: string; attachments?: Attachment[]; nonce: number }>();

  // Persona metadata drives workspace behavior by FAMILY, not by hardcoded id (so a DevOps/SecOps
  // code-family persona gates a folder like Code, and a knowledge persona starts orphan like Cowork).
  const [personas, setPersonas] = useState<Persona[] | null>(null);
  useEffect(() => {
    let stopped = false;
    // An EMPTY roster is impossible. The built-ins are compiled into the binary, so
    // the server can never honestly answer with none — and `listOf` turns every
    // failure, including the 404 you get while the sidecar is still binding, into
    // `[]`. Storing that is how a boot race becomes permanent: `personas` is then a
    // truthy empty array, so nothing falls back, and everything derived from it
    // degrades quietly. The header shows a raw id instead of a name, `hasShell` is
    // false so a Coder loses its Terminal and Browser buttons, `gatesWorkspace`
    // drops to guessing from the id. Nothing looks broken; things are just absent.
    //
    // Health already retries the sidecar's boot. This did not, and it is fetched at
    // mount, which is exactly when the sidecar may not be listening yet.
    const load = async (attempt = 0) => {
      const rows = await getPersonas().catch(() => [] as Persona[]);
      if (stopped) return;
      if (rows.length) {
        setPersonas(rows);
        return;
      }
      if (attempt < 5) setTimeout(() => load(attempt + 1), 250 * (attempt + 1));
    };
    const reload = () => void load();
    load();
    // Re-read when the roster changes. Fetched once on mount, this list went stale the
    // moment someone created an assistant — and the create flow drops you straight into a
    // session with it, so `personaOf` missed and the top bar showed the raw slug
    // ("modal-check") while the sidebar, which does re-read, showed "Modal Check".
    // Everything else keyed off this list — the family, and therefore whether the session
    // gates a folder — was guessing from the id at the same time.
    window.addEventListener(PERSONAS_CHANGED, reload);
    return () => {
      stopped = true;
      window.removeEventListener(PERSONAS_CHANGED, reload);
    };
  }, []);
  const personaOf = (a: string) => personas?.find((p) => p.id === a);

  // Pending Inbox items for the ACTIVE session — surfaced inline above the composer so an
  // unattended session's blocking question/approval can be answered in context (resolving the
  // same item the Inbox shows; first responder wins).
  const [sessionInbox, setSessionInbox] = useState<InboxItem[]>([]);
  // Whether the active session is Unattended — when true, the agent's prompts route to the Inbox,
  // so we suppress the inline live cards (the Inbox / answer-in-context path shows them instead).
  // A ref too, because the WS event handler closes over stale state.
  const [unattended, setUnattendedState] = useState(false);
  const unattendedRef = useRef(false);
  const markUnattended = useCallback((on: boolean) => {
    unattendedRef.current = on;
    setUnattendedState(on);
  }, []);
  // The Mode menu's "Send approvals to Inbox" toggle (§22 — the old InboxControl, folded in).
  const toggleUnattended = async (on: boolean) => {
    await setUnattended(sessionId, on);
    markUnattended(on);
    // First Unattended enable = Inbox machinery engaged → the account row's chip unlocks (§26).
    if (on) announceInboxUnlock();
  };
  const resolveSessionInbox = async (id: string, resolution: string) => {
    // Reported as "clicking the options does nothing", and it was two bugs: the
    // server handed back an item that was already resolved, and this swallowed the
    // failed resolve without a word. The second is why the first was invisible —
    // a click that silently achieves nothing is indistinguishable from a dead button.
    const ok = await reportIfFailed(
      resolveInboxItem(id, resolution),
      "Couldn't send that answer.",
    );
    getInbox(sessionId, "pending")
      .then((items) => setSessionInbox(stillOpen(items)))
      .catch(() => setSessionInbox([]));
    if (ok) refreshSessions(); // attention badge should drop right away
  };
  // Shows a working-area chip / project grouping. Persona's needs_workspace; fallback before load.
  const needsWorkspace = (a: string) => personaOf(a)?.needs_workspace ?? needsWorkspaceFallback(a);
  // MUST pick a folder before starting — project-scoped personas (git-bound Code, project-bound
  // Ops). Scratch/deliverable personas start orphan: the server auto-provisions a per-conversation
  // scratch dir and reports it in the `ready` event.
  const gatesWorkspace = (a: string) => {
    const p = personaOf(a);
    return p ? isProjectScoped(p) : gatesWorkspaceFallback(a);
  };
  // Has a shell, so the terminal and the browser have something behind them.
  //
  // The guard used to be `agent !== "chat"` — back when "chat" was the one persona
  // without a shell. There is no "chat" persona now, so that read as `true` for
  // everyone and every knowledge assistant grew a Terminal button opening onto
  // nothing. The comment beside it had said "only for personas that have a shell"
  // the whole time; this is that sentence, as code.
  // See `personaScope` for why these three are not the same question. Browser is a
  // capability button: without `web` the panel it opens can never be filled. The
  // terminal is a user surface and is always offered — `shell` decides only whether
  // the AGENT types into it too, which is a difference the label has to carry.
  const hasShell = (a: string) => showsTerminal(personaOf(a));
  const sharesShell = (a: string) => agentSharesShell(personaOf(a));
  const hasBrowser = (a: string) => showsBrowser(personaOf(a));

  // The desktop tray's "Settings" item dispatches this on the window.
  useEffect(() => {
    const open = () => openSettings("appearance");
    window.addEventListener("assistant:open-settings", open);
    return () => window.removeEventListener("assistant:open-settings", open);
  }, []);

  // "Run setup again" (from Settings) re-opens the wizard.
  useEffect(() => {
    const open = () => {
      setOnboarding(true);
    };
    window.addEventListener("assistant:open-onboarding", open);
    return () => window.removeEventListener("assistant:open-onboarding", open);
  }, []);

  const sessionRef = useRef<Session | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  // A prompt to auto-send once the next session connects (used by "Run now").
  const pendingPromptRef = useRef<string | null>(null);
  // The in-flight manual run to finalize after its first turn ({taskId, runId, sessionId}).
  const activeRunRef = useRef<{ taskId: string; runId: string; sessionId: string } | null>(null);

  // Fetch ALL sessions + known projects so the sidebar can group them.
  const refreshSessions = useCallback(() => {
    getSessions().then(setSessions).catch(() => setSessions([]));
    getRecentWorkspaces().then(setProjects).catch(() => setProjects([]));
  }, []);

  return { onboarding, setOnboarding, composerPrefill, setComposerPrefill, personas, personaOf, sessionInbox, setSessionInbox, unattended, unattendedRef, markUnattended, toggleUnattended, resolveSessionInbox, needsWorkspace, gatesWorkspace, hasShell, sharesShell, hasBrowser, sessionRef, scrollRef, pendingPromptRef, activeRunRef, refreshSessions };
}
