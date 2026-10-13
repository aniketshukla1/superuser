import { useAppState } from "./useAppState";
import { useNavigationState } from "./useNavigationState";
import { useLayout } from "./useLayout";
import { usePersonas } from "./usePersonas";
import { useBoot } from "./useBoot";
import { useSplashAndSettings } from "./useSplashAndSettings";
import { useSessionSocket } from "./useSessionSocket";
import { useFollowScroll } from "./useFollowScroll";
import { useVoiceAndSend } from "./useVoiceAndSend";
import { useTurnActions } from "./useTurnActions";
import { useSessionSwitching } from "./useSessionSwitching";
import { useConversationActions } from "./useConversationActions";
import { useChrome } from "./useChrome";

/** Everything App renders from: its state, what it derives, and its handlers. */
export function useAppModel() {
  const m0 = {  };
  const m1 = { ...m0, ...useAppState() };
  const m2 = { ...m1, ...useNavigationState(m1) };
  const m3 = { ...m2, ...useLayout(m2) };
  const m4 = { ...m3, ...usePersonas(m3) };
  const m5 = { ...m4, ...useBoot(m4) };
  const m6 = { ...m5, ...useSplashAndSettings(m5) };
  const m7 = { ...m6, ...useSessionSocket(m6) };
  const m8 = { ...m7, ...useFollowScroll(m7) };
  const m9 = { ...m8, ...useVoiceAndSend(m8) };
  const m10 = { ...m9, ...useTurnActions(m9) };
  const m11 = { ...m10, ...useSessionSwitching(m10) };
  const m12 = { ...m11, ...useConversationActions(m11) };
  const m13 = { ...m12, ...useChrome(m12) };
  return m13;
}

export type AppModel = ReturnType<typeof useAppModel>;
