import { useEffect, useState } from "react";
import { PERSONAS_CHANGED } from "../../api";
import { introForThisLaunch, SPLASH_SETTLE_MS } from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot>;

const SESSIONS_POLL_MS = 5000;

/** The splash from start to finish, and the settings the app loads when it starts. */
export function useSplashAndSettings(prev: Deps) {
  const { booting, loadSettings, openSettings, refreshSessions, surface, uiReady } = prev;

  // The launch film (components/LaunchFilm) and how it leaves. The app renders as soon as it is
  // ready, with the film laid over it until it has played or been skipped, then fading away.
  // Neither waits for the other: a slow runtime holds the film's last frame, and a fast one
  // waits for the film. With reduced motion there is no film and no fade.
  const appReady = !booting && uiReady;
  const [intro] = useState(introForThisLaunch);
  const [reelDone, setReelDone] = useState(intro === "off");
  const [introSkipped, setIntroSkipped] = useState(false);
  const [splashLeaving, setSplashLeaving] = useState(false);
  const [splashGone, setSplashGone] = useState(false);
  useEffect(() => {
    const filmOver = reelDone || introSkipped;
    if (!appReady || splashLeaving || !filmOver) return;
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      setSplashGone(true);
      return;
    }
    // A skip asked for the app now, so it gets no pause.
    const timer = setTimeout(() => setSplashLeaving(true), introSkipped ? 0 : SPLASH_SETTLE_MS);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appReady, reelDone, introSkipped]);

  /** Settings ▸ Models, from the composer's "No model connected" chip. */
  const openModelSetup = () => openSettings("models");

  // Coming back from Settings: the composer picks up any change to models or surfaces (the old
  // settings dialog did this when it closed).
  useEffect(() => {
    if (surface !== "settings") loadSettings();
  }, [surface]);

  // At start: the sessions, the models that can be chosen and which session surfaces show.
  useEffect(() => {
    refreshSessions();
    loadSettings();
  }, [refreshSessions]);

  // The session list is polled, so attention and liveness badges stay current and sessions
  // made elsewhere (unattended work, messages, automations) appear without a refresh. An
  // assistant switched off can archive its sessions on the server (§18), so that event also
  // refreshes at once rather than at the next poll.
  useEffect(() => {
    const poll = setInterval(refreshSessions, SESSIONS_POLL_MS);
    const onPersonas = () => refreshSessions();
    window.addEventListener(PERSONAS_CHANGED, onPersonas);
    return () => {
      clearInterval(poll);
      window.removeEventListener(PERSONAS_CHANGED, onPersonas);
    };
  }, [refreshSessions]);

  return { appReady, intro, reelDone, setReelDone, introSkipped, setIntroSkipped, splashLeaving, splashGone, setSplashGone, openModelSetup };
}
