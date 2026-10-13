import { useEffect, useState, type ReactNode } from "react";
import type { WatchActivity } from "../../api";
import { PageTabs, type PageTab } from "../PageTabs";
import { ScheduledView } from "../ScheduledView";
import { timeAgo } from "../../when";
import { WatchesView } from "./WatchesView";

// Watches — the half of "work that happens without you" that isn't on a clock.
//
// An automation fires at 9am whether or not there's anything to do. A watch is a
// standing question that only costs anything when something actually changed, and
// it answers with a decision rather than a run: skip, answer it here, or ask you.
//
// Both live under Automations because from where the user sits they're the same
// category — things the assistant does while they're not looking. The difference
// is the trigger, and that's what the two tabs say.
//
// The activity log is the point of the page, not a footnote. A background loop
// nobody can inspect is one nobody should trust, and "I looked and there was
// nothing" is most of what it does.

export const INPUT = "flex-1 min-w-0 px-3 py-2 rounded-lg border bg-paper text-md text-ink outline-none";

export type Tab = "scheduled" | "watches";

export const AUTOMATION_TABS: PageTab<Tab>[] = [
  { key: "scheduled", label: "Scheduled", hint: "Work that runs at a set time", testId: "automations-tab-scheduled" },
  { key: "watches", label: "Watches", hint: "Work that runs when something changes", testId: "automations-tab-watches" },
];

/** Puts a view's content under the surface's title and tabs. Both views take it, so the two
 *  can't drift apart; the task detail page doesn't, because inside one automation there is
 *  nothing to choose between. */
export type Frame = (content: ReactNode) => ReactNode;

/**
 * The Automations surface: two triggers, one place. Owns the tab so switching
 * doesn't reset either view, and hides the tabs on a task's detail page — you're
 * inside one automation there, not choosing between kinds.
 */
export function AutomationsSurface(props: {
  onOpenRun: (sessionId: string, workspace: string, agent: string, task?: { id: string; title: string }) => void;
  onRunNow: (taskId: string, title?: string) => void;
  initialOpenId?: string | null;
  /** Bumped by the sidebar's Scheduled "+" — see ScheduledView. */
  newRequest?: number;
  /** Driven from the URL when the app supplies it, so `#/automations?tab=watches`
   *  is an address and Back moves between the two. */
  tab?: string;
  onTabChange?: (t: Tab) => void;
}) {
  const [internal, setInternal] = useState<Tab>("scheduled");
  const tab: Tab = props.tab === "watches" || props.tab === "scheduled" ? props.tab : internal;
  const setTab = (t: Tab) => {
    setInternal(t);
    props.onTabChange?.(t);
  };
  // The sidebar's Scheduled band can deep-link to one automation while Watches is
  // the open tab; that's a request for Scheduled, not a silent no-op.
  useEffect(() => {
    if (props.initialOpenId) setTab("scheduled");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.initialOpenId]);
  // "New automation" is a request for Scheduled too, even if Watches is open.
  useEffect(() => {
    if (props.newRequest) setTab("scheduled");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.newRequest]);
  // The page is "Automations"; the tab says which kind. It used to open on a pill row above a
  // page titled "Scheduled", with no name for the surface itself anywhere on it.
  const frame: Frame = (content) => (
    <PageTabs title="Automations" tabs={AUTOMATION_TABS} current={tab} onChange={setTab}>
      {content}
    </PageTabs>
  );
  return tab === "watches" ? <WatchesView frame={frame} /> : <ScheduledView {...props} frame={frame} />;
}

export const relative = (t: number) => timeAgo(t);

export const DECISION: Record<WatchActivity["decision"], { label: string; cls: string }> = {
  skip: { label: "Nothing to do", cls: "text-faint" },
  act: { label: "Answered", cls: "text-accent" },
  escalate: { label: "Asked you", cls: "text-ink" },
};
