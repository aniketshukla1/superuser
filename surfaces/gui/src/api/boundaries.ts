import { fetch, httpBase, jsonOf } from "./neverRejectingJson";

// -- Boundaries ------------------------------------------------------------------

export interface BoundaryRule {
  /** What the user calls it. Optional — the expression is what's enforced. */
  name?: string;
  /** The rule, e.g. `intent == "send"`. */
  when: string;
  /** The preset it came from, if any. */
  preset?: string;
}

export interface Boundaries {
  /** enforce: a refused action doesn't run. record: it's written down as refused and runs. */
  mode: "enforce" | "record";
  deny: BoundaryRule[];
  /** Anything matching one of these is permitted. ["true"] permits everything not denied. */
  allow: string[];
}

export interface BoundaryPreset {
  id: string;
  name: string;
  when: string;
  /** What switching it on takes away that the name doesn't say. */
  cost: string;
}

export interface BoundaryField {
  field: string;
  means: string;
}

export interface BoundaryProblem {
  list: "deny" | "allow" | "";
  index: number;
  message: string;
}

export interface BoundariesState {
  boundaries: Boundaries;
  /** Set when the saved file can't be read — every action is refused until it's fixed or reset. */
  problem: string | null;
  problems: BoundaryProblem[];
  presets: BoundaryPreset[];
  fields: BoundaryField[];
}

export async function getBoundaries(): Promise<BoundariesState | null> {
  const res = await fetch(`${httpBase()}/v1/boundaries`);
  const body = await jsonOf(res);
  if (!body.boundaries) return null;
  return {
    boundaries: body.boundaries as Boundaries,
    problem: (body.problem as string | null) ?? null,
    problems: Array.isArray(body.problems) ? body.problems : [],
    presets: Array.isArray(body.presets) ? body.presets : [],
    fields: Array.isArray(body.fields) ? body.fields : [],
  };
}

export async function saveBoundaries(
  b: Boundaries,
): Promise<{ ok: boolean; boundaries?: Boundaries; problems?: BoundaryProblem[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/boundaries`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(b),
  });
  return jsonOf(res) as Promise<{ ok: boolean; boundaries?: Boundaries; problems?: BoundaryProblem[]; error?: string }>;
}

/** Is one rule usable? `problem` is null when it is. */
export async function checkBoundaryRule(when: string): Promise<{ ok: boolean; problem: string | null }> {
  const res = await fetch(`${httpBase()}/v1/boundaries/check`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ when }),
  });
  const body = await jsonOf(res);
  return { ok: body.ok === true, problem: (body.problem as string | null) ?? null };
}

/** One recorded action a candidate set would decide differently. */
export interface BoundaryChange {
  at: number;
  session: string;
  persona: string;
  tool: string;
  intent: string;
  what: string;
  was: "allowed" | "refused";
  would: "allowed" | "refused";
  rule?: string;
  rule_name?: string;
}

export interface BoundaryReport {
  scanned: number;
  would_refuse: number;
  would_allow: number;
  unchanged: number;
  changes: BoundaryChange[];
}

/** Replay a candidate set over the trail. Never saves. */
export async function testBoundaries(
  b: Boundaries,
): Promise<{ ok: boolean; report?: BoundaryReport; problems?: BoundaryProblem[]; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/boundaries/test`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ boundaries: b }),
  });
  return jsonOf(res) as Promise<{ ok: boolean; report?: BoundaryReport; problems?: BoundaryProblem[]; error?: string }>;
}

export async function resetBoundaries(): Promise<{ ok: boolean; boundaries?: Boundaries; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/boundaries/reset`, { method: "POST" });
  return jsonOf(res) as Promise<{ ok: boolean; boundaries?: Boundaries; error?: string }>;
}

/** What every assistant is told on every turn. Empty clears them. */
export async function setStandingInstructions(
  text: string,
): Promise<{ ok: boolean; standing_instructions?: string; max?: number; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/settings/standing-instructions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });
  return jsonOf(res) as Promise<{ ok: boolean; standing_instructions?: string; max?: number; error?: string }>;
}

/** How long a model may send nothing before its turn is ended. 0 switches a watchdog off. */
export async function setStallLimits(next: {
  stall_seconds?: number;
  stall_seconds_local?: number;
}): Promise<{ ok: boolean; stall_seconds?: number; stall_seconds_local?: number; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/settings/stall`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(next),
  });
  return jsonOf(res) as Promise<{ ok: boolean; stall_seconds?: number; stall_seconds_local?: number; error?: string }>;
}
