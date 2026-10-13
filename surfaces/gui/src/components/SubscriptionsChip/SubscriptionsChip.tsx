import { useEffect, useRef, useState } from "react";
import { getRecentChannels, subscribeChannel, unsubscribeChannel, type RecentChannel } from "../../api";
import { Icon, ICON } from "../Icon";
import { ChannelPicker } from "./ChannelPicker";

// The per-session "connections" chip in the composer head: shows how many channels this session
// listens to, and opens a popover to add (picker) / remove (×) — the per-session manage surface.
export function SubscriptionsChip({
  sessionId,
  channels,
  onChanged,
}: {
  sessionId: string;
  channels: string[];
  onChanged: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [recent, setRecent] = useState<RecentChannel[]>([]);
  const [draft, setDraft] = useState("");
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    getRecentChannels().then(setRecent).catch(() => setRecent([]));
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const add = async () => {
    const c = draft.trim();
    if (!c) return;
    await subscribeChannel(sessionId, c);
    setDraft("");
    onChanged();
  };
  const remove = async (c: string) => {
    await unsubscribeChannel(sessionId, c);
    onChanged();
  };

  return (
    <div className="sub-chip-wrap" ref={ref}>
      <button
        className={"wschip sub-chip" + (open ? " active" : "")}
        title="Channels this session listens to"
        onClick={() => setOpen((v) => !v)}
      >
        <Icon name="plug" size={ICON.sm} /> {channels.length || "+"}
      </button>
      {open && (
        <div className="sub-pop" onMouseDown={(e) => e.stopPropagation()}>
          <div className="sub-pop-head">Channels this session listens to</div>
          {channels.length === 0 ? (
            <div className="dim sub-pop-empty">Not subscribed to any channel.</div>
          ) : (
            channels.map((c) => {
              const nm = recent.find((r) => r.channel === c)?.name;
              return (
              <div className="sub-pop-row" key={c}>
                <span className="sub-pop-chan" title={c}>{nm ? `#${nm}` : c}</span>
                <button className="sub-pop-x" title="Unsubscribe" onClick={() => remove(c)}>
                  ×
                </button>
              </div>
              );
            })
          )}
          <div className="sub-pop-add">
            <ChannelPicker value={draft} onChange={setDraft} recent={recent} onSubmit={add} />
            <button className="btn-primary sm" disabled={!draft.trim()} onClick={add}>
              Add
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
