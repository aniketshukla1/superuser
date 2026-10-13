import { useEffect, useState } from "react";
import { getConnectors, getMcpServers } from "../api";
import { McpTab } from "./ManageTabs";
import { ConnectorsSection } from "./connectors/ConnectorsSection";
import { PageTabs, useInTabbedPage } from "./PageTabs";

// The Connectors surface (renamed from "Integrations", §26) keeps the left sub-nav, now just
// Connectors · MCP. The old "Messaging routing" tab (and its ⚠ unrouted badge) moved whole to
// Inbox ▸ Configure (§28): inbox-delivery config belongs with the Inbox, and Unrouted is
// "messages that never reached you". The one remaining Activity is the audit log, reached from
// the account menu.
type IntTab = "connectors" | "mcp";

// Two sections (UX-DECISIONS §21): connector detail lives as a SUBPAGE under Apps, never as a
// tab — the row must not grow per connector. They were a 208px column holding two entries,
// with "Connectors" written three times: the column's heading, its first entry, and the page.
export function IntegrationsView() {
  const [tab, setTab] = useState<IntTab>("connectors");
  // Counted from the two lists this page opens, so the tabs cannot disagree with them.
  // They used to be chips reading "Connected 0" and "MCP 0" above a page listing four
  // connected servers.
  const [counts, setCounts] = useState<{ apps: number | null; mcp: number | null }>({
    apps: null,
    mcp: null,
  });
  useEffect(() => {
    const load = () => {
      getConnectors()
        .then((cs) => setCounts((c) => ({ ...c, apps: cs.filter((x) => x.connected).length })))
        .catch(() => {});
      getMcpServers()
        .then((ss) => setCounts((c) => ({ ...c, mcp: ss.length })))
        .catch(() => {});
    };
    load();
    const t = setInterval(load, 10_000);
    return () => clearInterval(t);
  }, []);
  const tabs = [
    { key: "connectors" as const, label: "Apps", count: counts.apps || null, hint: "Apps your assistants can use" },
    { key: "mcp" as const, label: "MCP servers", count: counts.mcp || null, hint: "Tool servers, shared by every assistant" },
  ];

  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">
          <PageTabs title="Connectors" tabs={tabs} current={tab} onChange={setTab}>
            {tab === "connectors" ? (
              <section>
                <PanelHead
                  title="Apps"
                  sub="Apps and tools your assistants can use. Connected ones come first."
                />
                <ConnectorsSection />
              </section>
            ) : (
              <section>
                <PanelHead
                  title="MCP servers"
                  sub="External tool servers (stdio or HTTP), shared across all agents."
                />
                <McpTab />
              </section>
            )}
          </PageTabs>
        </div>
      </div>
    </main>
  );
}

// Every surface's page title goes through here, so this is where the outline is set.
// It used to emit an <h2> with no <h1> above it anywhere on the page, which left six
// of the seven surfaces headed by a level-2 heading and no level 1 at all — a screen
// reader's outline started mid-document (UX-REPORT §8).
//
// The title is the page heading step (--t-xl, 19px). It was --t-lg — the same 15px/600 as the
// title of every card on the page, so "Inbox" and "Run codex on codex" were typographically the
// same thing. Inside a tabbed page (PageTabs) the section's name is already on screen as the
// selected tab, so the heading is kept for the outline and hidden from view.
export function PanelHead({ title, sub }: { title: string; sub: string }) {
  const tabbed = useInTabbedPage();
  if (tabbed) {
    return (
      <div className="mb-4">
        <h2 className="sr-only">{title}</h2>
        <p className="page-lead">{sub}</p>
      </div>
    );
  }
  return (
    <div className="mb-4">
      <h1 className="page-title">{title}</h1>
      <p className="text-sm text-muted mt-1">{sub}</p>
    </div>
  );
}
