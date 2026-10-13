import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getRunJournal, type RunEvent, type RunJournal } from "../../api";
import type { TodoItem } from "../../types";
import { formatTime } from "../ArtifactViewer";
import { ToolRow, toolCallsFrom, type ToolCall } from "./ToolRow";
import { SplitHandle, useSplit } from "./SplitHandle";
import { Icon, ICON } from "../Icon";

/* ---- The tab -------------------------------------------------------------- */

export function ActivityTab({
  sessionId,
  running,
  refreshKey,
  todo,
  onCallsChange,
}: {
  sessionId: string;
  running: boolean;
  refreshKey: number;
  todo: TodoItem[];
  /** Lets the panel badge the Files tab with what this run touched. */
  onCallsChange?: (calls: ToolCall[]) => void;
}) {
  const [journal, setJournal] = useState<RunJournal | null>(null);
  const [pct, setPct] = useSplit();
  const boxRef = useRef<HTMLDivElement>(null);
  const rawRef = useRef<HTMLDivElement>(null);
  // Follow the feed only while the reader is already at the bottom. Yanking them
  // back down mid-scroll is how a live log becomes unreadable.
  const stick = useRef(true);

  const refresh = useCallback(() => {
    getRunJournal(sessionId, 0, 500)
      .then(setJournal)
      .catch(() => setJournal(null));
  }, [sessionId]);

  useEffect(() => {
    refresh();
  }, [refresh, refreshKey]);

  // While a turn is in flight the journal grows underneath us; poll so the feed
  // moves with the work instead of only after it.
  useEffect(() => {
    if (!running) return;
    const id = setInterval(refresh, 1200);
    return () => clearInterval(id);
  }, [running, refresh]);

  // Memoised against `journal`, not written inline: `journal?.events ?? []` allocates a
  // fresh array on every render, so the memo below never held and the effect that
  // reports calls upward re-fired forever.
  const events = useMemo(() => journal?.events ?? [], [journal]);
  const calls = useMemo(() => toolCallsFrom(events), [events]);

  useEffect(() => {
    onCallsChange?.(calls);
  }, [calls, onCallsChange]);

  useEffect(() => {
    const el = rawRef.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [events]);

  const onRawScroll = () => {
    const el = rawRef.current;
    if (!el) return;
    stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
  };

  // Nothing has happened yet: one calm empty state, not two panes each saying "nothing"
  // (one of them as a debug readout). The split and the raw frames arrive with the first frame.
  if (!running && calls.length === 0 && events.length === 0 && todo.length === 0) {
    return (
      <div className="activity-tab" ref={boxRef}>
        <div className="act-blank" data-testid="act-blank">
          <Icon name="clock" size={ICON.lg} />
          <div className="act-blank-title">No activity yet</div>
          <div className="act-blank-text">
            Every tool the assistant uses in this conversation appears here as it runs, with the raw
            frames behind it.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="activity-tab" ref={boxRef}>
      <div className="act-top" style={{ height: `${pct}%` }} data-testid="act-top">
        {todo.length > 0 && (
          <div className="act-plan">
            {todo.map((item, i) => (
              <div className={"rail-todo " + item.status} key={i}>
                <span className="rail-todo-mark" />
                <span>{item.content}</span>
              </div>
            ))}
          </div>
        )}

        {calls.length === 0 ? (
          <div className="rail-muted act-empty">
            {running ? "Working…" : "Nothing recorded for this conversation yet."}
          </div>
        ) : (
          <div className="act-list">
            {journal?.truncated && (
              <div className="rail-muted act-note">Older lines were dropped — the journal is capped.</div>
            )}
            {calls.map((c) => (
              <ToolRow key={c.seq} call={c} />
            ))}
          </div>
        )}
      </div>

      <SplitHandle boxRef={boxRef} pct={pct} onResize={setPct} />

      <div className="act-raw-pane" data-testid="act-raw-pane">
        <div className="act-raw-head">
          <span className="act-raw-label">Raw output</span>
          <span className="act-raw-count">{journal?.total ?? events.length}</span>
        </div>
        <div className="act-raw-list" ref={rawRef} onScroll={onRawScroll} data-testid="act-raw-list">
          {events.length === 0 ? (
            <div className="rail-muted act-empty">{running ? "Waiting for frames…" : "No frames recorded."}</div>
          ) : (
            events.map((e) => (
              <div key={e.seq} className="act-raw-row">
                <span className="act-raw-time">{formatTime(e.at)}</span>
                <span className="act-raw-type">{e.type}</span>
                <span className="act-raw-detail">{rawText(e)}</span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Everything this frame carried, in full.
 *
 * A frame is `{ at, type, data }` and `data` is whatever that frame had — the
 * journal records frames as they pass, it doesn't impose a schema on them. So this
 * reads the fields that turn up in practice and falls back to the raw shape rather
 * than showing nothing.
 *
 * Nothing is clipped here and newlines are kept: this is the half you come to when
 * the readable summary above has let you down, and a truncated honest answer is no
 * more use than no answer. The pane scrolls; the text does not need to fit.
 */
export function rawText(e: RunEvent): string {
  const d = (e.data ?? {}) as Record<string, unknown>;
  const first = d.name ?? d.tool ?? d.text ?? d.status ?? d.answer ?? d.error;
  if (typeof first === "string") return first;
  const keys = Object.keys(d);
  if (!keys.length) return "";
  try {
    return JSON.stringify(d, null, 2);
  } catch {
    // Cyclic or otherwise unserialisable data still deserves a line rather than a
    // thrown render.
    return String(d);
  }
}
