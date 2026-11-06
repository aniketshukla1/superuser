import { useEffect, useRef, useState } from "react";
import type { RecentChannel } from "../../api";
import { useSlackRoster } from "./useSlackRoster";

// A channel input with a popover of recently-seen channels (the "recent list + type-the-id"
// picker). Free typing is allowed (a slack:C0123 address or a channel Copy-link URL). The
// popover is hand-rolled, NOT a <datalist>: WKWebView (the macOS desktop shell) doesn't render
// datalist suggestions at all, so the native path would silently show nothing on Mac.
export function ChannelPicker({
  value,
  onChange,
  recent,
  onSubmit,
  onPickName,
}: {
  value: string;
  onChange: (v: string) => void;
  recent: RecentChannel[];
  onSubmit?: () => void;
  // Fires when a pick RESOLVES a display name for the raw address — callers can echo the
  // human name (+ workspace) wherever they show the target (§25 consent line, summaries).
  onPickName?: (address: string, name: string, workspace?: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const wrap = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (wrap.current && !wrap.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const { roster, searching } = useSlackRoster(open, value);

  // Filter as the user types (name, address, or last-message text); full list on focus.
  const q = value.trim().toLowerCase();
  const options = recent.filter(
    (c) =>
      !q ||
      c.channel.toLowerCase().includes(q) ||
      (c.name || "").toLowerCase().includes(q) ||
      (c.last_text || "").toLowerCase().includes(q),
  );
  // Roster hits the recent list already covers would be duplicates — drop them.
  const seen = new Set(options.map((c) => c.channel));
  const lookups = roster.filter((r) => !seen.has(r.address));

  // Display ≠ value (owner catch 2026-07-11: the box showed `slack:T…/C…`): the stored value
  // stays the raw address, but at rest the input shows the channel's NAME when we know it —
  // from a pick (remembered), the recent list, or a roster hit. Focus flips back to the raw
  // address for editing; the tooltip always carries it.
  const [focused, setFocused] = useState(false);
  const [pickedName, setPickedName] = useState<Record<string, string>>({});
  const knownName =
    pickedName[value] ||
    recent.find((c) => c.channel === value)?.name ||
    roster.find((r) => r.address === value)?.name ||
    "";
  const display = !focused && knownName ? `#${knownName}` : value;

  // A pick is a commit: blur the input so the display flips to the channel name (focus is
  // otherwise held by the mousedown-preventDefault that protects the pick from the blur).
  const inputRef = useRef<HTMLInputElement | null>(null);

  return (
    <div className="relative flex-1 min-w-0" ref={wrap}>
      <input
        ref={inputRef}
        className="chan-input w-full"
        placeholder="slack:C0123 or channel link"
        value={display}
        title={value || undefined}
        onChange={(e) => {
          onChange(e.target.value);
          setOpen(true);
        }}
        onFocus={() => {
          setFocused(true);
          setOpen(true);
        }}
        onBlur={() => setFocused(false)}
        onKeyDown={(e) => {
          if (e.key === "Escape") setOpen(false);
          if (e.key === "Enter" && onSubmit) {
            setOpen(false);
            onSubmit();
          }
        }}
      />
      {open && (options.length > 0 || lookups.length > 0 || searching) && (
        <div
          className="absolute left-0 right-0 top-full mt-1 z-40 rounded-xl border border-line bg-panel shadow-lg py-1 max-h-56 overflow-y-auto"
          role="listbox"
          data-testid="channel-suggestions"
        >
          {options.map((c) => (
            <button
              key={c.channel}
              role="option"
              className="block w-full text-left px-3 py-1.5 hover:bg-paper"
              onMouseDown={(e) => {
                // mousedown (not click) so the pick lands before the input's blur
                e.preventDefault();
                onChange(c.channel);
                if (c.name) {
                  setPickedName((m) => ({ ...m, [c.channel]: c.name! }));
                  onPickName?.(c.channel, c.name);
                }
                setOpen(false);
                inputRef.current?.blur();
              }}
            >
              <span className="text-sm text-ink">
                {c.name ? `#${c.name}` : c.channel}
              </span>
              {c.name && <span className="ml-1.5 text-xs text-faint">{c.channel}</span>}
              {c.last_text && (
                <span className="block text-xs text-faint truncate">
                  {c.last_from ? `${c.last_from}: ` : ""}
                  {c.last_text}
                </span>
              )}
            </button>
          ))}
          {/* The wait is visible: the first lookup per workspace sweeps the full
              channel roster (seconds on a big workspace; cached 15 min after). */}
          {searching && lookups.length === 0 && (
            <div
              className="px-3 py-1.5 text-sm text-faint"
              data-testid="roster-searching"
            >
              searching your workspace’s channels…
            </div>
          )}
          {/* Live workspace-roster hits: type the NAME, we resolved the id. */}
          {lookups.map((r) => (
            <button
              key={r.address}
              role="option"
              className="block w-full text-left px-3 py-1.5 hover:bg-paper"
              data-testid={`roster-channel-${r.address}`}
              onMouseDown={(e) => {
                e.preventDefault();
                onChange(r.address);
                if (r.name) {
                  setPickedName((m) => ({ ...m, [r.address]: r.name }));
                  onPickName?.(r.address, r.name, r.workspace || undefined);
                }
                setOpen(false);
                inputRef.current?.blur();
              }}
            >
              <span className="text-sm text-ink">
                {r.is_private ? "🔒 " : "#"}
                {r.name}
              </span>
              {r.workspace && (
                <span className="ml-1.5 text-xs text-faint">{r.workspace}</span>
              )}
              {!r.is_member && (
                <span className="block text-xs text-warnInk">
                  invite @superuser to this channel in Slack so it can listen
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
