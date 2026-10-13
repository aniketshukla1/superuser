import type { ConnectionScope } from "../../api";

// A channel address's platform: "slack:C0123" → "slack"; a bare id or "#mention" defaults to
// slack (the backend's own default when no platform prefix is given).
export const platformOf = (channel: string) => (channel.includes(":") ? channel.split(":")[0] : "slack");

export const TAG_CORE =
  "text-xs px-1.5 py-0.5 rounded-full bg-warnSoft/70 text-warnInk border border-warnInk/15";
export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0";
export const BTN_BORDERED =
  "ctl border border-line bg-paper hover:border-lineStrong shrink-0";

/**
 * Read / Write for one source — the source half of the trust model the Folders list
 * already has as RO/RW. Shown only for a source a message can actually leave through;
 * anywhere else it would be a control that gates nothing.
 *
 * `scope` is optional on the wire (a server older than this feature omits it), and an
 * absent value reads as "write" — that is what every connected source could do before
 * the gate existed, and an upgrade must not quietly revoke it.
 */
export function ScopePicker({
  scope,
  disabled,
  onPick,
}: {
  scope: ConnectionScope;
  disabled?: boolean;
  onPick: (next: ConnectionScope) => void;
}) {
  return (
    <div className="scope-picker" role="group" aria-label="What this session may do">
      {(["read", "write"] as const).map((s) => (
        <button
          key={s}
          type="button"
          className={"scope-opt" + (scope === s ? " on" : "")}
          aria-pressed={scope === s}
          disabled={disabled}
          data-testid={`scope-${s}`}
          title={
            s === "read"
              ? "Consult this source, but never send or post through it"
              : "May send, post or modify through this source"
          }
          onClick={() => scope !== s && onPick(s)}
        >
          {s === "read" ? "Read" : "Write"}
        </button>
      ))}
    </div>
  );
}
