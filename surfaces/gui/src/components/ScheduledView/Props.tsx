import { dayAndTime } from "../../when";
import type { Frame } from "../WatchesView";

/** What started a run, in words. The stored values are machine keys. */
export const TRIGGER: Record<string, string> = {
  schedule: "on schedule",
  manual: "run by hand",
  now: "run by hand",
  catchup: "caught up after a missed time",
  catch_up: "caught up after a missed time",
};

// Shared utility strings (the §28 page shell — mirrors IntegrationsView's constants).

// Parse a simple "min hour * * dow" cron back into the time + frequency the editor uses.
// Falls back to 09:00 / daily for anything it doesn't recognize (e.g. agent-written crons).
export function fromCron(cron?: string | null): { time: string; freq: string } {
  const parts = (cron || "").trim().split(/\s+/);
  if (parts.length !== 5) return { time: "09:00", freq: "daily" };
  const [m, h, , , dow] = parts;
  const hh = String(Math.min(23, Math.max(0, parseInt(h, 10) || 9))).padStart(2, "0");
  const mm = String(Math.min(59, Math.max(0, parseInt(m, 10) || 0))).padStart(2, "0");
  const freq = dow === "1-5" ? "weekdays" : dow === "0,6" || dow === "6,0" ? "weekends" : "daily";
  return { time: `${hh}:${mm}`, freq };
}

// "tomorrow at 7:00 am" rather than "15 Sept 2026, 07:00": the day relative to today is what
// a schedule line is read for, and the clock matches the schedule label beside it.
export const fmt = (t: number | null) => (t ? dayAndTime(t) : "—");

// Map a simple time-of-day + frequency selection to a 5-field cron string.
export function toCron(time: string, freq: string): string {
  const [h, m] = (time || "09:00").split(":").map((x) => parseInt(x, 10) || 0);
  const dow = freq === "weekdays" ? "1-5" : freq === "weekends" ? "0,6" : "*";
  return `${m} ${h} * * ${dow}`;
}

// The §28 page shell: full-bleed main, centered ≤4xl column — same as Connectors/Activity/Inbox.
// `frame` puts the content under the surface's title and tabs (see AutomationsSurface).
export function Shell({ children, frame = (c) => c }: { children: React.ReactNode; frame?: Frame }) {
  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">{frame(children)}</div>
      </div>
    </main>
  );
}

export interface Props {
  // `task` gives the opened run session its context (banner + "Back to runs"; owner ask 2026-07-04).
  onOpenRun: (
    sessionId: string,
    workspace: string,
    agent: string,
    task?: { id: string; title: string },
  ) => void;
  onRunNow: (taskId: string, title?: string) => void;
  // Open directly on a task's detail (set by the run banner's "Back to runs").
  initialOpenId?: string | null;
  /** Bumped by the sidebar's Scheduled "+" to open the new-automation form.
   *  A counter, not a flag, for the same reason `initialOpenId` needs its own
   *  effect: pressing + again after closing the form has to reopen it, and a
   *  boolean that is already `true` says nothing has changed. */
  newRequest?: number;
  // The Automations title and Scheduled/Watches tabs, owned by the surface above so the
  // choice survives navigating between the two. Not applied to the task detail page, where
  // you're inside one automation rather than choosing between kinds.
  frame?: Frame;
}
