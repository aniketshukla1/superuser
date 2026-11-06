import { useEffect, useRef } from "react";
import { Session } from "../../api";
import { chars, TRACE, trace } from "../../trace";
import { rememberLastSession } from "../appHelpers";
import { eventHandler } from "./sessionEvents/handleEvent";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";
import type { useSplashAndSettings } from "./useSplashAndSettings";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings>;

/** The live session socket: connecting, and applying every event the engine sends. */
export function useSessionSocket(prev: Deps) {
  const { agent, booting, gatesWorkspace, pendingPromptRef, refreshSessions, sessionId, sessionRef, setConnected, setItems, surface, workspace } = prev;
  const handsFreeRef = useRef(false);
  const spokenBufRef = useRef("");

  // A guard once lived here that watched `agent` and switched away from it when its "surface"
  // was hidden in Settings. It was written for three fixed surfaces (cowork, chat, code), which
  // the assistant list replaced, so it compared `agent` with ids no assistant has and did
  // nothing. Pointing it at the real `coder` id made it act on a flag that defaults to FALSE,
  // and every click on a Coder session bounced back to the general assistant, opening a
  // different conversation from the one clicked. The rule that came out of it: an effect that
  // watches `agent` and also SETS it can overturn a choice the user just made, a render later,
  // where nothing ties the effect to its cause. `selectSession` is the user speaking, and no
  // `useEffect` may overrule it (`appEffects.test.ts` holds that line).

  useEffect(() => {
    if (surface === "session") rememberLastSession(agent, sessionId, workspace);
  }, [surface, agent, sessionId, workspace]);

  // Connect again whenever the session or the assistant changes.
  useEffect(() => {
    // Boot and resume decide the session first.
    if (booting) return;
    // Code needs a folder before anything connects; the gate deals with that.
    if (gatesWorkspace(agent) && !workspace) return;

    // A "Run now" session sends its task as soon as it connects.
    const sendPendingRun = () => {
      const prompt = pendingPromptRef.current;
      if (!prompt) return;
      pendingPromptRef.current = null;
      setItems((before) => [...before, { kind: "user", text: prompt, ts: Date.now() / 1000 }]);
      TRACE && trace("chat", "auto-send on connect", { session: sessionId, text: chars(prompt) });
      sessionRef.current?.userMessage(prompt);
    };

    const live = new Session(sessionId, workspace || "", agent, {
      onEvent: eventHandler({ ...prev, handsFreeRef, spokenBufRef }),
      onOpen: () => {
        setConnected(true);
        sendPendingRun();
      },
      onClose: () => setConnected(false),
    });
    sessionRef.current = live;
    return () => live.close();
    // `workspace` is left out of the dependencies on purpose. Every real change of folder
    // (choosing one, selecting or switching a session, a new session) comes with a new
    // `sessionId`, so the socket still reconnects when it should. The one change of folder
    // alone is `ready` taking the scratch folder the server made for Cowork, and listing
    // `workspace` made that tear the socket down right after the first connect and lose the
    // user's first message (the "send twice" bug). The server derives the scratch folder from
    // `sessionId`, so not reconnecting for it is safe.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [booting, sessionId, agent, refreshSessions]);

  return { handsFreeRef, spokenBufRef };
}
