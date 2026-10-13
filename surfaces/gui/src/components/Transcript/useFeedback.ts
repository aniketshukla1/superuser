import { useEffect, useState } from "react";
import { getFeedback, type FeedbackReason } from "../../api";

/** This conversation's thumbs: the reasons on offer and the verdicts already given. */
export function useFeedback(sessionId?: string) {
  // Feedback state. Fetched per session rather than held in App: it is read once
  // when a conversation opens and changed only by clicking, so threading it through
  // the app's state would be motion for its own sake.
  const [reasons, setReasons] = useState<FeedbackReason[]>([]);
  const [given, setGiven] = useState<Record<number, "up" | "down">>({});
  useEffect(() => {
    let live = true;
    if (!sessionId) return;
    getFeedback(sessionId)
      .then((f) => {
        if (!live) return;
        setReasons(f.reasons);
        const seen: Record<number, "up" | "down"> = {};
        for (const g of f.given) if (g.message_at) seen[g.message_at] = g.verdict;
        setGiven(seen);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [sessionId]);
  return { reasons, given, setGiven };
}
