import type { InboxItem } from "../../api";
import type { IconName } from "../Icon";

/** What kind of thing is waiting, as a word and a mark — it was the lowercase machine key
 *  ("approval") in 11px grey, the least legible text on the card. */
export const KIND: Record<InboxItem["kind"], { label: string; icon: IconName }> = {
  approval: { label: "Needs your approval", icon: "shield" },
  question: { label: "Question", icon: "helpCircle" },
  notification: { label: "Update", icon: "inbox" },
  directory: { label: "Folder access", icon: "folder" },
  plan: { label: "Plan to review", icon: "check" },
};

// One Inbox item, rendered identically in the Inbox list and inline in its own session view
// (answer-in-context). Resolving either place hits the same item id — first responder wins.
// Questions (ask_user) mirror Claude Code's AskUserQuestion: optional quick-reply options + an
// always-available free-text escape, with optional multi-select.

// Shared styles (mock parity — same language as SourcesDrawer/PersonaView).
export const BTN_PRIMARY =
  "ctl bg-accent text-onAccent hover:brightness-105 disabled:opacity-40 disabled:hover:brightness-100";
export const BTN_BORDERED =
  "ctl border border-line bg-paper hover:border-lineStrong";
// §35 approval buttons: blue border for the primary, quiet Deny (matches ApprovalCard).
export const BTN_ACCENT =
  "ctl border border-accent text-accent font-semibold hover:bg-accentSoft";
export const BTN_QUIET = "ctl text-faint hover:text-danger";
export const OPT_BASE =
  "ctl gap-1.5 rounded-full border transition-colors";
export const OPT_OFF = "border-line bg-paper text-ink hover:border-accent hover:bg-accentSoft/50";
export const OPT_ON = "border-accent bg-accentSoft text-accent font-medium";
export const INPUT =
  "flex-1 min-w-0 rounded-lg bg-paper border border-line px-3 py-2 text-md text-ink placeholder:text-faint outline-none focus:border-lineStrong";
