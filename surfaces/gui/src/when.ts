// When something happened, said one way everywhere.
//
// Seven relative-time helpers had grown across the app, each correct and none agreeing: "11d
// ago" on the profile, "9 days ago" under a message, "5m ago" beside a Slack message, "2h ago"
// in the provider list — and past a week some kept counting ("429 days ago") while others
// switched to a date. Beside them sat four absolute formats: "14 Sept 2026, 12:30" on a run,
// "5 Aug" on a memory, "16:26" in the activity log, an ISO stamp inside a memory body. The
// same moment could be written three ways on one screen.
//
// One vocabulary now:
//   • a recent moment is an age — "just now", "5 minutes ago", "3 hours ago", "yesterday",
//     "4 days ago" — because for something that just happened, how long ago is the question;
//   • past a week it is a date — "14 Sep", with the year only when it is not this one;
//   • a moment with a time of day that matters (a run, a schedule) is "today at 7:00 am",
//     "tomorrow at 7:00 am", "14 Sep at 7:00 am";
//   • the exact stamp goes in a tooltip.
// Month names and the 12/24-hour clock follow the system locale rather than a choice made
// here, so the app reads the way the rest of the Mac does.

/** Epoch SECONDS (what the server stamps), an ISO string, or a Date. */
export type At = number | string | Date;

const DAY_MS = 86_400_000;

/** Milliseconds for any accepted input, or null when it cannot be read. */
export function toMillis(at: At | null | undefined): number | null {
  if (at === null || at === undefined || at === "") return null;
  if (at instanceof Date) return Number.isNaN(at.getTime()) ? null : at.getTime();
  if (typeof at === "number") return Number.isFinite(at) && at > 0 ? at * 1000 : null;
  const n = Number(at);
  if (Number.isFinite(n) && n > 0) return n * 1000; // a numeric string is still epoch seconds
  // "2026-07-01 08:00:00" — SQLite's shape. Chromium parses it; WebKit, which is what the
  // desktop app runs in, returns NaN. With a `T` both read it the same way.
  const iso = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}/.test(at) ? at.replace(" ", "T") : at;
  const t = Date.parse(iso);
  return Number.isNaN(t) ? null : t;
}

const startOfDay = (ms: number) => {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
};

/** Calendar days from `now`'s day to `ms`'s day: 0 today, -1 yesterday, 1 tomorrow. */
function dayOffset(ms: number, now: number): number {
  return Math.round((startOfDay(ms) - startOfDay(now)) / DAY_MS);
}

// The formatters below work in milliseconds internally. The exported ones take `At`, whose bare
// number means epoch SECONDS — so an exported formatter handed a millisecond value lands forty
// thousand years out. These two exist so nothing inside this file can make that mistake.
const dateOf = (ms: number, now: number) => {
  const sameYear = new Date(ms).getFullYear() === new Date(now).getFullYear();
  return new Date(ms).toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    ...(sameYear ? {} : { year: "numeric" }),
  });
};
const clockOf = (ms: number) => new Date(ms).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });

/** "14 Sep", or "14 Sep 2025" when it is not this year. */
export function shortDate(at: At | null | undefined, now = Date.now()): string {
  const ms = toMillis(at);
  return ms === null ? "" : dateOf(ms, now);
}

/** "7:00 am" (or "07:00", as the system clock reads). */
export function clock(at: At | null | undefined): string {
  const ms = toMillis(at);
  return ms === null ? "" : clockOf(ms);
}

/** An age for a recent moment, a date for an older one. */
export function timeAgo(at: At | null | undefined, now = Date.now()): string {
  const ms = toMillis(at);
  if (ms === null) return "";
  const secs = Math.floor((now - ms) / 1000);
  // A stamp slightly in the future is clock skew between two processes, not a prophecy.
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins} minute${mins === 1 ? "" : "s"} ago`;
  const days = -dayOffset(ms, now);
  const hrs = Math.floor(mins / 60);
  // Hours only within the same calendar day: "20 hours ago" for last night reads worse than
  // "yesterday", which is what the person would say.
  if (days === 0) return `${hrs} hour${hrs === 1 ? "" : "s"} ago`;
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  return dateOf(ms, now);
}

/** "today at 7:00 am", "tomorrow at 7:00 am", "14 Sep at 7:00 am". For moments whose time
 *  of day matters — a run that happened, a run that will. */
export function dayAndTime(at: At | null | undefined, now = Date.now()): string {
  const ms = toMillis(at);
  if (ms === null) return "";
  const off = dayOffset(ms, now);
  const day = off === 0 ? "today" : off === -1 ? "yesterday" : off === 1 ? "tomorrow" : dateOf(ms, now);
  return `${day} at ${clockOf(ms)}`;
}

/** The exact moment, for a tooltip. */
export function exactTime(at: At | null | undefined): string {
  const ms = toMillis(at);
  if (ms === null) return "";
  return new Date(ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
