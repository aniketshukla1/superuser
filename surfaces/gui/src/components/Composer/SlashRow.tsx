import { useEffect, useRef } from "react";
import type { Attachment } from "../../types";
import { formatTokens } from "../../usage";
import { Icon, ICON } from "../Icon";

// A row in the "+" attach menu.
/** One command in the "/" list: a name, and the rest in the tooltip.
 *
 *  The scroll-into-view is not a flourish. The list used to sit in the document flow and
 *  show everything; it now floats and scrolls, so arrowing past the last visible row moved
 *  a highlight you could no longer see. `block: "nearest"` scrolls only when it has to,
 *  which keeps the list still while you move within it. */
export function SlashRow({
  name,
  hint,
  active,
  onHover,
  onPick,
}: {
  name: string;
  hint: string;
  active: boolean;
  onHover: () => void;
  onPick: () => void;
}) {
  const el = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    // Feature-tested, not assumed. jsdom has no layout and therefore no
    // scrollIntoView, and an unguarded call threw during commit — which does not fail
    // politely, it unmounts the whole composer. The same lesson the ResizeObserver
    // guard in Transcript.tsx records: anything that needs a real layout engine has to
    // be optional, because losing the scroll position is a degradation and losing the
    // input is not.
    if (active && typeof el.current?.scrollIntoView === "function") {
      el.current.scrollIntoView({ block: "nearest" });
    }
  }, [active]);
  return (
    <button
      ref={el}
      type="button"
      role="option"
      aria-selected={active}
      title={hint}
      className={"slash-row" + (active ? " is-active" : "")}
      onMouseEnter={onHover}
      onClick={onPick}
    >
      <span className="slash-name">{name}</span>
      <span className="slash-hint">{hint}</span>
    </button>
  );
}

/** The send arrow. Was inlined three times, which is three chances for them to drift. */
export function SendGlyph() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 19V5M5 12l7-7 7 7" />
    </svg>
  );
}

// One field of the usage table: a label and a session-summed value. Cumulative across
// the whole session, never just the last turn.
export function usageStat(label: string, value: number) {
  return (
    <div className="flex items-baseline justify-between text-xs leading-snug">
      <span className="text-faint">{label}</span>
      <span className="text-ink tabular-nums">{formatTokens(value)}</span>
    </div>
  );
}

export function AttachChip({ a, onRemove }: { a: Attachment; onRemove: () => void }) {
  return (
    <div className={"attach-chip" + (a.kind === "image" ? " img" : "")}>
      {a.kind === "image" ? (
        <img src={a.data_url} alt={a.name} />
      ) : (
        <>
          <Icon name="file" size={ICON.md} />
          <span className="attach-name">{a.name}</span>
        </>
      )}
      <button className="attach-x" onClick={onRemove} title="Remove">
        ✕
      </button>
    </div>
  );
}
