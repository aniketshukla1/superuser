import { useMemo, type CSSProperties } from "react";
import { Art, Brackets } from "./Feeds";
import type { Acts } from "./script";
import { FINISHED } from "./work";

const DONE = "task complete";
const KPIS: [string, string, string?][] = [
  ["revenue", "$4.2M", "▲ 18.4%"],
  ["margin", "31%"],
  ["churn", "2.1%", "▼ 0.4"],
];

/**
 * The end of the hunt: one piece of work, finished, brought to the front and captured —
 * a shutter's flash, brackets locking round it, "task complete" typed under it — the way
 * the owner described it (2026-10-13). Then, as the scan crosses, it folds into the
 * centre, which is where the mark draws itself.
 *
 * It runs on its own clock from `acts.capture`; nothing here waits on anything.
 */
export function Capture({ acts }: { acts: Acts }) {
  // When it was captured: the real time, which is the only stamp here that is not staged.
  const stamp = useMemo(() => new Date().toLocaleTimeString("en-GB", { hour12: false }), []);
  const t = acts.capture;
  // The beats inside the capture scale with the cut: a second and a half, or one.
  const span = acts.sweep - t;
  const shot = { "--shot": `${Math.round(t + span * 0.28)}ms` } as CSSProperties;
  return (
    <>
    <div
      className="film-capture"
      style={{ "--in": `${t}ms`, "--out": `${acts.sweep}ms`, ...shot } as CSSProperties}
      aria-hidden="true"
    >
      <div className="film-capture-card">
        <div className="film-feed-head">
          <span className="film-feed-who">
            @{FINISHED.who} · {FINISHED.doing}
          </span>
          <span className="film-feed-live">captured {stamp}</span>
        </div>
        <div className="film-capture-kpis">
          {KPIS.map(([k, v, d]) => (
            <div key={k} className="film-kpi">
              <span>{k}</span>
              <b>{v}</b>
              {d && <em>{d}</em>}
            </div>
          ))}
        </div>
        <div className="film-capture-charts">
          <Art art="bars" seed={71} t={t + 100} />
          <Art art="line" seed={72} t={t + 100} />
        </div>
        <Brackets className="film-lock film-capture-lock" />
      </div>
      <div className="film-done">
        {("✓ " + DONE).split("").map((ch, i) => (
          <span key={i} className="film-char" style={{ animationDelay: `${Math.round(t + span * 0.4 + i * 28)}ms` }}>
            {ch}
          </span>
        ))}
      </div>
      <div className="film-done-meta" style={{ animationDelay: `${Math.round(t + span * 0.66)}ms` }}>
        4 charts · 12,408 rows · 3m 12s · waiting for your ok
      </div>
    </div>
    {/* The shutter: outside the card, whose transform would otherwise make "fixed" mean
        "fixed to the card". */}
    <div className="film-flash" style={shot} aria-hidden="true" />
    </>
  );
}
