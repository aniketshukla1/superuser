import { fetch, httpBase } from "./neverRejectingJson";
import type { Journal } from "./memoryJournal";

// ---- The wiki layer ------------------------------------------------------------
// The journal above records what happened. This records what it means: how the
// assistant's own work goes wrong, and what fixed it. It is the one store an undo
// does not touch — putting a skill back is useful, and forgetting *why* it was put
// back is how the same rejected change gets written again next week.

/** One failure mode or working strategy, distilled from past conversations. */
export interface WikiPattern {
  /** Slug and filename. Stable, so a second sighting reinforces this page. */
  id: string;
  title: string;
  kind: "failure" | "strategy";
  /** How many review passes have seen this. Recurrence is what earns a skill change. */
  seen: number;
  first_seen: string;
  updated: string;
  body: string;
}

/** One change a review proposed, and what became of it. */
export interface WikiImpact {
  at: string;
  /** The skill it changed, or the file it wrote. */
  target: string;
  patterns: string[];
  /** `undone` means the user rejected it — kept on purpose, so it is not retried. */
  verdict: "accepted" | "undone";
  snapshot: string;
  note: string;
}

export interface Wiki {
  patterns: WikiPattern[];
  impacts: WikiImpact[];
  /** The raw evolution log, one line per pass that learned or changed something. */
  log: string;
}

export interface DreamResult {
  status: "consolidated" | "skipped" | "failed";
  entries?: number;
  diary?: string;
  reason?: string;
}

export async function getJournal(): Promise<Journal | null> {
  const res = await fetch(`${httpBase()}/v1/journal`);
  return res.ok ? res.json() : null;
}

/** Consolidate the last `days` of daily memory into the core memory, now. */
export async function runDream(days = 3): Promise<DreamResult> {
  const res = await fetch(`${httpBase()}/v1/journal/dream`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ days }),
  });
  if (!res.ok) return { status: "failed", reason: `HTTP ${res.status}` };
  return res.json();
}
