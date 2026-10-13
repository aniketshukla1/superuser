import { useEffect, useMemo, useState } from "react";
import { getConnectors, getInbox, getInboxRouting, getPersonas, getRecentChannels, getUnrouted, resolveInboxItem, type InboxItem, type Persona, type RecentChannel } from "../../api";
import { InboxItemCard } from "../InboxItemCard";
import { InboxConfigure } from "../InboxConfigure";
import { PanelHead } from "../IntegrationsView";
import { PageTabs, type PageTab } from "../PageTabs";
import { shortPersonaName } from "../../personaScope";
import { isStaleApproval } from "../../inboxText";
import { type InboxTab } from "./ICON_FOR";
import { InboxFilters } from "./InboxFilters";
import { SessionChip } from "./SessionChip";
import { StaleApprovals } from "./StaleApprovals";

// The Inbox: pending approvals / questions / notifications from across sessions, including
// unattended ones. Resolving here releases any agent suspended on the item. Each item links back
// to its originating session so you can see the context before answering. Items whose session
// was deleted are closed server-side (an orphaned prompt can never be answered), so everything
// listed here is actionable. Filters: by kind and by persona (owner ask, 2026-07-03).
// Two page tabs (§28): Pending (the queue) and Configure (the former Connectors ▸ Messaging
// routing page — mirror channel, DM route, subscriptions, Unrouted). Pending's routing status
// is read-only and links to Configure; the old inline editor was the mirror setting's SECOND
// editor and is gone.
export function InboxView({
  onOpenSession,
}: {
  onOpenSession: (sessionId: string, workspace: string, agent: string) => void;
}) {
  const [tab, setTab] = useState<InboxTab>("pending");
  const [items, setItems] = useState<InboxItem[]>([]);
  const [personas, setPersonas] = useState<Persona[] | null>(null);
  const [routing, setRouting] = useState<string | null>(null); // e.g. "slack:C0123" or null
  const [slackConnected, setSlackConnected] = useState(false);
  const [recent, setRecent] = useState<RecentChannel[]>([]);
  const [unroutedCount, setUnroutedCount] = useState(0);
  const [kind, setKind] = useState<string>("all");
  const [personaFilter, setPersonaFilter] = useState<string>("all");

  const load = () => {
    getInbox(undefined, "pending").then(setItems).catch(() => {});
    getUnrouted().then((u) => setUnroutedCount(u.length)).catch(() => setUnroutedCount(0));
  };
  const loadRouting = () =>
    getInboxRouting()
      .then((bindings) => {
        const bound = bindings.find((b) => b.channel);
        setRouting(bound ? `${bound.channel}:${bound.target}` : null);
      })
      .catch(() => setRouting(null));
  useEffect(() => {
    load();
    loadRouting();
    getPersonas().then(setPersonas).catch(() => {});
    getConnectors()
      .then((cs) => setSlackConnected(!!cs.find((c) => c.name === "slack" && c.connected)))
      .catch(() => {});
    getRecentChannels().then(setRecent).catch(() => setRecent([]));
    const t = setInterval(() => {
      load();
      loadRouting(); // edits happen on the Configure tab; keep Pending's status line honest
    }, 4000);
    return () => clearInterval(t);
  }, []);

  const resolve = async (id: string, resolution: string) => {
    await resolveInboxItem(id, resolution);
    load();
  };

  // Approvals nobody answered for a day. The run that raised one has moved on or stopped, and
  // five of them stacked above today's questions is how a live request gets missed.
  const stale = items.filter((it) => isStaleApproval(it));

  // Personas that actually have pending items drive the filter chips (no empty chips).
  const personasWithItems = useMemo(() => {
    const ids = [...new Set(items.map((i) => i.session_agent).filter(Boolean))] as string[];
    return ids.map((id) => ({
      id,
      label: shortPersonaName(personas?.find((p) => p.id === id)?.name, id),
    }));
  }, [items, personas]);

  const visible = items.filter(
    (it) =>
      (kind === "all" || it.kind === kind) &&
      (personaFilter === "all" || it.session_agent === personaFilter),
  );


  const routingName = routing ? recent.find((c) => c.channel === routing)?.name : undefined;
  const routingLabel = routingName ? `#${routingName}` : routing;

  // A kind filter over one kind filters nothing; it appears once there are two to tell apart.
  const kindsPresent = new Set(items.map((it) => it.kind)).size;

  const tabs: PageTab<InboxTab>[] = [
    { key: "pending", label: "Pending", count: items.length || null, testId: "inbox-tab-pending" },
    {
      key: "configure",
      label: "Configure",
      count: unroutedCount || null,
      tone: "warn",
      hint: unroutedCount ? `${unroutedCount} undelivered — see Unrouted` : "Where inbox items are delivered",
      testId: "inbox-tab-configure",
    },
  ];
  const changeTab = (t: InboxTab) => {
    setTab(t);
    if (t === "pending") {
      // Configure-tab edits change the mirror target — re-read so the status line
      // is honest the moment the user lands back on Pending, not a poll later.
      loadRouting();
      load();
    }
  };

  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">
          <PageTabs title="Inbox" tabs={tabs} current={tab} onChange={changeTab}>
            {tab === "configure" ? (
              <>
                <PanelHead
                  title="Configure"
                  sub="Where inbox items go besides this app, which conversation answers direct messages, and anything that never reached one."
                />
                <InboxConfigure />
              </>
            ) : (
              <>
                <PanelHead
                  title="Pending"
                  sub="Approvals, questions, and notifications from your assistants — including sessions running unattended."
                />
                <div className="text-sm text-faint -mt-1 mb-4" data-testid="inbox-routing">
                  {routing ? (
                    <span>
                      Also delivered to{" "}
                      <span className="text-muted" title={routing}>
                        {routingLabel}
                      </span>{" "}
                      — replies there resolve items here.{" "}
                    </span>
                  ) : (
                    <span>Delivered here only. </span>
                  )}
                  {/* One link that says what it does. It was a parenthetical pointing at another page,
                      a roadmap promise, and a bare "Configure ›". */}
                  <button
                    className="text-accent hover:underline"
                    data-testid="inbox-route-configure"
                    onClick={() => setTab("configure")}
                  >
                    {routing ? "Change where they go ›" : slackConnected ? "Also send them to a Slack channel ›" : "Get these in Slack too ›"}
                  </button>
                </div>

                <StaleApprovals stale={stale} onDone={load} />

                <InboxFilters
                  kindsPresent={kindsPresent}
                  personasWithItems={personasWithItems}
                  kind={kind}
                  setKind={setKind}
                  personaFilter={personaFilter}
                  setPersonaFilter={setPersonaFilter}
                />

                {visible.length === 0 ? (
                  items.length === 0 ? (
                    <div className="manage-empty">
                      <div className="text-ink">Nothing needs you right now.</div>
                      <div className="mt-1">
                        When an assistant needs an approval or an answer to keep going, it waits here.
                      </div>
                    </div>
                  ) : (
                    <div className="manage-empty">Nothing pending for this filter.</div>
                  )
                ) : null}

                <div className="space-y-4">
                  {visible.map((it) => (
                    <InboxItemCard key={it.id} item={it} onResolve={resolve} chip={<SessionChip it={it} personas={personas} onOpenSession={onOpenSession} />} />
                  ))}
                </div>
              </>
            )}
          </PageTabs>
        </div>
      </div>
    </main>
  );
}
