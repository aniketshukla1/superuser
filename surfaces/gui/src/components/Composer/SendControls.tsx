import { Icon, ICON } from "../Icon";
import { SendGlyph } from "./SlashRow";

/** Send (and, mid-turn, Stop beside it). */
export function SendControls({
  running,
  hasContent,
  connected,
  blocked,
  needsModel,
  onSubmit,
  onInterrupt,
}: {
  running?: boolean;
  hasContent: boolean;
  connected?: boolean;
  /** Recording or transcribing: nothing can be sent until that ends. */
  blocked: boolean;
  needsModel: boolean;
  onSubmit: () => void;
  onInterrupt?: () => void;
}) {
  return (
    <div className="shrink-0 flex items-center gap-1.5">
      {/* Mid-turn, Send appears beside Stop rather than replacing it: you can
          redirect the agent or halt it, and those are different intentions. */}
      {running && hasContent && (
        <button
          className="w-7 h-7 rounded-full grid place-items-center shrink-0 bg-accent text-onAccent hover:brightness-105 transition"
          onClick={onSubmit}
          title="Send — the agent picks this up at its next step"
          aria-label="Send"
          data-testid="composer-steer"
        >
          <SendGlyph />
        </button>
      )}
      {running ? (
        <button
          className="w-7 h-7 rounded-full grid place-items-center shrink-0 bg-danger text-onAccent hover:brightness-110 transition"
          onClick={onInterrupt}
          title="Stop"
          aria-label="Stop"
          data-testid="composer-stop"
        >
          <Icon name="stop" size={ICON.md} />
        </button>
      ) : (
        <button
          className={
            "w-7 h-7 rounded-full grid place-items-center shrink-0 transition-colors " +
            (hasContent && connected && !blocked
              ? "bg-accent text-onAccent hover:brightness-105"
              : "bg-paper border border-line text-faint")
          }
          onClick={onSubmit}
          disabled={!connected || blocked}
          title={needsModel ? "Connect a model to send" : undefined}
          aria-label="Send"
        >
          <SendGlyph />
        </button>
      )}
    </div>  );
}
