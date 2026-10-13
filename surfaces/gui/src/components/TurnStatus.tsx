import { useEffect, useState } from "react";

/**
 * What the turn is doing, while it is doing it.
 *
 * Elapsed time, tokens so far, and the thing currently being waited on — the three
 * questions people actually have during a long turn, in the order they ask them. It sits
 * under the transcript and is replaced by the finished message when the turn ends.
 *
 * The clock ticks from the browser rather than the server. The server would have to send a
 * message a second to say nothing, and a turn that has genuinely stalled should still show
 * time passing — a frozen counter reads as "finished" when it means "no news".
 */
function elapsed(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rest = s % 60;
  if (m < 60) return `${m}m ${rest}s`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

const compact = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n));

export function TurnStatus(props: { startedAt: number | null; tokens: number; label?: string }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!props.startedAt) return;
    // Every second, and only while a turn is actually open — a timer left running behind an
    // idle screen re-renders the transcript once a second for nothing.
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [props.startedAt]);

  const parts = [
    props.startedAt ? elapsed(now - props.startedAt) : null,
    // Only once the provider has reported. Not every provider does, and "0 tokens" would be
    // a claim about the turn rather than an admission about the provider.
    props.tokens > 0 ? `${compact(props.tokens)} tokens` : null,
    props.label || "Working…",
  ].filter(Boolean);

  return (
    <div className="turn-status" data-testid="turn-status">
      {/* aria-live=polite, not assertive: this updates every second, and a screen reader
          reading a clock aloud over the answer is worse than not having it. Polite means it
          waits for a pause; the seconds themselves are aria-hidden for the same reason. */}
      <span className="turn-status-glyph" aria-hidden />
      <span aria-live="polite">{parts.join(" · ")}</span>
    </div>
  );
}
