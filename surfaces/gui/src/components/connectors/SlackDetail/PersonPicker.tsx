import { useEffect, useRef, useState } from "react";
import { allowUser, getSlackDirectory, type SlackMember } from "../../../api";

// "Find your name in a list": typeahead over the workspace directory (users.list,
// cached on the desktop). A pick lands on the allow-list with the display name in
// hand — the park→approve flow stays as the path for senders nobody pre-added.
export function PersonPicker({
  teamId,
  allowed,
  onChanged,
  onPick,
  buttonLabel = "＋ Add person",
  testId,
}: {
  teamId: string | null;
  allowed: string[];
  onChanged: () => void;
  onPick?: (member: SlackMember) => Promise<{ ok: boolean; error?: string }>;
  buttonLabel?: string;
  testId?: string;
}) {
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<SlackMember[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const wrap = useRef<HTMLSpanElement | null>(null);
  const btn = useRef<HTMLButtonElement | null>(null);
  // Fixed-position drop: the group cards clip overflow (GRP is overflow-hidden),
  // so an absolute popover inside them would be cut off after the first row.
  // Fixed means it can't be scrolled to either, so a button near the bottom of the window opens
  // it upward: dropped below, the list sat past the window's edge with no way to reach it.
  const [pos, setPos] = useState<{ top?: number; bottom?: number; left: number } | null>(null);
  const PICKER_H = 320;
  const toggle = () => {
    if (open) return setOpen(false);
    const r = btn.current?.getBoundingClientRect();
    const left = r ? Math.min(r.left, window.innerWidth - 300) : 0;
    const below = r ? window.innerHeight - r.bottom : 0;
    setPos(
      !r
        ? null
        : below < PICKER_H && r.top > below
          ? { bottom: window.innerHeight - r.top + 4, left }
          : { top: r.bottom + 4, left },
    );
    setOpen(true);
  };

  useEffect(() => {
    if (!open) return;
    const t = setTimeout(() => {
      getSlackDirectory(teamId || "default", q)
        .then((r) => {
          if (r.ok) {
            setRows(r.members || []);
            setErr(null);
          } else setErr(r.error || "directory unavailable");
        })
        .catch(() => setErr("directory unavailable"));
    }, 200);
    return () => clearTimeout(t);
  }, [open, q, teamId]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (wrap.current && !wrap.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const pick = async (m: SlackMember) => {
    const result = onPick
      ? await onPick(m)
      : await allowUser("slack", m.id, teamId, m.name);
    if (result?.ok === false) {
      setErr(result.error || "could not add person");
      return;
    }
    setOpen(false);
    setQ("");
    onChanged();
  };
  const candidates = rows.filter((m) => !allowed.includes(m.id));

  return (
    <span className="relative" ref={wrap}>
      <button
        ref={btn}
        className="inline-flex items-center px-2 py-0.5 rounded-full border border-dashed border-line text-sm text-muted hover:text-ink hover:border-faint"
        data-testid={testId || `add-person-${teamId || "default"}`}
        title="Pick from the workspace directory"
        onClick={toggle}
      >
        {buttonLabel}
      </button>
      {open && (
        <div
          className="fixed z-50 w-72 rounded-xl border border-line bg-panel shadow-lg p-1"
          style={{ top: pos?.top, bottom: pos?.bottom, left: pos?.left }}
          data-testid="person-picker"
        >
          <input
            autoFocus
            className="w-full bg-paper border border-line rounded-lg px-2 py-1 text-sm outline-none placeholder:text-faint"
            placeholder="Type a name…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setOpen(false);
            }}
          />
          <div className="max-h-56 overflow-y-auto py-1">
            {err ? (
              <div className="px-2 py-1.5 text-sm text-warnInk">{err}</div>
            ) : candidates.length === 0 ? (
              <div className="px-2 py-1.5 text-sm text-faint">no matches</div>
            ) : (
              candidates.map((m) => (
                <button
                  key={m.id}
                  className="block w-full text-left px-2 py-1.5 rounded-lg hover:bg-paper"
                  data-testid={`pick-person-${m.id}`}
                  title={`id ${m.id}`}
                  onMouseDown={(e) => {
                    // mousedown (not click) so the pick lands before the input's blur
                    e.preventDefault();
                    pick(m);
                  }}
                >
                  <span className="text-sm font-medium">{m.name}</span>{" "}
                  <span className="text-xs text-faint">@{m.handle}</span>
                  {m.guest && (
                    <span className="ml-1.5 text-xs text-warnInk bg-warnSoft/70 border border-warnInk/15 rounded px-1 py-0.5">
                      guest
                    </span>
                  )}
                </button>
              ))
            )}
          </div>
          <div className="px-2 pb-1 text-xs text-faint">
            From your workspace directory — stays on this computer.
          </div>
        </div>
      )}
    </span>
  );
}
