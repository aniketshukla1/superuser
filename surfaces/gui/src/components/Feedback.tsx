import { useState } from "react";
import { sendFeedback, type FeedbackReason } from "../api";
import { Icon, ICON } from "./Icon";

/**
 * Thumbs on an answer — the only judgement in this app that is not a model's.
 *
 * The harness scoreboard is deliberately judge-free: every check is a pure function
 * over a record, because a model grading a model makes the grader's reliability the
 * thing in question. That rule costs coverage. Two failure modes cannot be checked
 * mechanically at all — an answer that cites a result the tool never returned, and
 * one that quietly prefers a memory a newer one replaced. Both need judgement about
 * meaning. A person supplies it without the circularity.
 *
 * A bare thumbs-down is close to worthless, so this does not collect one. "Bad" says
 * a turn was wrong and nothing about which part was wrong, and a hundred of them
 * still say nothing. A downvote therefore opens a short fixed list of reasons — the
 * same discipline `ask_user` is held to, where a question with no options is refused,
 * because writing the options is the work. The list is fetched from the server rather
 * than held here: two copies drift, and the store fills with reasons the fold ignores.
 *
 * Nothing here is sent to a model. It is written to a file a person reads.
 *
 * Split in three so the transcript can put the parts in two places: the thumbs belong in
 * the reply's action row with copy and retry, and the "what was wrong?" list needs room
 * under the reply. They were one inline element dropped after the row, and because the row
 * is positioned over that same spot, the arrows landed between its icons.
 */

export interface FeedbackTarget {
  sessionId?: string;
  messageAt?: number;
  onGiven?: (verdict: "up" | "down") => void;
}

/** The verdict's state and the one way to file it. `ready` is false without a session and a
 *  message to attach to: a verdict filed against the wrong turn is worse than none. */
export function useFeedbackVerdict({ sessionId, messageAt, onGiven }: FeedbackTarget) {
  const [picking, setPicking] = useState(false);
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);

  const send = async (verdict: "up" | "down", reason?: string) => {
    if (!sessionId || !messageAt) return;
    setBusy(true);
    try {
      await sendFeedback({ session: sessionId, messageAt, verdict, reason, note: note.trim() || undefined });
      onGiven?.(verdict);
      setPicking(false);
      setNote("");
    } finally {
      setBusy(false);
    }
  };

  return { ready: !!sessionId && !!messageAt, picking, setPicking, note, setNote, busy, send };
}

/** The two thumbs, dressed like the controls beside them in a reply's action row. */
export function FeedbackButtons({
  given,
  busy,
  onUp,
  onDown,
}: {
  given?: "up" | "down";
  busy: boolean;
  onUp: () => void;
  onDown: () => void;
}) {
  return (
    <span className="feedback" role="group" aria-label="Rate this answer">
      <button
        className={"feedback-btn hit-target" + (given === "up" ? " on" : "")}
        title="This was good"
        aria-label="This was good"
        aria-pressed={given === "up"}
        disabled={busy}
        onClick={onUp}
      >
        <Icon name="thumbUp" size={ICON.sm} />
      </button>
      <button
        className={"feedback-btn hit-target" + (given === "down" ? " on" : "")}
        title="Something was wrong"
        aria-label="Something was wrong"
        aria-pressed={given === "down"}
        disabled={busy}
        onClick={onDown}
      >
        <Icon name="thumbDown" size={ICON.sm} />
      </button>
    </span>
  );
}

/** "What was wrong?" — a fixed list of reasons, an optional note, and a way out. */
export function FeedbackReasons({
  reasons,
  busy,
  note,
  onNote,
  onPick,
  onCancel,
}: {
  reasons: FeedbackReason[];
  busy: boolean;
  note: string;
  onNote: (note: string) => void;
  onPick: (reasonId: string) => void;
  onCancel: () => void;
}) {
  return (
    <div className="feedback-reasons" role="group" aria-label="What was wrong?">
      <div className="feedback-prompt">What was wrong?</div>
      {reasons.map((r) => (
        <button key={r.id} className="feedback-reason" disabled={busy} onClick={() => onPick(r.id)}>
          {r.label}
        </button>
      ))}
      <input
        className="feedback-note"
        placeholder="Anything else? (optional)"
        value={note}
        onChange={(e) => onNote(e.target.value)}
        aria-label="Optional note"
      />
      <button className="feedback-cancel" disabled={busy} onClick={onCancel}>
        Cancel
      </button>
    </div>
  );
}

/** Both parts in one place, for anywhere that has no action row to share. */
export function Feedback({
  sessionId,
  messageAt,
  reasons,
  given,
  onGiven,
}: FeedbackTarget & {
  reasons: FeedbackReason[];
  given?: "up" | "down";
}) {
  const fb = useFeedbackVerdict({ sessionId, messageAt, onGiven });
  if (!fb.ready) return null;
  return fb.picking ? (
    <FeedbackReasons
      reasons={reasons}
      busy={fb.busy}
      note={fb.note}
      onNote={fb.setNote}
      onPick={(id) => void fb.send("down", id)}
      onCancel={() => fb.setPicking(false)}
    />
  ) : (
    <FeedbackButtons given={given} busy={fb.busy} onUp={() => void fb.send("up")} onDown={() => fb.setPicking(true)} />
  );
}
