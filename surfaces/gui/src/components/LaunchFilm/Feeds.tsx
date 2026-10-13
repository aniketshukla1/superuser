import { useMemo, type CSSProperties, type ReactNode } from "react";
import { bars, calendar, donut, kpi, line } from "./artCharts";
import { checklist, code, email, kanban, ledger, terminal, type R } from "./artDocs";
import { PLANES, rng, type FeedPlan } from "./script";
import type { Art as ArtKind } from "./work";

/** What the machine said about a screen it locked onto, and when. */
export interface Lock {
  at: number;
  verdict: string;
}

/**
 * The work, flying out of the dark.
 *
 * Screens sit still on six planes at different depths and the camera flies forward
 * through them: near planes sweep past at the edges, far ones grow out of the haze, and
 * the parallax between them is what makes it read as space rather than as cards sliding
 * about. Each screen is framed like a live feed — who is
 * working, on what, a light that says it is live — and draws its own work as it arrives.
 */
export function Feeds({ plan, locks }: { plan: FeedPlan[]; locks: Map<string, Lock> }) {
  // Worked out once, against the window as it opened.
  const view = useMemo(() => ({ w: window.innerWidth || 1280, h: window.innerHeight || 800 }), []);
  return (
    <div className="film-depth" aria-hidden="true">
      <div className="film-camera">
        {PLANES.map((z, p) => {
          // How much wider than the screen this plane's visible area is from where the
          // camera starts, so a screen placed at the frame's edge starts at the edge.
          const spread = (900 - (z - 400)) / 900;
          return (
            <div
              key={z}
              className="film-plane"
              style={{
                transform: `translateZ(${z}px)`,
                opacity: [0.3, 0.42, 0.58, 0.74, 0.9, 1][p],
                filter: p === 0 ? "blur(1.6px)" : undefined,
              }}
            >
              {plan
                .filter((f) => f.plane === p)
                .map((f) => (
                  <Feed
                    key={f.key}
                    feed={f}
                    lock={locks.get(f.key)}
                    x={f.sx * (view.w / 2) * spread * 0.78}
                    y={f.sy * (view.h / 2) * spread * 0.78}
                  />
                ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Feed({ feed, lock, x, y }: { feed: FeedPlan; lock?: Lock; x: number; y: number }) {
  const { task } = feed;
  return (
    <div
      data-feed={feed.key}
      className="film-feed"
      style={{ "--px": `${x}px`, "--py": `${y}px`, animationDelay: `${Math.round(feed.appear)}ms` } as CSSProperties}
    >
      <div className="film-feed-head">
        <span className="film-feed-who">
          @{task.who} · {task.doing}
        </span>
        <span className="film-feed-live">
          <i /> live {feed.stamp}
        </span>
      </div>
      <Art art={task.art} seed={feed.seed} t={feed.appear} />
      <span className="film-band" style={{ animationDelay: `${(feed.seed % 7) * 230}ms` }} />
      {lock && (
        <>
          <Brackets className="film-lock" />
          <Verdict lock={lock} />
        </>
      )}
    </div>
  );
}

export function Brackets({ className }: { className: string }) {
  return (
    <span className={className}>
      <i className="tl" />
      <i className="tr" />
      <i className="bl" />
      <i className="br" />
    </span>
  );
}

/** "working…", then the verdict typed over it. Mounted at the lock, so delays count from it. */
function Verdict({ lock }: { lock: Lock }) {
  return (
    <span className="film-verdict">
      <span className="film-assessing">working…</span>
      <span className="film-verdict-text">
        <span className="film-char" style={{ animationDelay: "360ms" }}>
          ✓{" "}
        </span>
        {lock.verdict.split("").map((ch, i) => (
          <span key={i} className="film-char" style={{ animationDelay: `${380 + i * 22}ms` }}>
            {ch}
          </span>
        ))}
      </span>
    </span>
  );
}

const DRAW: Record<ArtKind, (r: R, t: number, id: number) => ReactNode> = {
  code, email, kanban, checklist, ledger, terminal, bars, line, donut, calendar, kpi,
};

/** One screen's work, generated from its seed: the same picture every render. */
export function Art({ art, seed, t, className = "film-art" }: { art: ArtKind; seed: number; t: number; className?: string }) {
  return (
    <svg className={className} viewBox="0 0 270 150" aria-hidden="true">
      {DRAW[art](rng(seed), t, seed)}
    </svg>
  );
}
