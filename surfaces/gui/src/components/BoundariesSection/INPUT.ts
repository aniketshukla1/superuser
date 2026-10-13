import type { Boundaries } from "../../api";

// Settings ▸ Boundaries — what your assistants may never do, in rules you can read.
//
// The approval settings decide HOW an action happens (ask first, run,
// park it); a boundary is the user's own sentence about what must not happen at all,
// checked by the gateway before every tool call. Three things on this page carry the
// weight:
//
//   * presets that say what they COST — a checkbox that stops "submit" also stops Enter
//     everywhere, and a person should know that before switching it on;
//   * "Record it and let it through", so a rule can be watched against real work before
//     it refuses any ("a governance feature nobody dares switch on is not one");
//   * "Test first", which replays the draft over actions already on the trail and says
//     what it would have changed — a boundary is otherwise written blind.
//
// Nothing invalid can be saved: the server checks every rule, and a rule that can't be
// read would refuse whatever it might cover. The page checks each rule as it is typed so
// that failure is met here, not in the middle of someone's task.

export const INPUT = "w-full px-3 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
export const CODE_INPUT =
  "w-full px-3 py-1.5 rounded-lg border border-line bg-paper font-mono text-sm text-ink outline-none focus:border-accent";
export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export const EMPTY: Boundaries = { mode: "enforce", deny: [], allow: ["true"] };

export const EXAMPLES: { when: string; says: string }[] = [
  { when: `intent == "send"`, says: "Never send a message." },
  { when: `file.name == ".env"`, says: "Never read or write a file called .env." },
  { when: `intent == "run_command" && contains(command, "git push")`, says: "Never push with git." },
  { when: `unattended && intent == "write_tool"`, says: "Automations never change anything in connected apps." },
];

/** Compared by meaning: the server omits an empty name or preset, and a name typed and
 *  deleted again is not a change anyone made. */
export const normal = (b: Boundaries | null) =>
  b && {
    mode: b.mode,
    deny: b.deny.map((r) => ({ name: (r.name ?? "").trim(), when: r.when.trim(), preset: r.preset ?? "" })),
    allow: b.allow,
  };
export const same = (a: Boundaries | null, b: Boundaries | null) => JSON.stringify(normal(a)) === JSON.stringify(normal(b));

export const lines = (text: string) =>
  text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);

export const when = (t: number) => {
  const d = new Date(t * 1000);
  return d.toLocaleString(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
};
