import type { CSSProperties, ReactNode } from "react";

// The written kinds of work: code, mail, a board, a checklist, a ledger, a test run. Each
// draws itself from the moment its screen switches on (`t`), so the screens read as work
// happening rather than as pictures of it. 270 × 150 user units; shapes, not prose — at
// the size a flying screen is seen, a line of text is a bar of colour anyway.

export const INK = "rgba(238,243,248,0.84)";
export const DIM = "rgba(238,243,248,0.36)";
export const FAINT = "rgba(238,243,248,0.1)";
export const ICE = "#7CC4FF";
export const MINT = "#5EE6C2";
export const AMBER = "#F2C14E";
export const VIOLET = "#B39DFF";
export const ROSE = "#FF7AA8";

export type R = () => number;
/** An animation starting `ms` after the screen does. */
export const at = (t: number, ms: number) => ({ animationDelay: `${Math.round(t + ms)}ms` }) as CSSProperties;

const TOKENS = [ICE, MINT, AMBER, VIOLET, INK, INK, DIM];

/** An editor mid-change: numbered lines, tokens in colour, the newest line still typing. */
export function code(r: R, t: number): ReactNode {
  const indents = [0, 1, 2, 2, 1, 2, 3, 3, 2, 1];
  return indents.map((ind, i) => {
    const y = 12 + i * 13.5;
    let x = 26 + ind * 11;
    const n = 2 + Math.floor(r() * 4);
    return (
      <g key={i}>
        <text x={4} y={y + 5} fontSize={7} fill={DIM}>{i + 41}</text>
        {i === 7 && <rect x={22} y={y - 4} width={246} height={13} fill={ICE} opacity={0.09} />}
        {Array.from({ length: n }, (_, k) => {
          const w = 9 + r() * 32;
          const el = (
            <rect key={k} className="film-reach" style={at(t, 120 + i * 70 + k * 22)}
              x={x} y={y} width={w} height={5} rx={1} fill={TOKENS[Math.floor(r() * TOKENS.length)]} />
          );
          x += w + 5;
          return el;
        })}
      </g>
    );
  });
}

/** A reply being drafted, waiting on the person before it goes. */
export function email(r: R, t: number): ReactNode {
  const body = Array.from({ length: 6 }, (_, i) => (i === 5 ? 70 : 150 + r() * 100));
  return (
    <g>
      <text x={6} y={14} fontSize={7} fill={DIM}>TO</text>
      <rect x={30} y={9} width={96} height={6} rx={1} fill={INK} opacity={0.75} />
      <text x={6} y={29} fontSize={7} fill={DIM}>RE</text>
      <rect x={30} y={24} width={160} height={6} rx={1} fill={INK} />
      <line x1={6} x2={264} y1={39} y2={39} stroke={FAINT} />
      {body.map((w, i) => (
        <rect key={i} className="film-reach" style={at(t, 160 + i * 150)} x={6} y={50 + i * 12} width={w} height={5} rx={1} fill={INK} opacity={0.62} />
      ))}
      <g className="film-pop" style={at(t, 1100)}>
        <rect x={146} y={124} width={118} height={17} rx={3} fill="none" stroke={AMBER} />
        <text x={154} y={135} fontSize={7} fill={AMBER}>DRAFT · NEEDS YOUR OK</text>
      </g>
    </g>
  );
}

/** A board: what's next, what's moving, what's done. */
export function kanban(r: R, t: number): ReactNode {
  const cols = [["TO DO", 4, AMBER], ["DOING", 3, ICE], ["DONE", 4, MINT]] as const;
  return cols.map(([name, cards, tag], c) => (
    <g key={name} transform={`translate(${6 + c * 88},0)`}>
      <text x={0} y={12} fontSize={7} fill={DIM}>{name}</text>
      {Array.from({ length: cards }, (_, k) => (
        <g key={k} className="film-pop" style={at(t, 150 + (c * 4 + k) * 70)}>
          <rect x={0} y={20 + k * 31} width={80} height={25} rx={3} fill={FAINT} />
          <rect x={0} y={20 + k * 31} width={3} height={25} fill={tag} />
          <rect x={9} y={27 + k * 31} width={30 + r() * 36} height={4} rx={1} fill={INK} opacity={0.7} />
          <rect x={9} y={35 + k * 31} width={18 + r() * 24} height={3} rx={1} fill={DIM} />
        </g>
      ))}
    </g>
  ));
}

/** A checklist being worked down. */
export function checklist(r: R, t: number): ReactNode {
  return Array.from({ length: 8 }, (_, i) => {
    const y = 10 + i * 17;
    const done = i < 6;
    return (
      <g key={i}>
        <rect x={6} y={y} width={9} height={9} rx={2} fill="none" stroke={done ? MINT : DIM} />
        {done && <path className="film-tick" style={at(t, 200 + i * 160)} d={`M8 ${y + 4.6} l2.2 2.2 l4 -4.6`} fill="none" stroke={MINT} strokeWidth={1.4} pathLength={1} />}
        <rect x={24} y={y + 2.5} width={80 + r() * 120} height={4} rx={1} fill={done ? DIM : INK} opacity={done ? 1 : 0.8} />
      </g>
    );
  });
}

/** Receipts matched to the statement, a running total in the corner. */
export function ledger(r: R, t: number): ReactNode {
  return (
    <g>
      <text x={6} y={11} fontSize={6.5} fill={DIM}>DATE</text>
      <text x={52} y={11} fontSize={6.5} fill={DIM}>ITEM</text>
      <text x={264} y={11} fontSize={6.5} fill={DIM} textAnchor="end">AMOUNT</text>
      {Array.from({ length: 7 }, (_, i) => {
        const y = 26 + i * 17;
        const amt = (40 + r() * 1900).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
        return (
          <g key={i} className="film-pop" style={at(t, 150 + i * 120)}>
            <text x={6} y={y} fontSize={7} fill={DIM}>{`Oct ${String(2 + i * 2).padStart(2, "0")}`}</text>
            <rect x={52} y={y - 5} width={60 + r() * 90} height={4.5} rx={1} fill={INK} opacity={0.6} />
            <text x={252} y={y} fontSize={7} fill={INK} textAnchor="end">${amt}</text>
            <circle cx={260} cy={y - 2.5} r={2.3} fill={i === 5 ? AMBER : MINT} />
          </g>
        );
      })}
    </g>
  );
}

/** A test run, line by line, ending green. */
export function terminal(_r: R, t: number): ReactNode {
  const lines: [string, string][] = [
    ["$ pnpm test", INK],
    ["✓ retry.spec.ts      8 tests", MINT],
    ["✓ auth.spec.ts      31 tests", MINT],
    ["✓ billing.spec.ts   52 tests", MINT],
    ["✓ queue.spec.ts     19 tests", MINT],
    ["✓ export.spec.ts    44 tests", MINT],
    ["Tests  248 passed (2.41s)", INK],
  ];
  return lines.map(([text, fill], i) => (
    <text key={i} className="film-line" style={at(t, 120 + i * 190)} x={6} y={16 + i * 18} fontSize={8} fill={fill}>{text}</text>
  ));
}
