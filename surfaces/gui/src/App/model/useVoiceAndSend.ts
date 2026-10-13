import { useState, useEffect, useRef } from "react";
import { type VoiceState, isTauri, listenVoice, stopConversation, stopSpeaking, startConversation } from "../../tauri";
import type { Attachment } from "../../types";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useSessionSocket } from "./useSessionSocket";
import type { useFollowScroll } from "./useFollowScroll";

/** Hands-free voice and sending a message. */
export function useVoiceAndSend(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> & ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll>) {
  const { handsFreeRef, spokenBufRef, setItems, sessionRef, model, followLatest } = prev;
  // Hands-free conversation. Refs as well as state: the socket handler is created
  // once and closes over its first render, so reading `handsFree` there would read
  // whatever it was when the session opened rather than what it is now.
  const [handsFree, setHandsFree] = useState(false);
  const [voiceState, setVoiceState] = useState<VoiceState>("off");
  const [voiceError, setVoiceError] = useState<string | null>(null);
  useEffect(() => {
    handsFreeRef.current = handsFree;
    if (!handsFree) spokenBufRef.current = "";
  }, [handsFree]);

  // A spoken turn goes through the SAME send path as typing — voice is an input
  // method, not a second conversation with its own rules.
  const sendRef = useRef<(t: string) => void>(() => {});
  useEffect(() => {
    if (!isTauri()) return;
    let stops: Array<() => void> = [];
    let live = true;
    void (async () => {
      const a = await listenVoice<{ state: VoiceState }>("voice-state", (p) => setVoiceState(p.state));
      const b = await listenVoice<{ text: string }>("voice-turn", (p) => {
        const said = (p.text || "").trim();
        if (said) sendRef.current(said);
      });
      // Barge-in already stopped the voice in Rust; drop the half-written sentence
      // so the interrupted tail is not spoken into the next answer.
      const c = await listenVoice<unknown>("voice-bargein", () => {
        spokenBufRef.current = "";
      });
      // A voice that fails quietly is indistinguishable from one that is listening,
      // which is the worst possible failure for a hands-free surface.
      const e = await listenVoice<{ message: string }>("voice-error", (p) => {
        if (p?.message) setVoiceError(p.message);
      });
      stops.push(e);
      if (!live) {
        a();
        b();
        c();
        return;
      }
      stops = [a, b, c];
    })();
    return () => {
      live = false;
      stops.forEach((s) => s());
    };
  }, []);

  const toggleHandsFree = async () => {
    if (handsFree) {
      setHandsFree(false);
      setVoiceState("off");
      await stopConversation().catch(() => {});
      await stopSpeaking().catch(() => {});
      return;
    }
    setVoiceError(null);
    try {
      await startConversation();
      setHandsFree(true);
    } catch (e) {
      setVoiceState("off");
      setHandsFree(false);
      // Surfaced rather than thrown into a caller that drops it: the button would
      // otherwise look like it had worked.
      setVoiceError(e instanceof Error ? e.message : "Couldn't start hands-free listening.");
    }
  };

  const send = (text: string, attachments?: Attachment[], skill?: string) => {
    // Force-run shows exactly what the user typed: "/name rest". Must match the server's
    // `display` sidecar formula so the turn_start dedupe recognizes the local echo.
    const shown = skill ? `/${skill}${text ? ` ${text}` : ""}` : text;
    setItems((p) => [...p, { kind: "user", text: shown, attachments, ts: Date.now() / 1000 }]);
    // The visible model rides along with the message (single source of truth per turn).
    sessionRef.current?.userMessage(text, attachments, model, skill);
    followLatest(); // sending always re-engages stream-following, wherever the user had scrolled
  };
  sendRef.current = (t: string) => send(t);

  return { handsFree, voiceState, voiceError, toggleHandsFree, send };
}
