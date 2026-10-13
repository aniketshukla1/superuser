import { TASKS, type Task } from "./work";

// The launch film's script: when each beat lands, and where everything sits.
//
// The owner's brief (2026-10-13), as a title sequence: in the dark a command types itself; screens of
// Superuser at work fly out of the centre while code runs down both edges; the machine
// locks onto what is getting done; one finished task is captured and stamped; then the
// mark draws itself and the name rolls in, glowing, on matte black.
//
// Seeded, so it opens the same way every launch — a title that reshuffles itself reads
// as noise rather than as a title.

export type Cut = "full" | "short";

export interface Acts {
  /** The command types itself in the dark (negative: not in this cut). */
  command: number;
  /** Screens start arriving out of the dark, and the terminals wake. */
  feeds: number;
  /** The machine starts locking onto work. */
  search: number;
  /** Its last lock. */
  searchEnd: number;
  /** One finished task comes forward and is captured. */
  capture: number;
  /** A scan crosses the screen; everything falls back. */
  sweep: number;
  /** Every lock's line runs home. */
  converge: number;
  /** The mark draws itself. */
  mark: number;
  /** The name rolls in, letter by letter. */
  title: number;
  /** The name glows. */
  glow: number;
  /** The tagline, word by word. */
  tagline: number;
}

/** The first launch plays it all; later launches get a five-second cut. */
export const ACTS: Record<Cut, Acts> = {
  full: { command: 80, feeds: 950, search: 1900, searchEnd: 6250, capture: 6500, sweep: 7900, converge: 7980, mark: 8350, title: 8900, glow: 9900, tagline: 10050 },
  short: { command: -1, feeds: 60, search: 380, searchEnd: 1700, capture: 1850, sweep: 2850, converge: 2900, mark: 3200, title: 3500, glow: 4200, tagline: 4250 },
};

/** How long each cut runs before the app may take over, in ms. */
export const FILM_MS: Record<Cut, number> = { full: 11400, short: 5000 };

/** How many times the machine locks, per cut. */
const LOCKS: Record<Cut, number> = { full: 11, short: 4 };

/** Depths of the planes the camera flies through, farthest first, in px. */
export const PLANES = [-3400, -2600, -1900, -1300, -800, -340] as const;
const FEEDS = 30;

export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface FeedPlan {
  key: string;
  task: Task;
  seed: number;
  plane: number;
  /** Place on its plane, as a fraction of the visible half-width/height, -1…1. */
  sx: number;
  sy: number;
  /** When it switches on, in ms from the film mounting. */
  appear: number;
  stamp: string;
}

/**
 * Thirty screens on six planes of five, spread by the golden angle so no two crowd one
 * corner. The near planes keep the middle clear — the captured task and the name come
 * out of it — and every plane's screens switch on across the hunt, not all at once.
 */
export function feedPlan(cut: Cut, seed: number): FeedPlan[] {
  const r = rng(seed);
  const a = ACTS[cut];
  const span = (a.searchEnd - a.feeds) * 0.62;
  const slots = Math.ceil(FEEDS / PLANES.length);
  return Array.from({ length: FEEDS }, (_, i) => {
    const plane = i % PLANES.length;
    const slot = Math.floor(i / PLANES.length);
    const angle = i * 2.39996 + r() * 0.4;
    const inner = plane >= 4 ? 0.46 : plane === 3 ? 0.34 : 0.14;
    const reach = inner + (1 - inner) * (0.25 + r() * 0.75);
    return {
      key: `feed-${i}`,
      task: TASKS[(i * 7 + slot) % TASKS.length],
      seed: seed + i * 977,
      plane,
      sx: Math.cos(angle) * reach,
      sy: Math.sin(angle) * reach * 0.82,
      appear: a.feeds + (slot * span) / slots + (PLANES.length - 1 - plane) * 40 + r() * 140,
      stamp: `${String(10 + Math.floor(r() * 49)).padStart(2, "0")}:${String(Math.floor(r() * 60)).padStart(2, "0")}`,
    };
  });
}

/** When the machine locks, evenly across the hunt. */
export function lockTimes(cut: Cut): number[] {
  const a = ACTS[cut];
  const n = LOCKS[cut];
  return Array.from({ length: n }, (_, i) => a.search + ((a.searchEnd - a.search) * i) / (n - 1));
}

/** Characters a rolling letter passes through before it lands. */
export const GLYPHS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#$%&*+<>/=?";
