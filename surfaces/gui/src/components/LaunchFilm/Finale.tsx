import { useEffect, useState, type CSSProperties } from "react";
import { rng } from "./script";
import type { Point } from "./Search";

/**
 * Everything coming home: a pulse from every place the machine locked, and from all
 * round the edge of the frame, each running a gentle curve into the centre and
 * quickening as it is pulled in. Pulses rather than lines: a pulse is
 * only ever where it is now, so the screen is empty again by the time the mark draws.
 */
export function Converge({ at, from, centre }: { at: number; from: () => Point[]; centre: () => Point }) {
  const [streaks, setStreaks] = useState<{ d: string; length: number; delay: number; duration: number }[]>([]);
  const [home, setHome] = useState<Point | null>(null);

  useEffect(() => {
    const t = window.setTimeout(() => {
      const to = centre();
      const w = window.innerWidth;
      const h = window.innerHeight;
      const r = rng(7);
      const ring = Array.from({ length: 14 }, (_, i) => {
        const a = (i / 14) * Math.PI * 2 + r() * 0.35;
        return { x: w / 2 + Math.cos(a) * w * 0.56, y: h / 2 + Math.sin(a) * h * 0.6 };
      });
      setHome(to);
      setStreaks(
        [...from(), ...ring].map((p, i) => {
          const dx = to.x - p.x;
          const dy = to.y - p.y;
          const straight = Math.hypot(dx, dy) || 1;
          // Bent a little to one side, so they arrive like a current rather than spokes.
          const bend = (r() - 0.5) * 0.45 * straight;
          const c = { x: (p.x + to.x) / 2 - (dy / straight) * bend, y: (p.y + to.y) / 2 + (dx / straight) * bend };
          return {
            d: `M${p.x},${p.y} Q${c.x},${c.y} ${to.x},${to.y}`,
            length: curveLength(p, c, to),
            delay: Math.round((i % 7) * 46 + r() * 70),
            duration: Math.round(440 + r() * 220),
          };
        }),
      );
    }, at);
    return () => window.clearTimeout(t);
    // Fixed for the life of one film.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <svg className="film-converge" aria-hidden="true">
      <g className="film-glow">
        {streaks.map((s, i) => (
          <path
            key={i}
            className="film-streak"
            d={s.d}
            strokeDasharray={`26 ${Math.ceil(s.length + 60)}`}
            style={{ "--from": 26, "--to": -Math.ceil(s.length), animationDelay: `${s.delay}ms`, animationDuration: `${s.duration}ms` } as CSSProperties}
          />
        ))}
      </g>
      {home &&
        [380, 560, 760].map((delay) => (
          <circle key={delay} className="film-ring" cx={home.x} cy={home.y} r={10} style={{ animationDelay: `${delay}ms` }} />
        ))}
    </svg>
  );
}

/** A quadratic curve's length, by walking it — exact enough for a dash. */
function curveLength(a: Point, c: Point, b: Point): number {
  let length = 0;
  let prev = a;
  for (let i = 1; i <= 24; i++) {
    const t = i / 24;
    const u = 1 - t;
    const next = { x: u * u * a.x + 2 * u * t * c.x + t * t * b.x, y: u * u * a.y + 2 * u * t * c.y + t * t * b.y };
    length += Math.hypot(next.x - prev.x, next.y - prev.y);
    prev = next;
  }
  return length;
}

/**
 * The mark, drawn where everything converged: the tile's outline, then the `#` stroke by
 * stroke — the two uprights, the two bars — then the tile takes its matte fill. `still`
 * shows it finished, for a launch that skips the film.
 */
export function Mark({ at, still }: { at: number; still?: boolean }) {
  const stroke = (delay: number, duration: number) =>
    ({ animationDelay: `${at + delay}ms`, animationDuration: `${duration}ms` }) as CSSProperties;
  return (
    <svg className={"film-mark" + (still ? " is-still" : "")} viewBox="0 0 48 48" aria-hidden="true">
      <rect className="film-mark-fill" style={stroke(560, 500)} x="2" y="2" width="44" height="44" rx="10" />
      <path className="film-mark-tile" style={stroke(0, 520)} pathLength={1}
        d="M12 2 H36 A10 10 0 0 1 46 12 V36 A10 10 0 0 1 36 46 H12 A10 10 0 0 1 2 36 V12 A10 10 0 0 1 12 2 Z" />
      <g transform="translate(12 12)">
        <path className="film-mark-line" style={stroke(180, 300)} d="M9.4 4.2 L7.2 19.8" pathLength={1} />
        <path className="film-mark-line" style={stroke(260, 300)} d="M16.8 4.2 L14.6 19.8" pathLength={1} />
        <path className="film-mark-line" style={stroke(340, 300)} d="M4.6 9.6 H19.4" pathLength={1} />
        <path className="film-mark-line" style={stroke(420, 300)} d="M3.8 14.4 H18.6" pathLength={1} />
      </g>
    </svg>
  );
}
