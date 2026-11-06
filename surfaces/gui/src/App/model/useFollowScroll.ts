import { useEffect, useRef, useState } from "react";
import { getArtifacts, getInbox, getUnattended } from "../../api";
import { stillOpen } from "../../sessionInbox";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";
import type { useSessionSocket } from "./useSessionSocket";
import type { useSplashAndSettings } from "./useSplashAndSettings";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> &
  ReturnType<typeof useSessionSocket>;

/** How close to the end still counts as "at the bottom", in pixels. */
const BOTTOM_SLACK = 48;
const INBOX_POLL_MS = 4000;
const ARTIFACT_RETRIES = 5;

/** Following the stream: the transcript stays pinned to the end only while the reader is
 *  there, plus two per-session reads that ride along (the file count and the Inbox). */
export function useFollowScroll(prev: Deps) {
  const { agent, browserRefreshKey, items, markUnattended, scrollRef, sessionId, setArtifactCount, setSessionInbox, streaming, surface } = prev;

  // FB-004: scroll along only while the reader is AT the end, so scrolling up mid-stream to
  // read holds. `pinned` is the live answer, updated on every scroll without a render;
  // `following` copies it into state for the jump-to-latest pill. A smooth scroll started
  // here fires scroll events of its own, and while one is moving (`gliding`) those must not
  // count as the reader leaving, or each new chunk would unpin itself. The glide only goes
  // down, so a smaller scrollTop during it means the reader took over.
  const pinned = useRef(true);
  const gliding = useRef(false);
  const lastTop = useRef(0);
  const [following, setFollowing] = useState(true);

  const glideToEnd = () => {
    const box = scrollRef.current;
    if (!box) return;
    gliding.current = true;
    box.scrollTo({ top: box.scrollHeight, behavior: "smooth" });
  };

  const followLatest = () => {
    pinned.current = true;
    setFollowing(true);
    glideToEnd();
  };

  const handleScroll = () => {
    const box = scrollRef.current;
    if (!box) return;
    const top = box.scrollTop;
    const atEnd = box.scrollHeight - top - box.clientHeight < BOTTOM_SLACK;
    if (gliding.current) {
      const stillGoingDown = !atEnd && top >= lastTop.current;
      if (stillGoingDown) {
        lastTop.current = top;
        return;
      }
      // Either it arrived, or it moved up mid-glide: the reader has taken over.
      gliding.current = false;
    }
    lastTop.current = top;
    pinned.current = atEnd;
    setFollowing(atEnd);
  };

  // Each session opens pinned. This effect must come BEFORE the one below: when a switch and
  // its loaded items land in the same commit, resetting first is what lets the first scroll to
  // the end happen.
  useEffect(() => {
    pinned.current = true;
    setFollowing(true);
  }, [sessionId]);

  useEffect(() => {
    if (pinned.current) glideToEnd();
  }, [items, streaming]);

  // How many files this conversation has made, for the topbar's "Artifacts" button; read here
  // so it works with the panel hidden, when the panel itself fetches nothing. It lists the
  // SESSION's own folder, so it means the same for every assistant, and `> 0` is the whole
  // test (an assistant check here once hid the button from the very sessions with the most
  // files). It retries because it is read once per session: a request that lost the race
  // with the sidecar used to leave the count at 0 for the life of the session (owner,
  // 2026-09-01). A failed read is not an empty folder, so 0 is only written when the server
  // says 0.
  useEffect(() => {
    if (surface !== "session") {
      setArtifactCount(0);
      return;
    }
    let stopped = false;
    const read = (attempt: number) => {
      getArtifacts(sessionId)
        .then((files) => !stopped && setArtifactCount(files.length))
        .catch(() => {
          if (stopped || attempt >= ARTIFACT_RETRIES) return;
          setTimeout(() => read(attempt + 1), 250 * (attempt + 1));
        });
    };
    read(0);
    return () => {
      stopped = true;
    };
  }, [agent, surface, sessionId, browserRefreshKey]);

  // The open session's pending Inbox items, for the answer-in-context card: read on a session
  // change and after each turn, and polled slowly so a question from an unattended agent shows.
  useEffect(() => {
    if (surface !== "session") return;
    const refresh = () => {
      // `stillOpen` on top of the server's own filter: a card for something that can no longer
      // be answered only misleads, and this closes the gap between a stale read and the next
      // poll (see `sessionInbox`).
      getInbox(sessionId, "pending")
        .then((pending) => setSessionInbox(stillOpen(pending)))
        .catch(() => setSessionInbox([]));
      getUnattended(sessionId)
        .then(markUnattended)
        .catch(() => markUnattended(false));
    };
    refresh();
    const poll = setInterval(refresh, INBOX_POLL_MS);
    return () => clearInterval(poll);
  }, [surface, sessionId, browserRefreshKey, markUnattended]);

  return { following, followLatest, handleScroll };
}
