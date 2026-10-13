// An automation's schedule, and the state of one of its runs, in the app's words.
//
// The schedule label is built server-side ("Every day at ~7:00 AM", core/automation.rs) and
// sat on the same line as a next-run time formatted by the browser ("15 Sept 2026, 07:00"):
// a tilde nobody explained, and two clocks in one sentence. The tilde is real — the
// scheduler fires within its polling window, not on the second — so it becomes a word, and
// the time is re-said in the same clock the rest of the line uses.

import { clock } from "./when";

const DAYS: [RegExp, string][] = [
  [/\bweekdays?\b/i, "Mon–Fri"],
  [/\bweekends?\b/i, "Sat–Sun"],
  [/\bmondays?\b/i, "Mon"],
  [/\btuesdays?\b/i, "Tue"],
  [/\bwednesdays?\b/i, "Wed"],
  [/\bthursdays?\b/i, "Thu"],
  [/\bfridays?\b/i, "Fri"],
  [/\bsaturdays?\b/i, "Sat"],
  [/\bsundays?\b/i, "Sun"],
];

/** The cadence in the few characters a sidebar row has beside a name: "Every day at ~7:00 AM"
 * → "7:00 am", "Every Monday at ~9:30 AM" → "Mon 9:30 am", "Weekdays at 8:00 AM" →
 * "Mon–Fri 8:00 am". A schedule with no clock time ("Every hour") is already short and comes
 * back as scheduleLabel says it. The full sentence belongs in the row's tooltip. */
export function scheduleShort(raw: string | null | undefined): string {
  if (!raw) return "";
  const t = raw.match(/\b(\d{1,2}):(\d{2})\s*(AM|PM)\b/i);
  if (!t) return scheduleLabel(raw);
  let hour = Number(t[1]) % 12;
  if (t[3].toUpperCase() === "PM") hour += 12;
  const d = new Date();
  d.setHours(hour, Number(t[2]), 0, 0);
  const day = DAYS.find(([re]) => re.test(raw))?.[1];
  return [day, clock(d)].filter(Boolean).join(" ");
}

/** "Every day at ~7:00 AM" → "Every day at about 7:00 am" (in the system's own clock). */
export function scheduleLabel(raw: string | null | undefined): string {
  if (!raw) return "";
  return raw.replace(/~?\b(\d{1,2}):(\d{2})\s*(AM|PM)\b/gi, (match, h: string, m: string, ap: string) => {
    let hour = Number(h) % 12;
    if (ap.toUpperCase() === "PM") hour += 12;
    const d = new Date();
    d.setHours(hour, Number(m), 0, 0);
    return `${match.startsWith("~") ? "about " : ""}${clock(d)}`;
  });
}

/** How long a run may say "running" before the app stops believing it. A run is a single
 *  turn with an iteration cap; one still "running" after this long was cut off — the app
 *  closed, the machine slept — and will never report. */
export const RUN_STALE_AFTER_S = 2 * 60 * 60;

/** A run's status as it should be read now. */
export function runStatus(status: string, startedAt: number, nowMs = Date.now()): { label: string; tone: "ok" | "error" | "running" | "stale" } {
  if (status === "running") {
    return nowMs / 1000 - startedAt > RUN_STALE_AFTER_S
      ? { label: "didn't finish", tone: "stale" }
      : { label: "running", tone: "running" };
  }
  if (status === "ok") return { label: "ok", tone: "ok" };
  return { label: status, tone: "error" };
}

/** The last run, as the end of an automation's summary line. It read "last run running" and
 *  "last run ok": the machine's status words dropped after a label. */
export function lastRunText(status: string): string {
  if (status === "running") return "running now";
  if (status === "ok") return "last run succeeded";
  if (status === "error" || status === "failed") return "last run failed";
  return `last run ${status}`;
}

/** "1 file" / "3 files". */
export const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
