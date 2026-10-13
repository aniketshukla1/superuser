import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { isTauri, speak, stopSpeaking } from "../../tauri";
import { Icon, ICON } from "../Icon";
import { exactTime, timeAgo } from "../../when";

// Long user pastes swallow the transcript (owner ask 2026-07-30): clamp past a generous
// threshold with a more…/less… toggle. Normal typed messages never see the control; the
// full text still drives copy (BubbleMeta) and is what the model received.
/** Lines of a long prompt shown before it is folded away.
 *
 *  Measured in LINES, not characters. The old rule cut at 1200 characters, which is
 *  a different height for every prompt — a wall of short lines and a paragraph of
 *  prose both hit 1200 and collapsed to wildly different boxes. What the reader
 *  cares about is how much of the screen their own question is taking, and that is
 *  a line count. */
export const USER_CLAMP_LINES = 8; // must match `-webkit-line-clamp` on `.user-clamp`
void USER_CLAMP_LINES;

export function ClampedUserText({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  const [needsClamp, setNeedsClamp] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  // Whether it overflows is a question about the rendered box, not the string —
  // the same text clamps at one window width and doesn't at another.
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const check = () => setNeedsClamp(el.scrollHeight > el.clientHeight + 2);
    check();
    // Guarded, not assumed: jsdom has no ResizeObserver, and a component that throws
    // in a test environment will throw in whatever else lacks the API. The initial
    // measurement still runs; only re-measuring on resize is lost, which is a
    // degradation rather than a failure.
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(check);
    ro.observe(el);
    return () => ro.disconnect();
  }, [text]);

  return (
    <>
      {/* One class at a time, never both: the fade means "hidden below", which is
          false when it fits and false again when it is open. */}
      <div
        ref={box}
        className={"user-clamp" + (open ? " is-open" : needsClamp ? " is-clamped" : "")}
      >
        {text}
      </div>
      {(needsClamp || open) && (
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          className="user-clamp-more"
          data-testid="user-show-more"
        >
          {open ? "Show less" : "Show more"}
        </button>
      )}
    </>
  );
}

// Hover affordances for a message bubble (FB-005): copy the raw text + the message's time.
// Lives in a ZERO-HEIGHT strip under the bubble (absolute, inside the transcript's 20px gap)
// so revealing it on group-hover never shifts the layout. `ts` is unix seconds — canonical
// messages carry it, pre-stamp history doesn't, so the time simply omits itself when absent.
// "2 minutes ago" rather than a clock time: on a message you are looking at now, how long ago
// is the question, and the exact time is in the tooltip for when it is not. The formatter is
// the app's shared one (when.ts), so a week-old message reads "5 Sep" here as it does
// everywhere else, not "9 days ago" here and a date on the next screen.

// Reading aloud is a desktop-shell capability (a Tauri command). Computed once rather than
// per render: it cannot change while the app is open.
export const canSpeak = isTauri();

export function BubbleMeta({
  text,
  ts,
  align,
  speakable,
  onRetry,
  extra,
}: {
  text: string;
  ts?: number;
  align: "left" | "right";
  /** Read-aloud is offered on replies, not on your own messages: you wrote it, and a
   *  speaker icon under every question is two controls where one is useful. */
  speakable?: boolean;
  /** Offered on the last reply only — "try that again" means the most recent answer, and a
   *  retry button beside an older one would silently regenerate something else. */
  onRetry?: () => void;
  /** More controls for this row, placed after retry and before the time. The row is laid over
   *  the space under the bubble, so anything that should sit beside these controls has to be
   *  IN the row: rendered after it, it lands in the same spot and the two overlap. */
  extra?: ReactNode;
}) {
  const [copied, setCopied] = useState(false);
  const [speaking, setSpeaking] = useState(false);
  const when = typeof ts === "number" ? new Date(ts * 1000) : null;
  const copy = () => {
    // "Copied" only after the write actually lands — WebKit can reject outside a
    // trusted gesture, and claiming success on a silent no-op would gaslight the user.
    navigator.clipboard
      ?.writeText(text)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      })
      .catch(() => {});
  };
  return (
    <div className="relative h-0 select-none">
      <div
        className={
          "absolute top-1 flex items-center gap-2 text-xs leading-none text-faint whitespace-nowrap opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity " +
          (align === "right" ? "right-0" : "left-0")
        }
      >
        <button
          className="hit-target flex items-center cursor-pointer hover:text-muted"
          data-testid="bubble-copy"
          title="Copy message"
          onClick={copy}
        >
          {copied ? "Copied" : <Icon name="copy" size={ICON.sm} />}
        </button>
        {/* Read it aloud. Only in the desktop shell — `speak` is a Tauri command, and a
            button that throws in a browser is worse than one that is not offered. It is a
            toggle: the same control that started a two-minute answer has to be the one that
            stops it, or stopping means hunting for something else. */}
        {canSpeak && speakable && (
          <button
            className="hit-target flex items-center cursor-pointer hover:text-muted"
            data-testid="bubble-speak"
            title={speaking ? "Stop reading" : "Read aloud"}
            aria-label={speaking ? "Stop reading" : "Read aloud"}
            onClick={() => {
              if (speaking) {
                stopSpeaking().catch(() => {});
                setSpeaking(false);
                return;
              }
              setSpeaking(true);
              // NOT cleared when the promise resolves. `speak` returns once the shell has
              // QUEUED the text, not when it has finished saying it — resolving on that
              // would flip the control back to "Read aloud" milliseconds after you pressed
              // it, while the machine was still talking, and there would be no way to stop
              // it. Nothing emits a speech-finished event today (voice.rs broadcasts
              // listening/hearing/thinking, never speaking), so the control stays on Stop
              // until you press it. The cost is a label that can outlive the speech; the
              // alternative was a Stop button you could never catch. Pressing Stop when
              // nothing is playing is harmless.
              speak(text).catch(() => setSpeaking(false));
            }}
          >
            <Icon name={speaking ? "stop" : "speaker"} size={ICON.sm} />
          </button>
        )}
        {onRetry && (
          <button
            className="hit-target flex items-center cursor-pointer hover:text-muted"
            data-testid="bubble-retry"
            title="Try that again"
            aria-label="Try that again"
            onClick={onRetry}
          >
            <Icon name="refresh" size={ICON.sm} />
          </button>
        )}
        {extra}
        {when && (
          <span data-testid="bubble-ts" title={exactTime(when)}>
            {timeAgo(when)}
          </span>
        )}
      </div>
    </div>
  );
}
