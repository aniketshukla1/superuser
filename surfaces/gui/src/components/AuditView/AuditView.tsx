import { useState } from "react";
import { PanelHead } from "../IntegrationsView";
import { PageTabs, type PageTab } from "../PageTabs";
import { NetworkSection } from "../NetworkSection";
import { ActionsSection } from "./ActionsSection";

// Privacy — the two halves of "what has this thing been doing on my behalf".
//
//   Network — what left this machine, from the one chokepoint every request goes
//             through (`core/egress.rs`). Hosts, not sessions.
//   Actions — what it did: connector syncs, tool calls, approvals.
//
// One page with two tabs rather than two surfaces, because from where the user
// sits it is one question; the tabs name the only real difference, which is
// whether the thing crossed the network boundary.
//
// Network is the default tab. The page is reached by someone checking on the
// product's central claim, and "who has heard from you" is that claim's answer.
export const INPUT = "px-3 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0";
export const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export type Tab = "network" | "actions";

export const PRIVACY_TABS: PageTab<Tab>[] = [
  { key: "network", label: "Network", hint: "What left this machine", testId: "privacy-tab-network" },
  { key: "actions", label: "Actions", hint: "What your assistants did", testId: "privacy-tab-actions" },
];

export function AuditView() {
  const [tab, setTab] = useState<Tab>("network");

  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">
          <PageTabs title="Privacy" tabs={PRIVACY_TABS} current={tab} onChange={setTab}>
            <PanelHead
              title={tab === "network" ? "Network" : "Actions"}
              sub={
                tab === "network"
                  ? "Everything that left this machine, and who received it. Recorded at the one point outbound requests pass through — query strings are never stored."
                  : "What your assistants did, what your boundaries refused, and what started each run. Written before each action runs; what a message said or a file held is never stored."
              }
            />
            {tab === "network" ? <NetworkSection /> : <ActionsSection />}
          </PageTabs>
        </div>
      </div>
    </main>
  );
}

// What to show, in the words a person would use for it. Each maps to the query the
// route understands, so the menu cannot offer a view the server can't answer.
export type Show = "all" | "actions" | "refused" | "declined" | "stalled" | "boundaries";
export const SHOW: { key: Show; label: string; query: { event?: string; refused?: boolean } }[] = [
  { key: "all", label: "Everything", query: {} },
  { key: "actions", label: "Actions", query: { event: "action" } },
  { key: "refused", label: "Refused by a boundary", query: { refused: true } },
  { key: "declined", label: "Declined by an assistant", query: { event: "assistant.declined" } },
  { key: "stalled", label: "A model went quiet", query: { event: "turn.stalled" } },
  { key: "boundaries", label: "Boundary changes", query: { event: "policy" } },
];

export const RETENTION: { days: number; label: string }[] = [
  { days: 0, label: "Keep everything" },
  { days: 365, label: "Keep a year" },
  { days: 90, label: "Keep 90 days" },
  { days: 30, label: "Keep 30 days" },
];
