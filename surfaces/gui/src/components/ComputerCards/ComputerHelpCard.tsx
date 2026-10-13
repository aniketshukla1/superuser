import { useEffect, useRef, useState } from "react";
import { computerControl, getComputer, getComputerFrame, type ComputerControl } from "../../api";
import { Icon, ICON } from "../Icon";

/** Close a card whose request the browser no longer has — replayed after the fact, expired,
 *  or answered from the live view, where the same request is shown too. Checked while the
 *  card is up, so the two can never both be waiting for an answer already given. */
export function useLapsed(persona: string, open: (c: ComputerControl) => boolean, onEnded?: () => void) {
  const ended = useRef(onEnded);
  ended.current = onEnded;
  const still = useRef(open);
  still.current = open;
  useEffect(() => {
    let live = true;
    const check = () =>
      getComputer(persona)
        .then((s) => {
          if (live && s && !still.current(s.control)) ended.current?.();
        })
        .catch(() => {});
    void check();
    const t = window.setInterval(() => void check(), 2000);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [persona]);
}

// The assistant's own browser, in the conversation.
//
//   * ComputerTile — the screen a navigation KEPT. A finished turn shows the page it
//     opened, never whatever the browser shows today: a record must not change its mind.
//   * ComputerHelpCard — the turn is waiting for the user's hands (a sign-in, a CAPTCHA).
//     Asking is not taking: the assistant cannot hand itself to the user, only ask.
//   * SecretCard — the turn is waiting for one value typed straight into a page field.
//     The value goes to the page and nowhere else: not to the model, not to the trail,
//     not into this component's state once it has landed.

export const hostOf = (u: string) => {
  try {
    return new URL(u).host || u;
  } catch {
    return u;
  }
};

export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export function ComputerTile({
  persona,
  frame,
  url,
  title,
  onWatch,
}: {
  persona: string;
  frame: string;
  url: string;
  title: string;
  onWatch?: (persona: string) => void;
}) {
  // undefined = loading, null = no longer kept (30 days, or signed out since).
  const [src, setSrc] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    let live = true;
    getComputerFrame(persona, frame)
      .then((s) => live && setSrc(s))
      .catch(() => live && setSrc(null));
    return () => {
      live = false;
    };
  }, [persona, frame]);

  return (
    <figure className="rounded-xl2 border border-line bg-panel overflow-hidden max-w-[560px]" data-testid="computer-tile">
      {src ? (
        <img src={src} alt={title ? `The page “${title}” as it looked` : "The page as it looked"} className="block w-full h-auto" />
      ) : (
        <div className="aspect-[16/10] grid place-items-center text-sm text-faint">
          {src === null ? "That screen is no longer kept." : "Loading the screen…"}
        </div>
      )}
      <figcaption className="flex items-center gap-2 px-3 py-2 border-t border-line">
        <Icon name="globe" size={ICON.md} className="text-faint shrink-0" />
        <span className="min-w-0 flex-1 truncate text-sm text-ink" title={url}>
          {title || hostOf(url)}
          {title && <span className="text-faint"> — {hostOf(url)}</span>}
        </span>
        {onWatch && (
          <button className={BTN_BORDERED} onClick={() => onWatch(persona)} data-testid="computer-tile-watch">
            Watch live
          </button>
        )}
      </figcaption>
    </figure>
  );
}

export function ComputerHelpCard({
  persona,
  name,
  reason,
  url,
  onTaken,
  onDeclined,
  onEnded,
  compact,
}: {
  persona: string;
  name: string;
  reason: string;
  url: string;
  /** Called once control is the user's — open the live view here. */
  onTaken: (persona: string) => void;
  onDeclined: () => void;
  /** The request is no longer open in the browser. */
  onEnded?: () => void;
  compact?: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const firstRef = useRef<HTMLButtonElement>(null);
  useLapsed(persona, (c) => c.requested, onEnded);
  useEffect(() => {
    const t = window.setTimeout(() => firstRef.current?.focus(), 60);
    return () => clearTimeout(t);
  }, []);

  const act = async (action: "take" | "decline") => {
    setBusy(true);
    setError("");
    const res = await computerControl(persona, action).catch(() => null);
    setBusy(false);
    if (!res?.ok) {
      setError(res?.error || "That didn't reach the browser. Try again.");
      return;
    }
    if (action === "take") onTaken(persona);
    else onDeclined();
  };

  return (
    <div
      className={"rounded-xl2 border border-line bg-panel p-3" + (compact ? "" : " max-w-[560px]")}
      role="alertdialog"
      aria-label={`${name} needs you in its browser`}
      data-testid="computer-help-card"
    >
      <span className="sr-only" role="status" aria-live="polite">
        {`${name} needs you in its browser: ${reason}`}
      </span>
      <div className="flex items-start gap-2.5">
        <Icon name="globe" size={ICON.lg} className="text-accent shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="text-md font-medium text-ink">{name} needs your hands in its browser</div>
          {reason && <p className="text-sm text-ink mt-1 leading-prose">{reason}</p>}
          {url && <p className="text-sm text-faint mt-0.5 truncate">{hostOf(url)}</p>}
          <p className="text-sm text-muted mt-1.5 leading-prose">
            Take control, do what it asks, then hand the browser back. {name} can't act while you have it.
          </p>
        </div>
      </div>
      {error && <p className="text-sm text-danger mt-2">{error}</p>}
      <div className="flex items-center gap-2 mt-3">
        <button ref={firstRef} className={BTN_ACCENT} disabled={busy} onClick={() => void act("take")} data-testid="computer-help-take">
          Take control
        </button>
        <span className="flex-1" />
        <button className={BTN_BORDERED} disabled={busy} onClick={() => void act("decline")} data-testid="computer-help-decline">
          Not now
        </button>
      </div>
    </div>
  );
}
