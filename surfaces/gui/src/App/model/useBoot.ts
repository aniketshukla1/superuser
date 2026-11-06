import { useEffect, useState } from "react";
import { getHealth, getRecentWorkspaces, getSessions, getSettings, type Health } from "../../api";
import { parse } from "../../route";
import { loadTranscript } from "../../sessionTranscript";
import { isTauri } from "../../tauri";
import type { SessionInfo } from "../../types";
import { emptyUsage, usageFromMessages } from "../../usage";
import { sessionTs } from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";

type BootDeps = ReturnType<typeof useAppState> &
  ReturnType<typeof useNavigationState> &
  ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas>;

const HEALTH_TRIES = 40; // about 20 seconds at one try every half second
const HEALTH_GAP_MS = 500;
const REVEAL_ANYWAY_MS = 1500;

/** The conversation a launch should reopen. One named in the URL wins over the most recent:
 *  `#/chat/<id>` exists so that following it lands there, and reopening "wherever you were"
 *  over a link just followed would make the address decorative. */
function sessionToReopen(sessions: SessionInfo[]): SessionInfo | undefined {
  const route = parse(window.location.hash);
  const linked = route.surface === "session" && route.id ? sessions.find((s) => s.session_id === route.id) : undefined;
  if (linked) return linked;
  let newest: SessionInfo | undefined;
  for (const s of sessions) if (!newest || sessionTs(s) > sessionTs(newest)) newest = s;
  return newest;
}

/** Starting up: wait for the server, show the first-run wizard if needed, and reopen the last
 *  conversation or put up the folder gate. */
