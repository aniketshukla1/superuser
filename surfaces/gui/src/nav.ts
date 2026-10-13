// The app's non-session surfaces, as data.
//
// The sidebar renders these twice — as rows in the nav block and as items in the
// account menu — and before this list existed the two were written out by hand in
// both places, so they drifted: Brain didn't exist, Inbox and Connectors were only
// reachable from the menu, and adding a surface meant editing four files. Icons,
// labels and order live here; the sidebar owns only how a row looks and App owns
// what clicking one does.
//
// Deliberately data-only (no JSX, no imports from components beyond the icon
// name type) so it can be imported from anywhere without pulling React in.
import type { IconName } from "./components/Icon";

/** A surface the sidebar can navigate to. Mirrors App's `surface` union, minus
 *  the ones you never *navigate* to (`session` is where you already are;
 *  `persona` and `settings` are reached from their own affordances). */
export type AppSurface = "brain" | "scheduled" | "inbox" | "integrations" | "audit";

export interface NavEntry {
  id: AppSurface;
  label: string;
  icon: IconName;
  /** Stable hook for tests and analytics; travels with the entry so it stays
   *  attached to the feature wherever the entry moves in the order. */
  testid: string;
  /** A one-line description of the surface, used as the row's tooltip. */
  hint: string;
  /** `rail` = a row in the sidebar's nav block. Every entry is also in the
   *  account menu, so nothing becomes unreachable if the rail is trimmed. */
  rail: boolean;
  /** Where in the rail it sits.
   *
   *  `primary` is always on screen. `secondary` lives behind the "More" row, which
   *  is a disclosure and not a deletion — the point is that the four or five things
   *  competing for the eye become two, without anything moving out of reach.
   *
   *  What earns `primary`: something you act ON (Inbox holds decisions waiting for
   *  you and carries a count) or something the product is FOR (Brain is the memory
   *  the whole thing is built around). Configuration and records are secondary —
   *  you go to them deliberately, so they cost a click and save a row. */
  tier: "primary" | "secondary";
}

export const NAV: NavEntry[] = [
  {
    id: "brain",
    label: "Brain",
    icon: "diamond",
    testid: "nav-brain",
    hint: "Everything your assistants have learned — memories, the timeline, and what they know by topic",
    rail: true,
    tier: "primary",
  },
  {
    id: "scheduled",
    label: "Automations",
    icon: "clock",
    testid: "nav-automations",
    hint: "Work that runs on a schedule",
    rail: true,
    tier: "secondary",
  },
  {
    id: "inbox",
    label: "Inbox",
    icon: "inbox",
    testid: "nav-inbox",
    hint: "Actions waiting on your decision",
    rail: true,
    tier: "primary",
  },
  {
    id: "integrations",
    label: "Connectors",
    icon: "plug",
    testid: "nav-connectors",
    hint: "The apps your assistants can read from and act on",
    rail: true,
    tier: "secondary",
  },
  {
    id: "audit",
    // Renamed from "Activity". The hint always claimed this was "a record of
    // everything that left this machine", which the action log it showed could not
    // back up — nothing recorded outbound traffic at all. Now that one does, the
    // label may as well be the promise the product leads with. The surface id is
    // unchanged, so `#/audit` and every existing link still work.
    label: "Privacy",
    icon: "shield",
    testid: "nav-activity",
    hint: "What left this machine, where to, and why",
    rail: true,
    tier: "secondary",
  },
];

/** The entries shown as rows in the sidebar's nav block, in order. */
export const RAIL_NAV = NAV.filter((n) => n.rail);
/** Always on screen. */
export const RAIL_PRIMARY = RAIL_NAV.filter((n) => n.tier === "primary");
/** Behind the "More" disclosure. */
export const RAIL_SECONDARY = RAIL_NAV.filter((n) => n.tier === "secondary");
