import type { Wiki } from "./wikiLayer";

// ---- Memory journal (the timeline layer) ---------------------------------------
// Memories are things known *about* a topic. The journal is what was learned *when*:
// conversation → dated daily summary → the distilled core carried into every turn.

/** One dated file — a daily summary or a dream diary. */
export interface JournalEntry {
  date: string;
  body: string;
}

export interface Journal {
  /** The distilled core memory, injected into every conversation. */
  core: string;
  core_entries: number;
  /** What was actually said, as it fell out of the context window — the evidence
   *  the nightly consolidation reads. */
  dailies: JournalEntry[];
  /** The model's summary of those same turns. A view of the evidence, not the
   *  thing memory is built from. Absent on a core too old to send it. */
  recaps?: JournalEntry[];
  dreams: JournalEntry[];
  /** What each self-evolution review pass changed. */
  evolutions: JournalEntry[];
  /** False when the nightly consolidation is switched off. */
  nightly_enabled: boolean;
  /** False when self-evolution is switched off. */
  evolution_enabled: boolean;
  /** True when a review pass is on record and can be put back. */
  can_undo: boolean;
  /** What reviews have *learned*, as opposed to what they changed. Absent on a
   *  core built before the wiki layer existed. */
  wiki?: Wiki;
}
