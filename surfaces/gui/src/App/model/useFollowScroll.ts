import { useRef, useState, useEffect } from "react";
import { getArtifacts, getInbox, getUnattended } from "../../api";
import { stillOpen } from "../../sessionInbox";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useSessionSocket } from "./useSessionSocket";

/** Stream-following: keep the transcript at the bottom while the user is there. */
export function useFollowScroll(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> & ReturnType<typeof useSessionSocket>) {
  const { scrollRef, sessionId, items, streaming, surface, setArtifactCount, agent,
      browserRefreshKey, setSessionInbox, markUnattended } = prev;
  // Stream-following (FB-004): auto-scroll only while the user is AT the bottom, so scrolling
  // up to read during a streaming turn sticks. `atBottomRef` is the live truth (per scroll
  // event, no re-render); `following` mirrors it into state for the jump-to-latest pill.
  // Programmatic smooth-scrolls fire scroll events of their own — while one is in flight
  // (`autoScrollingRef`) they must not read as "the user scrolled up", or every stream tick
  // would disengage its OWN follow. The animation only moves down, so a decreasing scrollTop
  // mid-flight can only be the user taking over.
  const atBottomRef = useRef(true);
  const autoScrollingRef = useRef(false);
  const lastScrollTopRef = useRef(0);
  const [following, setFollowing] = useState(true);
  const scrollToBottom = () => {
    const el = scrollRef.current;
    if (!el) return;
    autoScrollingRef.current = true;
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  };
  const followLatest = () => {
    atBottomRef.current = true;
    setFollowing(true);
    scrollToBottom();
  };
  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const top = el.scrollTop;
    const atBottom = el.scrollHeight - top - el.clientHeight < 48;
    if (autoScrollingRef.current) {
      if (atBottom) autoScrollingRef.current = false; // landed
      else if (top >= lastScrollTopRef.current) {
        lastScrollTopRef.current = top; // still animating down — not the user
        return;
      } else autoScrollingRef.current = false; // moved UP mid-flight — user takeover
    }
    lastScrollTopRef.current = top;
    atBottomRef.current = atBottom;
    setFollowing(atBottom);
  };
  // A different session is a fresh viewport — never inherit a scrolled-up state. Declared
  // BEFORE the auto-scroll effect: when a session switch and its hydrated items land in one
  // commit, the reset must run first or the stale ref would skip the initial bottom-scroll.
  useEffect(() => {
    atBottomRef.current = true;
    setFollowing(true);
  }, [sessionId]);
  useEffect(() => {
    if (atBottomRef.current) scrollToBottom();
  }, [items, streaming]);

  // Track produced-file count for the topbar "Artifacts" affordance (works even when the rail is
  // hidden, where the rail itself doesn't fetch). Refreshes on file writes/turn end.
  //
  // No persona test. It was `agent === "cowork"` — one assistant, by an id the roster
  // dropped — and I first replaced that with "any assistant that isn't repo-bound",
  // which was wrong in the other direction: `session_artifacts` lists the SESSION's
  // own folder, not the workspace, so "files this conversation produced" means the
  // same thing for a Coder as for anyone else. On this machine the session with the
  // most artifacts is a Coder one, and my rule was hiding exactly that.
  //
  // `artifactCount > 0` was always the real gate. A persona test on top of it could
  // only ever take the affordance away from someone who had files to show.
  //
  // The fetch retries, for the same reason the persona load above does. This is read
  // once per session and its failure was swallowed, so a request that lost the race
  // with the sidecar left the count at 0 with nothing to correct it: the Artifacts
  // button never appeared for the whole life of that session, on a conversation that
  // demonstrably had files. Owner-hit 2026-09-01, on a reload that happened to land
  // while the backend was restarting — the count came back the moment anything else
  // moved the deps.
  //
  // A failure and an empty folder are NOT the same answer, which is why `catch` no
  // longer stands in for "none". Zero is only written when the server actually said
  // zero.
  useEffect(() => {
    if (surface !== "session") {
      setArtifactCount(0);
      return;
    }
    let stopped = false;
    const load = (attempt = 0) => {
      getArtifacts(sessionId)
        .then((a) => !stopped && setArtifactCount(a.length))
        .catch(() => {
          if (!stopped && attempt < 5) setTimeout(() => load(attempt + 1), 250 * (attempt + 1));
        });
    };
    load();
    return () => {
      stopped = true;
    };
  }, [agent, surface, sessionId, browserRefreshKey]);

  // Keep the active session's pending Inbox items fresh (answer-in-context card). Loads on session
  // change + after each turn, plus a slow poll so an unattended agent's new question surfaces.
  useEffect(() => {
    if (surface !== "session") return;
    const load = () => {
      // `stillOpen` on top of the server's own filter: a card for an item that
      // cannot be answered can only mislead, and this closes the window between a
      // stale fetch and the next poll. See `sessionInbox`.
      getInbox(sessionId, "pending")
        .then((items) => setSessionInbox(stillOpen(items)))
        .catch(() => setSessionInbox([]));
      getUnattended(sessionId).then(markUnattended).catch(() => markUnattended(false));
    };
    load();
    const t = setInterval(load, 4000);
    return () => clearInterval(t);
  }, [surface, sessionId, browserRefreshKey, markUnattended]);

  return { following, followLatest, handleScroll };
}
