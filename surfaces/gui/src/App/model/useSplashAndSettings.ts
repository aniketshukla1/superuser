import { useState, useEffect } from "react";
import { introForThisLaunch, SPLASH_SETTLE_MS } from "../appHelpers";
import { PERSONAS_CHANGED } from "../../api";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";

/** The splash's lifecycle and the settings the app loads at start. */
export function useSplashAndSettings(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot>) {
  const { booting, uiReady, openSettings, surface, loadSettings, refreshSessions } = prev;
  // The launch film and its exit (components/LaunchFilm). The app renders the moment it is
  // ready; the reel stays laid over it until it has played or been skipped, then fades
  // off. Neither waits on the other: a slow runtime holds the reel's last frame, and a
  // fast one waits for the reel. Reduced motion plays no reel and no exit.
  const appReady = !booting && uiReady;
  const [intro] = useState(introForThisLaunch);
  const [reelDone, setReelDone] = useState(intro === "off");
  const [introSkipped, setIntroSkipped] = useState(false);
  const [splashLeaving, setSplashLeaving] = useState(false);
  const [splashGone, setSplashGone] = useState(false);
  useEffect(() => {
    if (!appReady || splashLeaving || !(reelDone || introSkipped)) return;
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      setSplashGone(true);
      return;
    }
    // No beat after a skip: that asked for the app, now.
    const t = setTimeout(() => setSplashLeaving(true), introSkipped ? 0 : SPLASH_SETTLE_MS);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appReady, reelDone, introSkipped]);


  // Open Settings → Configure Models (from the composer's "No model connected" chip).
  const openModelSetup = () => openSettings("models");

  // Leaving the Settings page: pick up any model/surface changes for the composer (the modal used to
  // do this on close).
  useEffect(() => {
    if (surface !== "settings") loadSettings();
  }, [surface]);

  useEffect(() => {
    refreshSessions();
    loadSettings(); // selectable models + which session surfaces are visible
  }, [refreshSessions]);

  // Poll the session list so the attention/liveness badges stay live and sessions created
  // out-of-band (unattended work, messaging, automations) appear without a manual refresh.
  useEffect(() => {
    const t = setInterval(refreshSessions, 5000);
    return () => clearInterval(t);
  }, [refreshSessions]);

  // Persona toggles can archive sessions server-side (disable-archives, §18): refetch on the
  // personas-changed event so the sidebar section disappears immediately, not on the next poll.
  useEffect(() => {
    const onPersonas = () => refreshSessions();
    window.addEventListener(PERSONAS_CHANGED, onPersonas);
    return () => window.removeEventListener(PERSONAS_CHANGED, onPersonas);
  }, [refreshSessions]);

  return { appReady, intro, reelDone, setReelDone, introSkipped, setIntroSkipped, splashLeaving, splashGone, setSplashGone, openModelSetup };
}
