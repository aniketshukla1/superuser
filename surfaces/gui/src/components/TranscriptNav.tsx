import { useCallback, useEffect, useRef, useState } from "react";
import type { Item } from "../types";

/**
 * A map of the questions in a conversation, and a way back to any of them.
 *
 * A long session scrolls past the point where scrolling finds anything — you remember
 * asking about the thing, not how far up it was. This lists what you asked, in order, and
 * jumps.
 *
 * Collapsed it is a column of ticks, one per turn, with the one you are reading marked.
 * That much is ambient: it says how long the conversation is and where in it you are,
 * without asking for a click or covering any text. Hovering — or tabbing in — expands it to
 * the questions themselves.
 *
 * Entries are USER turns only. The replies are answers to these, so a combined list would
 * be the same conversation written out twice, and "what did I ask?" is the question people
 * actually have when they go looking.
 */
const MIN_TURNS = 2; // a map of one thing is not a map

function firstLine(text: string): string {
  const line = text.trim().split("\n").find((l) => l.trim().length > 0) || text.trim();
  return line.replace(/\s+/g, " ").trim();
}

export function TranscriptNav(props: {
  items: Item[];
  scrollRef: React.RefObject<HTMLElement | null>;
}) {
  const [open, setOpen] = useState(false);
  const [current, setCurrent] = useState(-1);
  // Which entry the Tab key lands on. A twenty-five-turn conversation put twenty-five stops
  // between the page and its own text; a list navigated with the arrow keys has one.
  const [focused, setFocused] = useState(0);
  const rowRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const turns = props.items
    .filter((it) => it.kind === "user")
    .map((it, n) => ({ n, label: firstLine((it as { text: string }).text) }));

  // Which turn you are reading: the last one whose top has passed the top of the viewport.
  // Read from the DOM on scroll rather than tracked with an observer per row, because rows
  // are added and replaced while a reply streams and a stale observer points at a node that
  // no longer exists.
  const sync = useCallback(() => {
    const scroller = props.scrollRef.current;
    if (!scroller) return;
    const top = scroller.getBoundingClientRect().top;
    let seen = -1;
    scroller.querySelectorAll<HTMLElement>("[data-turn]").forEach((el) => {
      // 24px of slack: a turn whose top edge is level with the viewport still reads as
      // "the one I am on", not "the one before".
      if (el.getBoundingClientRect().top - top <= 24) seen = Number(el.dataset.turn);
    });
    setCurrent(seen);
  }, [props.scrollRef]);

  useEffect(() => {
    const scroller = props.scrollRef.current;
    if (!scroller) return;
    sync();
    scroller.addEventListener("scroll", sync, { passive: true });
    return () => scroller.removeEventListener("scroll", sync);
  }, [sync, props.scrollRef, props.items.length]);

  if (turns.length < MIN_TURNS) return null;

  const go = (turn: number) => {
    const scroller = props.scrollRef.current;
    if (!scroller) return;
    // Honour the OS setting. This is a jump across a long document, which is exactly the
    // movement that makes motion-sensitive people ill.
    const behavior: ScrollBehavior = window.matchMedia?.("(prefers-reduced-motion: reduce)")
      .matches
      ? "auto"
      : "smooth";
    if (turn < 0) {
      scroller.scrollTo({ top: 0, behavior });
      return;
    }
    const el = scroller.querySelector<HTMLElement>(`[data-turn="${turn}"]`);
    if (!el) return;
    const delta = el.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    scroller.scrollTo({ top: scroller.scrollTop + delta - 20, behavior });
  };

  const entries = [{ n: -1, label: "Session start" }, ...turns];

  return (
    <nav
      className={"tnav" + (open ? " tnav-open" : "")}
      aria-label="Jump to a question"
      data-testid="transcript-nav"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      // Tabbing in opens it too. A list that exists only under a pointer is a list a
      // keyboard user is told about and then cannot read.
      onFocus={() => setOpen(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOpen(false);
      }}
    >
      {entries.map((e, i) => {
        const isCurrent = e.n === current;
        const move = (to: number) => {
          const at = (to + entries.length) % entries.length;
          setFocused(at);
          rowRefs.current[at]?.focus();
        };
        return (
          <button
            key={e.n}
            ref={(el) => {
              rowRefs.current[i] = el;
            }}
            className={"tnav-item" + (isCurrent ? " is-current" : "")}
            onClick={() => go(e.n)}
            onFocus={() => setFocused(i)}
            tabIndex={i === Math.min(focused, entries.length - 1) ? 0 : -1}
            onKeyDown={(ev) => {
              if (ev.key === "ArrowDown") {
                ev.preventDefault();
                move(i + 1);
              } else if (ev.key === "ArrowUp") {
                ev.preventDefault();
                move(i - 1);
              } else if (ev.key === "Home") {
                ev.preventDefault();
                move(0);
              } else if (ev.key === "End") {
                ev.preventDefault();
                move(entries.length - 1);
              }
            }}
            title={e.label}
            aria-current={isCurrent ? "true" : undefined}
          >
            <span className="tnav-tick" aria-hidden />
            <span className="tnav-label">{e.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
