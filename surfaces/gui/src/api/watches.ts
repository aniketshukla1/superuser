import { fetch, httpBase, jsonBody, jsonOf } from "./neverRejectingJson";

// -- Watches: the evaluating background loop ---------------------------------
// An automation runs on a clock. A watch runs when something *changed*, and only
// spends a model call when it did — see `core/subconscious.rs`.

export interface Watch {
  id: string;
  question: string;
  sources: string[];
  enabled: boolean;
  writes: boolean;
  created_at: number;
  last_tick: number | null;
}

export interface WatchActivity {
  id: string;
  watch_id: string;
  question: string;
  at: number;
  decision: "skip" | "act" | "escalate";
  reason: string;
  evidence: string;
  note: string | null;
  ask_id: string | null;
}

export interface WatchesState {
  watches: Watch[];
  activity: WatchActivity[];
  /** False when the whole loop is switched off in Settings. */
  enabled: boolean;
}

export async function getWatches(): Promise<WatchesState> {
  const res = await fetch(`${httpBase()}/v1/watches`);
  if (!res.ok) return { watches: [], activity: [], enabled: false };
  const j = await jsonOf(res);
  return { watches: j.watches ?? [], activity: j.activity ?? [], enabled: j.enabled ?? false };
}

export async function createWatch(question: string, sources: string[]): Promise<Watch | null> {
  const res = await fetch(`${httpBase()}/v1/watches`, jsonBody("POST", { question, sources }));
  const j = await jsonOf(res);
  return j.ok ? j.watch : null;
}

export async function setWatchEnabled(id: string, enabled: boolean): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/watches/${encodeURIComponent(id)}`, jsonBody("PATCH", { enabled }));
  return res.ok && (await jsonOf(res)).ok === true;
}

export async function deleteWatch(id: string): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/watches/${encodeURIComponent(id)}`, { method: "DELETE" });
  return res.ok && (await jsonOf(res)).ok === true;
}

/** One pass now, instead of waiting for the interval. Returns what it did. */
export async function tickWatches(): Promise<{
  evaluated: number;
  quiet: number;
  acted: number;
  escalated: number;
  failed: number;
} | null> {
  const res = await fetch(`${httpBase()}/v1/watches/tick`, { method: "POST" });
  return res.ok ? res.json() : null;
}
