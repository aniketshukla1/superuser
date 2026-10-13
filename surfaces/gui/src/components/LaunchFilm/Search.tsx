import { useEffect, useRef, useState, type CSSProperties } from "react";
import { motion } from "framer-motion";
import { Brackets } from "./Feeds";
import { lockTimes, type Cut, type FeedPlan } from "./script";

/** A point on screen, in px. */
export interface Point {
  x: number;
  y: number;
}

interface Target {
  key: string;
  who: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * The machine checking the work.
 *
 * A reticle on top of everything, in screen space, jumping from screen to screen: it
 * finds where one is at that instant, snaps around it, and hands over to that screen's
 * own brackets, which travel on with it through depth. Each lock sends a line back to
 * the centre, where the mark will form. A screen out of frame, or too small to read, is
 * passed over rather than locked where nobody can see it.
 */
export function Search({
  cut,
  plan,
  onLock,
  centre,
}: {
  cut: Cut;
  plan: FeedPlan[];
  onLock: (key: string, at: Point) => void;
  centre: () => Point;
}) {
  const [target, setTarget] = useState<Target | null>(null);
  const [pings, setPings] = useState<{ id: number; from: Point; to: Point }[]>([]);
  const [count, setCount] = useState(0);
  const locked = useRef(new Set<string>());
  const latest = useRef(onLock);
  latest.current = onLock;

  useEffect(() => {
    // Near planes first: they are the ones big enough to read.
    const order = [...plan.filter((f) => f.plane >= 4), ...plan.filter((f) => f.plane === 3), ...plan.filter((f) => f.plane === 2)];
    const times = lockTimes(cut);
    const timers = times.map((at, n) =>
      window.setTimeout(() => {
        const found = pick(order, locked.current);
        if (!found) return;
        locked.current.add(found.key);
        setTarget(found);
        setCount((c) => c + 1);
        const from = { x: found.x + found.w / 2, y: found.y + found.h / 2 };
        setPings((p) => [...p.slice(-5), { id: n, from, to: centre() }]);
        window.setTimeout(() => latest.current(found.key, from), 120);
      }, at),
    );
    const done = window.setTimeout(() => setTarget(null), times[times.length - 1] + 260);
    return () => {
      timers.forEach((t) => window.clearTimeout(t));
      window.clearTimeout(done);
    };
    // The plan and the centre are fixed for the life of one film.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="film-search" aria-hidden="true">
      <svg className="film-pings">
        {pings.map((p) => (
          <line
            key={p.id}
            className="film-ping"
            x1={p.from.x}
            y1={p.from.y}
            x2={p.to.x}
            y2={p.to.y}
            style={{ "--len": Math.hypot(p.to.x - p.from.x, p.to.y - p.from.y) } as CSSProperties}
          />
        ))}
      </svg>
      {target && (
        <motion.div
          className="film-reticle"
          initial={false}
          animate={{ x: target.x - 10, y: target.y - 10, width: target.w + 20, height: target.h + 20, opacity: 1 }}
          transition={{ type: "spring", stiffness: 520, damping: 38, mass: 0.6 }}
        >
          <Brackets className="film-reticle-corners" />
          <span className="film-reticle-label">
            task {target.key.replace("feed-", "0x")} · @{target.who}
          </span>
        </motion.div>
      )}
      <div className="film-hud">
        <div className="film-hud-title">superuser // at work</div>
        <div>tasks&nbsp;&nbsp;{String(plan.length).padStart(2, "0")}</div>
        <div>
          done&nbsp;&nbsp;&nbsp;<b>{String(count).padStart(2, "0")}</b>
        </div>
      </div>
    </div>
  );
}

/** The next screen worth locking: not locked yet, in frame, and big enough to read. */
function pick(order: FeedPlan[], locked: Set<string>): Target | null {
  const w = window.innerWidth;
  const h = window.innerHeight;
  for (const feed of order) {
    if (locked.has(feed.key)) continue;
    const el = document.querySelector<HTMLElement>(`[data-feed="${feed.key}"]`);
    if (!el) continue;
    const r = el.getBoundingClientRect();
    const visible = r.left > 8 && r.top > 40 && r.right < w - 8 && r.bottom < h - 60;
    const plane = el.closest<HTMLElement>(".film-plane");
    const seen = Number(getComputedStyle(el).opacity) * (plane ? Number(getComputedStyle(plane).opacity) : 1);
    if (!visible || r.width < 120 || seen < 0.6) continue;
    return { key: feed.key, who: feed.task.who, x: r.left, y: r.top, w: r.width, h: r.height };
  }
  return null;
}
