import type { ComputerStatus } from "../../api";

// Settings ▸ Browser — the assistants' own browser.
//
// One real Chrome per assistant, kept apart from the user's own browser and from each
// other: its own profile, its own sign-ins, and a proxy that keeps it off this machine's
// network. Off until someone switches it on, because it starts a browser and because it
// can act on sites it is signed in to.
//
// Three things live here that nowhere else can: the switch; signing an assistant in to a
// site before it needs one (the wheel is yours before the page loads); and Sign out of
// everything, which deletes the profile and every kept screen — confirmed first, because
// there is no undo.

export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";
export const INPUT = "flex-1 min-w-0 px-3 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";

export function stateLine(c: ComputerStatus): { text: string; tone: "ink" | "muted" | "accent" } {
  if (c.control.holder === "user") return { text: "You have control", tone: "accent" };
  if (c.control.secret_wanted) return { text: `Waiting for you to enter ${c.control.secret_wanted}`, tone: "accent" };
  if (c.control.requested) return { text: `Needs you: ${c.control.reason || "take control to continue"}`, tone: "accent" };
  if (!c.running) return { text: c.has_profile ? "Closed — its sign-ins are kept" : "Not started", tone: "muted" };
  return { text: c.title ? `On “${c.title}”` : c.url || "Open", tone: "ink" };
}
