import { useRef, useEffect } from "react";
import { rememberLastSession } from "../appHelpers";
import { TRACE, trace, chars } from "../../trace";
import { Session } from "../../api";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import { eventHandler } from "./sessionEvents/handleEvent";

/** The live session socket: connect, and apply every event the engine sends. */
export function useSessionSocket(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings>) {
  const { surface, agent, sessionId, workspace, booting, gatesWorkspace, setItems, setConnected,
      refreshSessions, pendingPromptRef, sessionRef } = prev;
  const handsFreeRef = useRef(false);
  const spokenBufRef = useRef("");

  // There used to be a guard here that watched `agent` and switched it away when the
  // "surface" it belonged to was hidden in Settings. It is gone, and the reason is
  // worth keeping.
  //
  // It was written for a fixed set of three surfaces — cowork, chat, code — that you
  // could hide individually. The persona roster replaced that: availability is now
  // `enabled` and `surfaced` per assistant, and `surface_chat` / `surface_code` are
  // read by nothing else in this app. So the guard compared `agent` against surface
  // ids no persona has, which made it permanently inert, and I "fixed" it by pointing
  // it at the real `coder` id — which turned it live against a flag that defaults to
  // FALSE. Every click on a Coder session then bounced straight back to the general
  // assistant, opening a different conversation than the one clicked.
  //
  // The lesson is not "be careful with that flag". It is that an effect which watches
  // `agent` and also SETS it can undo an explicit choice the user just made, and does
  // it one render later where nothing connects cause to effect. `selectSession` is the
  // user speaking; nothing may overrule it from a `useEffect`. `appEffects.test.ts`
  // holds that line.

  useEffect(() => {
    if (surface === "session") rememberLastSession(agent, sessionId, workspace);
  }, [surface, agent, sessionId, workspace]);

  // (re)connect when workspace, session, or agent changes
  useEffect(() => {
    if (booting) return; // wait until boot/resume settles the session before connecting
    if (gatesWorkspace(agent) && !workspace) return; // Code needs a folder (gate handles it)
    const handleEvent = eventHandler({ ...prev, handsFreeRef, spokenBufRef });

    const session = new Session(sessionId, workspace || "", agent, {
      onEvent: handleEvent,
      onOpen: () => {
        setConnected(true);
        // Auto-send the task prompt once a "Run now" session connects.
        const p = pendingPromptRef.current;
        if (p) {
          pendingPromptRef.current = null;
          setItems((prev) => [...prev, { kind: "user", text: p, ts: Date.now() / 1000 }]);
          TRACE && trace("chat", "auto-send on connect", { session: sessionId, text: chars(p) });
          sessionRef.current?.userMessage(p);
        }
      },
      onClose: () => setConnected(false),
    });
    sessionRef.current = session;
    return () => session.close();
    // NOTE: `workspace` is intentionally NOT a dependency. Every real workspace change
    // (pick folder, select/switch session, new session) is paired with a `sessionId`
    // change, so the socket still reconnects when it should. The one workspace-only change
    // is the `ready` handler adopting the server's provisioned Cowork scratch dir — listing
    // `workspace` here made that adoption tear down and rebuild the socket immediately after
    // first connect, dropping the user's first message (the "send twice" bug). The scratch
    // dir is deterministic from `sessionId` server-side, so skipping that reconnect is safe.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [booting, sessionId, agent, refreshSessions]);

  return { handsFreeRef, spokenBufRef };
}
