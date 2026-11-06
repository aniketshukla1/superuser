import { useCallback, useEffect, useState, useRef } from "react";
import { TRACE, trace } from "../../trace";
import { loadTranscript } from "../../sessionTranscript";
import { usageFromMessages, emptyUsage } from "../../usage";
import { RAIL_MIN_W } from "../appHelpers";
import type { GateReturn } from "../../gateCancel";
import type { useAppState } from "./useAppState";

/** Which session, persona page, live view and gate is showing, and how to open each. */
export function useNavigationState(prev: ReturnType<typeof useAppState>) {
  const { route, setSessionIdState, navigate, sessionId, leaveConversation, sessions, agent,
      setAgent, workspace, setWorkspace, setBranch, setItems, setUsage, setShowGate } = prev;
  // The persona whose detail page is showing — from the URL, so `#/persona/ops` is
  // a real address. Empty falls back to the active session's persona.
  const personaViewId = route.surface === "persona" ? route.id || "" : "";
  // Which conversation you're in, as an address. Step 6 gave every *surface* a URL
  // and left this one out, so a link to "that thread where we worked out the
  // migration" didn't exist and reloading always dropped you somewhere else.
  //
  // The id stays React state rather than being read straight off the URL, because
  // it has to survive leaving the session view: standing on Brain, there is still
  // a conversation you'd go back to, and `#/brain` has nowhere to put it. So state
  // is the truth and the URL is a mirror, synced both ways below.
  //
  // `setSessionId` keeps its old signature — all nine call sites are untouched,
  // the same trick `setSurface` used — it just writes the hash as well.
  const setSessionId = useCallback(
    (id: string) => {
      setSessionIdState(id);
      navigate({ surface: "session", id });
    },
    [navigate],
  );
  // The other direction: Back, Forward, or a pasted link. Adopting the id is
  // enough to switch — the effect on `sessionId` reconnects the socket and loads
  // the transcript, exactly as clicking the row in the sidebar would.
  useEffect(() => {
    if (route.surface === "session" && route.id && route.id !== sessionId) {
      setSessionIdState(route.id);
      // The same clearing the sidebar path does — see `leaveConversation`.
      leaveConversation("route");
      TRACE && trace("chat", "select session", { to: route.id, via: "route" });
      // And the same adoption: a link into another persona's conversation has to
      // bring that persona and its folder with it, or the socket connects as
      // whoever you happened to be looking at and the header names the wrong one.
      const known = sessions.find((s) => s.session_id === route.id);
      if (known) {
        if (known.agent && known.agent !== agent) setAgent(known.agent);
        if (known.workspace && known.workspace !== workspace) {
          setWorkspace(known.workspace);
          setBranch(null);
        }
      }
      loadTranscript(route.id)
        .then(({ messages: m, items: loaded }) => {
          setItems(loaded);
          setUsage(usageFromMessages(m));
        })
        .catch(() => {
          setItems([]);
          setUsage(emptyUsage());
        });
    }
    // `sessionId` is deliberately not a dependency: this reacts to the URL
    // changing, and including it would make the effect chase its own writes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.surface, route.id]);
  // Where "back" goes from an assistant's page: the open session, or Settings ▸ Personas when
  // the page was opened from there (assistants are configured in Settings now).
  const [personaViewReturn, setPersonaViewReturn] = useState<"session" | "settings">("session");
  const openPersona = (id: string, from: "session" | "settings" = "session") => {
    setPersonaViewReturn(from);
    navigate({ surface: "persona", id });
  };
  const [browserRefreshKey, setBrowserRefreshKey] = useState(0);
  // Whose browser is open live, if anyone's — from a kept screen or a request for help.
  const [liveView, setLiveView] = useState<string | null>(null);
  // At the 880px minimum the window allows, keeping both panels left the transcript a
  // 284px gutter and pushed the composer's own buttons out under the rail, where the rail
  // won the hit test and Send could not be clicked (UX-REPORT §4). The first threshold,
  // 1040, fixed that and nothing above it: at the 1100px default the panel still opened
  // and took the chat down to its floor. RAIL_MIN_W is now derived from what has to fit
  // (see its definition). Below it the rail starts closed; the topbar toggle still opens
  // it, and below 1040 it opens as an overlay (see styles.css).
  const [railHidden, setRailHidden] = useState(
    () => typeof window !== "undefined" && window.innerWidth < RAIL_MIN_W,
  );
  // Only follows the window while the user hasn't taken over: once they toggle the
  // rail themselves, their choice wins until the next launch.
  const railChosen = useRef(false);
  // Where cancelling the folder gate goes back to.
  //
  // Opening a workbench from the New menu is a NAVIGATION: it switches the agent,
  // clears the workspace and puts the gate up. Cancelling it should undo that
  // navigation. It used to invent a destination instead — the general assistant —
  // so declining the folder dialog moved you off the conversation you were in and
  // onto the Home tab, which is not what "cancel" means anywhere.
  //
  // Null when nothing sensible preceded the gate (a true first run), and the
  // assistant fallback is then right: it is the case that reasoning was written for.
  const gateReturn = useRef<GateReturn | null>(null);
  /** Raise the folder gate, saying what it navigated away from.
   *
   * The one way in. Four call sites raise this dialog and only two of them recorded a
   * return point, so the same cancel landed you back where you were from the New menu
   * and on Home from a persona header — one bug wearing two behaviours. Routing every
   * open through here means a fifth site cannot be added without answering the
   * question, and `gateCancel.test.ts` holds that line. */
  const openGate = (from: GateReturn | null) => {
    gateReturn.current = from;
    setShowGate(true);
  };
  useEffect(() => {
    const onResize = () => {
      if (railChosen.current) return;
      setRailHidden(window.innerWidth < RAIL_MIN_W);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return { personaViewId, setSessionId, personaViewReturn, openPersona, browserRefreshKey, setBrowserRefreshKey, liveView, setLiveView, railHidden, setRailHidden, railChosen, gateReturn, openGate };
}
