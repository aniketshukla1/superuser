import type { ReactNode } from "react";
import { AMBER, DIM, FAINT, ICE, INK, MINT, ROSE, VIOLET, at, type R } from "./artDocs";

// The drawn kinds of work — the reports the owner asked to see "visually appealing":
// bars that grow, a line that draws with its area filling under it, a donut, a week of
// meetings, a forecast. Same 270 × 150 units as the written kinds.

/** A revenue chart: a bar a month, this one picked out. */
export function bars(r: R, t: number): ReactNode {
  const n = 10;
  return (
    <g>
      <text x={6} y={11} fontSize={7} fill={DIM}>REVENUE BY MONTH</text>
      <text x={264} y={12} fontSize={10} fill={INK} textAnchor="end">$4.2M</text>
      {[40, 80, 120].map((y) => <line key={y} x1={6} x2={264} y1={y} y2={y} stroke={FAINT} />)}
      {Array.from({ length: n }, (_, i) => {
        const h = 26 + r() * 88 + i * 2.5;
        return (
          <rect key={i} className="film-rise" style={at(t, 120 + i * 60)}
            x={10 + i * 25.5} y={140 - h} width={17} height={h} rx={2} fill={i === n - 2 ? MINT : ICE} opacity={i === n - 2 ? 1 : 0.72} />
        );
      })}
      <line x1={6} x2={264} y1={140} y2={140} stroke={DIM} />
    </g>
  );
}

/** A metric over time: the line draws itself, the area fills under it. */
export function line(r: R, t: number, id: number): ReactNode {
  let y = 70 + r() * 30;
  const pts = Array.from({ length: 16 }, (_, i) => {
    y = Math.max(28, Math.min(126, y + (r() - 0.55) * 26));
    return [8 + i * 17, y] as const;
  });
  const d = pts.map(([x, py], i) => `${i ? "L" : "M"}${x.toFixed(1)},${py.toFixed(1)}`).join(" ");
  const [ex, ey] = pts[pts.length - 1];
  return (
    <g>
      <defs>
        <linearGradient id={`film-fill-${id}`} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stopColor={ICE} stopOpacity={0.32} />
          <stop offset="1" stopColor={ICE} stopOpacity={0} />
        </linearGradient>
      </defs>
      {[30, 64, 98, 132].map((gy) => <line key={gy} x1={6} x2={264} y1={gy} y2={gy} stroke={FAINT} />)}
      <path className="film-wash" style={at(t, 700)} d={`${d} L${ex},140 L8,140 Z`} fill={`url(#film-fill-${id})`} />
      <path d={pts.map(([x, py], i) => `${i ? "L" : "M"}${x},${(py + 14).toFixed(1)}`).join(" ")} fill="none" stroke={DIM} strokeDasharray="3 3" />
      <path className="film-draw" style={at(t, 100)} d={d} fill="none" stroke={ICE} strokeWidth={1.6} pathLength={1} />
      <circle className="film-pop" style={at(t, 1050)} cx={ex} cy={ey} r={3.2} fill={MINT} />
      <text x={6} y={14} fontSize={7} fill={DIM}>P99 LATENCY · 24H</text>
    </g>
  );
}

/** Where the customers went: a donut, a legend, the number that matters in the middle. */
export function donut(_r: R, t: number): ReactNode {
  const parts: [number, string, string][] = [[46, ICE, "Pro"], [27, MINT, "Team"], [17, VIOLET, "Starter"], [10, AMBER, "Trial"]];
  let off = 0;
  return (
    <g>
      <circle cx={66} cy={76} r={44} fill="none" stroke={FAINT} strokeWidth={14} />
      {parts.map(([v, c], i) => {
        const el = (
          <circle key={i} className="film-arc" style={at(t, 120 + i * 180)} cx={66} cy={76} r={44} fill="none" stroke={c} strokeWidth={14}
            pathLength={100} strokeDasharray={`${v - 1.2} ${100 - v + 1.2}`} strokeDashoffset={-off} transform="rotate(-90 66 76)" />
        );
        off += v;
        return el;
      })}
      <text x={66} y={78} fontSize={15} fill={INK} textAnchor="middle">2.1%</text>
      <text x={66} y={92} fontSize={6.5} fill={DIM} textAnchor="middle">CHURN</text>
      {parts.map(([v, c, name], i) => (
        <g key={name} className="film-pop" style={at(t, 300 + i * 160)}>
          <rect x={140} y={36 + i * 24} width={8} height={8} rx={2} fill={c} />
          <text x={156} y={43 + i * 24} fontSize={8} fill={INK}>{name}</text>
          <text x={262} y={43 + i * 24} fontSize={8} fill={DIM} textAnchor="end">{v}%</text>
        </g>
      ))}
    </g>
  );
}

/** A week of meetings, with the two that need a decision marked. */
export function calendar(r: R, t: number): ReactNode {
  const days = ["MON", "TUE", "WED", "THU", "FRI"];
  const tones = [ICE, VIOLET, MINT, ICE, AMBER, ROSE];
  return (
    <g>
      {days.map((d, i) => <text key={d} x={8 + i * 52} y={10} fontSize={6.5} fill={DIM}>{d}</text>)}
      {[30, 60, 90, 120].map((y) => <line key={y} x1={6} x2={264} y1={y} y2={y} stroke={FAINT} />)}
      {days.map((_, i) =>
        Array.from({ length: 2 + Math.floor(r() * 2) }, (_, k) => {
          const y = 18 + k * 42 + r() * 12;
          return (
            <rect key={`${i}-${k}`} className="film-pop" style={at(t, 120 + (i * 3 + k) * 60)}
              x={6 + i * 52} y={y} width={46} height={18 + r() * 14} rx={3} fill={tones[(i + k) % tones.length]} fillOpacity={0.75} />
          );
        }),
      )}
    </g>
  );
}

/** A forecast: the headline number, its trend, and what it rests on. */
export function kpi(r: R, t: number): ReactNode {
  let y = 60;
  const d = Array.from({ length: 12 }, (_, i) => {
    y = Math.max(34, Math.min(76, y - 3 + (r() - 0.35) * 10));
    return `${i ? "L" : "M"}${150 + i * 10},${y.toFixed(1)}`;
  }).join(" ");
  const tiles: [string, string][] = [["RUNWAY", "19 mo"], ["BURN", "$210k"], ["MARGIN", "31%"]];
  return (
    <g>
      <text x={6} y={12} fontSize={7} fill={DIM}>CASH FORECAST</text>
      <text className="film-line" style={at(t, 150)} x={6} y={52} fontSize={28} fill={INK}>$4.2M</text>
      <text className="film-line" style={at(t, 450)} x={8} y={70} fontSize={8} fill={MINT}>▲ 18.4% QoQ</text>
      <path className="film-draw" style={at(t, 300)} d={d} fill="none" stroke={MINT} strokeWidth={1.6} pathLength={1} />
      {tiles.map(([k, v], i) => (
        <g key={k} className="film-pop" style={at(t, 600 + i * 150)}>
          <rect x={6 + i * 88} y={92} width={80} height={46} rx={4} fill={FAINT} />
          <text x={14 + i * 88} y={108} fontSize={6.5} fill={DIM}>{k}</text>
          <text x={14 + i * 88} y={128} fontSize={13} fill={INK}>{v}</text>
        </g>
      ))}
    </g>
  );
}
