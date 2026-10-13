import type { Facet, FacetClass, ProfileForm } from "../../api";
import { timeAgo } from "../../when";

// The profile: how you want to be worked with.
//
// The other Brain sections show what is *known*. This one shows what is being
// *done differently because of you* — which is the one kind of memory that acts on
// its own, so it's also the one that has to be inspectable. Three things are
// deliberately on screen at once:
//
//   what it acts on   — Active facets, exactly what reaches the system prompt
//   what it's weighing — Provisional ones, tracked and not acted on
//   why it thinks so   — the words that put each one there
//
// A store that quietly forms opinions about someone and only shows them the
// conclusions is the version of this people find creepy. Showing the evidence is
// also what makes Pin and Forget mean something: you can see what you're keeping.

export const CLASSES: { key: FacetClass; label: string; blurb: string }[] = [
  { key: "identity", label: "Identity", blurb: "Who you are and what you're responsible for." },
  { key: "style", label: "Style", blurb: "How to talk to you — length, tone, how much to explain." },
  { key: "veto", label: "Do not", blurb: "Things to never do. These fade slowly." },
  { key: "tooling", label: "Tooling", blurb: "What you work with." },
  { key: "goal", label: "Goals", blurb: "What you're working towards beyond one conversation." },
  { key: "channel", label: "Channels", blurb: "Where and how to reach you." },
];

export const CUE: Record<Facet["cue"], string> = {
  explicit: "you said so",
  structural: "from your profile",
  behavioral: "inferred",
};

export const relative = (t: number) => timeAgo(t);

/** The line of evidence worth showing under a belief — or null when there is none.
 *
 *  Two kinds of "evidence" restated the row instead of supporting it. A fact saved by a tool
 *  call carries its own storage record ("style | questions | Do not ask any questions |
 *  explicit"), which printed the value a second time with the machine's punctuation around it.
 *  And a profile field's evidence is its field name ("Your profile ▸ Name"), which the cue
 *  beside it already says. What is left is the thing the line is for: the words someone
 *  actually said. */
export function evidenceLine(facet: Pick<Facet, "evidence" | "value" | "from_form">): string | null {
  const e = (facet.evidence || "").trim();
  if (!e || facet.from_form) return null;
  if (/^[\w-]+ \| [^|]+ \| [^|]+ \| (explicit|structural|behavioral)$/.test(e)) return null;
  if (e === facet.value) return null;
  return e;
}

/** "Your profile ▸ Name" → "Name": which field a form-derived belief came from. */
export const fieldOf = (evidence: string) => evidence.split("▸").pop()?.trim() || "";

export const FIELDS: { key: keyof ProfileForm; label: string; hint: string; area?: boolean }[] = [
  { key: "name", label: "Name", hint: "" },
  { key: "call_me", label: "Call me", hint: "Used instead of your name, if you'd rather." },
  { key: "role", label: "Role", hint: "" },
  { key: "about", label: "About", hint: "Anything worth knowing up front.", area: true },
  { key: "tone", label: "Preferred tone", hint: "e.g. concise, or warm and detailed." },
  { key: "interests", label: "Interests", hint: "" },
  { key: "likes", label: "Likes", hint: "" },
  { key: "dislikes", label: "Never do", hint: "Kept as a rule, not a preference." },
  { key: "goals", label: "Goals", hint: "", area: true },
];
