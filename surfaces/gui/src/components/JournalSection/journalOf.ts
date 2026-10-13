import type { Journal } from "../../api";

/** What each view holds, for its tooltip. "Dreams" and "Recaps" are this feature's own words,
 *  and a tab named for a metaphor needs to say what is behind it before it is clicked. */
export const TAB_HINT: Record<Tab, string> = {
  core: "The memory carried into every conversation",
  daily: "Turns that fell out of long conversations, kept as they were said",
  recaps: "Summaries of those turns, filed beside the words they came from",
  dreams: "Notes from the nightly consolidation: what it merged, learned or corrected",
  learned: "Changes a review made to a skill or a remembered fact",
  patterns: "What reviews noticed about how the work goes",
};

// The memory *timeline*, beside the flat list of memories.
//
// Memories are things known about a topic. This is what was learned and when: a
// conversation that outgrows its context window leaves the turns that fell out
// (Daily — what was actually said) plus the model's summary of them (Recaps), and a
// nightly pass distills the *evidence* into the core memory carried into every
// future turn. Both tabs exist so a wrong entry in the core can be checked against
// the words it came from rather than against another summary.
//
// The core is editable here on purpose. It is loaded into every conversation, so a
// wrong entry keeps being wrong until someone corrects it — and correcting a
// markdown file should not require arguing with an assistant about what it thinks
// it knows.
//
// Patterns are the other axis. Every other tab here records what *happened*; that
// one records what it meant — the walls the work keeps hitting, and the changes
// that were tried and rejected. It is the only view that survives an undo, which is
// exactly why it is worth showing: "Learned" can go empty after you put a pass back,
// and the reasoning that produced it should not vanish with it.

export type Tab = "core" | "daily" | "recaps" | "dreams" | "learned" | "patterns";

export const list = <T,>(v: T[] | undefined): T[] => (Array.isArray(v) ? v : []);

/** The journal with every list present. A core older than one of its fields sends the payload
 *  without it, and `j.dailies.length` on that payload unmounted the whole Brain page. A switch
 *  the payload doesn't mention is not reported as off. */
export function journalOf(raw: Partial<Journal> | null | undefined): Journal | null {
  if (!raw || typeof raw !== "object") return null;
  return {
    core: typeof raw.core === "string" ? raw.core : "",
    core_entries: Number(raw.core_entries) || 0,
    dailies: list(raw.dailies),
    recaps: list(raw.recaps),
    dreams: list(raw.dreams),
    evolutions: list(raw.evolutions),
    nightly_enabled: raw.nightly_enabled !== false,
    evolution_enabled: raw.evolution_enabled !== false,
    can_undo: raw.can_undo === true,
    wiki: {
      patterns: list(raw.wiki?.patterns),
      impacts: list(raw.wiki?.impacts),
      log: raw.wiki?.log ?? "",
    },
  };
}