export function useBoot(prev: BootDeps) {
  const {
    agent, connected, showGate, gatesWorkspace, openGate, setAgent, setBranch, setItems, setModel,
    setModelContextWindows, setModelLabels, setModelReady, setModels, setOnboarding, setProjects,
    setSessionId, setSessions, setShowGate, setSurfaces, setUsage, setWorkspace,
  } = prev;

  const loadSettings = () =>
    getSettings()
      .then((settings) => {
        setModels(settings.models || []);
        setModelLabels(settings.model_labels || {});
        setModelContextWindows(settings.model_context_windows || {});
        setModelReady(settings.model_ready);
        if (settings.surfaces) setSurfaces(settings.surfaces);
      })
      .catch(() => {});

  // The desktop shell starts the sidecar alongside the window, so the server may take a second
  // or two to answer; health is retried for a while, and the gate only appears once it is up.
  const [booting, setBooting] = useState(true);
  const [bootFailed, setBootFailed] = useState(false);
  // Retry bumps this to run the health effect again.
  const [bootKey, setBootKey] = useState(0);
  // What the splash may truthfully say. It used to print six canned "[ ok ]" lines on a timer,
  // claiming steps that hadn't happened, "binding 127.0.0.1" with nothing listening among them
  // (UX-REPORT §2); now it reports these real signals.
  const [healthOk, setHealthOk] = useState(false);
  // True once boot reopened an earlier conversation; the splash words itself on it.
  const [resumedExisting, setResumedExisting] = useState(false);
  // How saved credentials are stored, from health. Only "locked" is shown (SecretsLockedNotice).
  const [secrets, setSecrets] = useState<string | undefined>(undefined);
  // Holds the splash until the reopened session is actually CONNECTED, not merely until
  // `booting` clears, so an early click can't land on a session still settling. Once set, it
  // stays set.
  const [uiReady, setUiReady] = useState(false);

  const retryBoot = () => {
    setBootFailed(false);
    setBootKey((key) => key + 1);
  };

  const reopen = async (session: SessionInfo) => {
    setResumedExisting(true);
    if (session.agent) setAgent(session.agent);
    if (session.workspace) {
      setWorkspace(session.workspace);
      setBranch(null);
    }
    try {
      const { messages, items } = await loadTranscript(session.session_id);
      setItems(items);
      setUsage(usageFromMessages(messages));
    } catch {
      setItems([]);
      setUsage(emptyUsage());
    }
    setSessionId(session.session_id);
    setShowGate(false);
  };

  // With no folder seeded by the server, reopen what the user had last: the latest
  // conversation (its folder, assistant and transcript), or else the latest project folder.
  // Only a true first run, with nothing to reopen, reaches the folder gate.
  const reopenOrGate = async () => {
    try {
      const sessions = (await getSessions()).filter((s) => s.session_id && !s.session_id.startsWith("__"));
      setSessions(sessions);
      const target = sessionToReopen(sessions);
      if (target) {
        await reopen(target);
        return;
      }
    } catch {
      // Go on to the folders.
    }
    try {
      const recents = await getRecentWorkspaces();
      setProjects(recents);
      // Only a surface that needs a folder (Code) takes a recent one by itself; Cowork starts without.
      const folder = gatesWorkspace(agent) ? recents.find((w) => w.exists) || recents[0] : undefined;
      if (folder) {
        setWorkspace(folder.path);
        setShowGate(false);
        return;
      }
    } catch {
      // Go on to the gate.
    }
    // Only Code insists on a folder at first run, and there is nothing behind this gate: boot is
    // here because there was no conversation to reopen.
    if (gatesWorkspace(agent)) openGate(null);
    else setShowGate(false);
  };

  useEffect(() => {
    let cancelled = false;

    const onHealthy = async (health: Health) => {
      setHealthOk(true);
      setModel(health.model);
      setSecrets(health.secrets);
      // The desktop's first-run wizard, until the user finishes or dismisses it.
      if (isTauri()) {
        getSettings()
          .then((settings) => !cancelled && !settings.onboarded && setOnboarding(true))
          .catch(() => {});
      }
      // Decide the active session BEFORE clearing `booting`, which releases the connection
      // effect: reopening is async, and clearing first would connect the throwaway starting
      // session against an empty or stale folder, and the server would make a pointless scratch
      // folder for it before the real session took over. Cowork ignores default_workspace,
      // which is a Code idea.
      if (health.default_workspace && gatesWorkspace(agent)) setWorkspace(health.default_workspace);
      else await reopenOrGate();
      // The settings load at mount races the sidecar's start and drops its failure; on a cold
      // start that left "Loading models…" up until someone opened Settings (owner, 2026-07-23).
      // Health has just answered, so this one gets through.
      loadSettings();
      if (!cancelled) setBooting(false);
    };

    const tryHealth = (left: number) => {
      getHealth()
        .then((health) => (cancelled ? undefined : onHealthy(health)))
        .catch(() => {
          if (cancelled) return;
          // Out of tries: stay on the splash and say so. Falling through to the gate used to
          // drop the user into a workspace that looked normal with nothing listening (UX-REPORT §3).
          if (left <= 0) setBootFailed(true);
          else setTimeout(() => tryHealth(left - 1), HEALTH_GAP_MS);
        });
    };

    setBooting(true);
    setHealthOk(false);
    tryHealth(HEALTH_TRIES);
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bootKey]);

  // Show the app when boot has settled and the reopened session is connected, or the folder
  // gate is up.
  useEffect(() => {
    if (!uiReady && !booting && (connected || showGate)) setUiReady(true);
  }, [uiReady, booting, connected, showGate]);

  // If the reopened session never reports connected (a slow or unreachable backend), show the
  // app anyway. Health already passed, so a working connect takes well under a second; this
  // only matters when something is wrong, so it waits briefly.
  useEffect(() => {
    if (uiReady || booting) return;
    const timer = setTimeout(() => setUiReady(true), REVEAL_ANYWAY_MS);
    return () => clearTimeout(timer);
  }, [uiReady, booting]);

  return { loadSettings, booting, resumedExisting, uiReady, healthOk, secrets, setSecrets, bootFailed, retryBoot };
}
