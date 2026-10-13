import { getSettings, getSessions, getRecentWorkspaces, getHealth } from "../../api";
import { useState, useEffect } from "react";
import type { SessionInfo } from "../../types";
import { parse } from "../../route";
import { loadTranscript } from "../../sessionTranscript";
import { usageFromMessages, emptyUsage } from "../../usage";
import { isTauri } from "../../tauri";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";

/** Boot: health checks, onboarding, and resuming the last session or the gate. */
export function useBoot(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas>) {
  const { setModels, setModelLabels, setModelContextWindows, setModelReady, setSurfaces,
      setSessions, setAgent, setWorkspace, setBranch, setItems, setUsage, setSessionId,
      setShowGate, setProjects, gatesWorkspace, agent, openGate, setModel, setOnboarding,
      connected, showGate } = prev;
  const loadSettings = () =>
    getSettings()
      .then((s) => {
        setModels(s.models || []);
        setModelLabels(s.model_labels || {});
        setModelContextWindows(s.model_context_windows || {});
        setModelReady(s.model_ready);
        if (s.surfaces) setSurfaces(s.surfaces);
      })
      .catch(() => {});

  // initial: adopt the server's seed workspace if any, else force the gate.
  // Retry health for a while: the desktop shell starts its sidecar in parallel, so the
  // server may not answer for a second or two. Only fall back to the gate once it's truly up.
  const [booting, setBooting] = useState(true);
  // True once we've resumed a prior conversation on boot (drives the splash wording).
  const [resumedExisting, setResumedExisting] = useState(false);
  // Latched: keep the boot splash up until the restored session is actually CONNECTED (not just
  // until `booting` clears), so an early click can't land on a session that's still settling.
  const [uiReady, setUiReady] = useState(false);
  // What the splash is allowed to claim. The readout used to be a canned list of six
  // `[ ok ]` lines on a timer — it reported success for steps that had not happened,
  // including `binding 127.0.0.1` with nothing listening (UX-REPORT §2). These are the
  // real signals it now prints.
  const [healthOk, setHealthOk] = useState(false);
  // How saved credentials are kept, from health — only "locked" is shown (SecretsLockedNotice).
  const [secrets, setSecrets] = useState<string | undefined>(undefined);
  const [bootFailed, setBootFailed] = useState(false);
  const [bootKey, setBootKey] = useState(0); // bumped by Retry to re-run the health effect
  const retryBoot = () => {
    setBootFailed(false);
    setBootKey((k) => k + 1);
  };

  // On boot with no seeded workspace, reopen the last thing the user had — most recent
  // conversation (restores its folder + agent + transcript), else the most recent project
  // folder. Only a true first run (nothing to resume) falls through to the folder gate.
  const resumeLastOrGate = async () => {
    let loadedSessions: SessionInfo[] = [];
    try {
      loadedSessions = (await getSessions()).filter((s) => s.session_id && !s.session_id.startsWith("__"));
      setSessions(loadedSessions);
      const sess = loadedSessions;
      const ts = (s: SessionInfo) => Date.parse(s.updated_at || "") || Number(s.updated_at) || 0;
      // A conversation named in the URL outranks the most recent one: the whole
      // point of `#/chat/<id>` is that opening it takes you there, and resuming
      // "wherever you were last" over the top of a link the user just followed
      // would make the address decorative.
      const linked = parse(window.location.hash);
      const deepLinked = linked.surface === "session" && linked.id ? sess.find((s) => s.session_id === linked.id) : undefined;
      const last = deepLinked ?? [...sess].sort((a, b) => ts(b) - ts(a))[0];
      if (last) {
        setResumedExisting(true);
        if (last.agent) setAgent(last.agent);
        if (last.workspace) {
          setWorkspace(last.workspace);
          setBranch(null);
        }
        try {
          const { messages, items: loaded } = await loadTranscript(last.session_id);
          setItems(loaded);
          setUsage(usageFromMessages(messages));
        } catch {
          setItems([]);
          setUsage(emptyUsage());
        }
        setSessionId(last.session_id);
        setShowGate(false);
        return;
      }
    } catch {
      /* fall through */
    }
    try {
      const recents = await getRecentWorkspaces();
      setProjects(recents);
      // Only auto-adopt a recent folder for gated surfaces (Code). Cowork starts orphan.
      if (gatesWorkspace(agent)) {
        const ws = recents.find((w) => w.exists) || recents[0];
        if (ws) {
          setWorkspace(ws.path);
          setShowGate(false);
          return;
        }
      }
    } catch {
      /* fall through */
    }
    // Only Code forces a first-run folder gate — and this one has nothing behind it:
    // boot got here because there was no conversation to resume.
    if (gatesWorkspace(agent)) openGate(null);
    else setShowGate(false);
  };

  useEffect(() => {
    let cancelled = false;
    const attempt = (tries: number) => {
      getHealth()
        .then(async (h) => {
          if (cancelled) return;
          setHealthOk(true);
          setModel(h.model);
          setSecrets(h.secrets);
          // First-run setup wizard (desktop): show until the user completes/dismisses it.
          if (isTauri()) {
            getSettings()
              .then((s) => !cancelled && !s.onboarded && setOnboarding(true))
              .catch(() => {});
          }
          // Settle the active session BEFORE clearing `booting` (which unblocks the connection
          // effect). resumeLastOrGate is async — if we cleared `booting` first, the throwaway
          // initial sessionId would connect against an empty/stale workspace and the server
          // would provision a junk per-conversation scratch dir for it before resume could
          // flip to the real session. Cowork ignores default_workspace (a Code concept).
          if (h.default_workspace && gatesWorkspace(agent)) setWorkspace(h.default_workspace);
          else await resumeLastOrGate();
          // The mount-time loadSettings races the sidecar boot and swallows its failure —
          // on a cold start that left "Loading models…" stuck until the user visited
          // Settings (owner-hit 2026-07-23). Health just answered, so this one lands.
          loadSettings();
          if (!cancelled) setBooting(false);
        })
        .catch(() => {
          if (cancelled) return;
          if (tries <= 0) {
            // The old fall-through cleared `booting` and opened the folder gate, which
            // dropped the user into a workspace that looked entirely normal while
            // nothing was listening (UX-REPORT §3). Stay on the splash and say so.
            setBootFailed(true);
          } else {
            setTimeout(() => attempt(tries - 1), 500);
          }
        });
    };
    setBooting(true);
    setHealthOk(false);
    attempt(40); // ~20s of 500ms retries
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bootKey]);

  // Reveal the UI once boot has settled AND the restored session is connected (or we're showing
  // the folder gate). Latched, so later reconnects never flash the splash again.
  useEffect(() => {
    if (uiReady || booting) return;
    if (connected || showGate) setUiReady(true);
  }, [uiReady, booting, connected, showGate]);
  // Safety net: if the restored session never reports connected (backend slow/unreachable), reveal
  // the UI anyway. Boot already passed the health check, so a live connect is sub-second; this only
  // bites in the failure case, so keep it short.
  useEffect(() => {
    if (uiReady || booting) return;
    const t = setTimeout(() => setUiReady(true), 1500);
    return () => clearTimeout(t);
  }, [uiReady, booting]);

  return { loadSettings, booting, resumedExisting, uiReady, healthOk, secrets, setSecrets, bootFailed, retryBoot };
}
