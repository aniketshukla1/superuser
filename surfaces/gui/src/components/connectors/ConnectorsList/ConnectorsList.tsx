import { AVATAR, Icon, ICON } from "../../Icon";
import { useState } from "react";
import type { CloudStatus, Connector, SlackStatus } from "../../../api";
import { ConnectorBadge } from "../../../connectors/ConnectorIcon";
import { AddConnectionModal } from "../AddConnectionModal";
import { GRP, GRP_H, FOOT, PILL_QUIET, ROW } from "../ui";
import { healthChip } from "./healthChip";

export type Tab = "featured" | "all" | "connected";

// No "MCP" filter: it sat one row under the page's "MCP servers" tab and meant something
// else (apps that happen to connect over MCP), and how an app connects is not a reason
// anyone looks for it.
export const TABS: { id: Tab; label: string }[] = [
  { id: "featured", label: "Featured" },
  { id: "all", label: "All" },
  { id: "connected", label: "Connected" },
];

// "Featured" is the everyday set most people actually connect first — mail,
// calendar, chat, notes, code, payments. Everything else is one tab away under
// All; this is an ordering opinion, not a capability gate.
export const FEATURED = new Set([
  "gmail", "google", "googlecalendar", "calendar", "outlook", "slack", "discord",
  "notion", "github", "linear", "jira", "stripe", "hubspot", "asana", "email",
  "drive", "dropbox", "figma",
]);
export const isFeatured = (c: { name: string; connected?: boolean }) => FEATURED.has(c.name) || !!c.connected;

// The Connectors LIST (UX-DECISIONS §21): connected first in their own inset group —
// rows navigate to the connector's detail subpage; problems surface as a chip in the
// list, never one click deep. Available connectors below with a Connect pill.

// Rows shown before "Show more", on All only: Featured is already the short list, and folding
// it hid part of the set it exists to show.
export const AVAILABLE_FOLD = 8;

export function ConnectorsList({
  connectors,
  cloud,
  slack,
  onOpen,
  onChanged,
}: {
  connectors: Connector[];
  cloud: CloudStatus | null;
  slack: SlackStatus | null;
  onOpen: (name: string) => void;
  onChanged: () => void;
}) {
  const [filter, setFilter] = useState("");
  const [showAll, setShowAll] = useState(false);
  const [connecting, setConnecting] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("featured");

  const q = filter.trim().toLowerCase();
  const match = (c: Connector) => !q || c.title.toLowerCase().includes(q) || c.name.includes(q);
  // Tab counts are computed over everything, not the filtered view — a count that
  // changed as you typed would be useless as a sense of what's there.
  const counts = {
    featured: connectors.filter(isFeatured).length,
    all: connectors.length,
    connected: connectors.filter((c) => c.connected).length,
  };
  const inTab = (c: Connector) => (tab === "all" ? true : tab === "featured" ? isFeatured(c) : !!c.connected);

  const connected = connectors.filter((c) => c.connected && match(c) && inTab(c));
  const available = connectors.filter((c) => !c.connected && c.available && match(c) && inTab(c));
  const folds = tab === "all" && !showAll && !q && available.length > AVAILABLE_FOLD;
  const shown = folds ? available.slice(0, AVAILABLE_FOLD) : available;
  const connectingC = connecting ? connectors.find((c) => c.name === connecting) : null;

  return (
    <div>
      {/* Wraps rather than overflows: with a third fixed nav column on this surface, an
          880px window pushed the search field 59px past the window edge (UX-REPORT §4). */}
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
        <div className="flex flex-wrap items-center gap-1 min-w-0" role="tablist">
          {TABS.map((t) => (
            <button
              key={t.id}
              role="tab"
              aria-selected={tab === t.id}
              data-testid={`connectors-tab-${t.id}`}
              onClick={() => {
                setTab(t.id);
                setShowAll(false);
              }}
              className={
                "px-2.5 py-1 rounded-lg text-sm " +
                (tab === t.id ? "bg-accentSoft text-accent" : "text-muted hover:text-ink")
              }
            >
              {t.label} <span className="text-faint">{counts[t.id]}</span>
            </button>
          ))}
        </div>
        <input
          placeholder="Search"
          aria-label="Search connectors"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="w-44 max-w-full min-w-0 px-3.5 py-1.5 rounded-full border border-line bg-panel text-md outline-none focus:border-accent"
        />
      </div>

      {/* No cloud strip here anymore (§26): the sidebar's account row is the permanent
          sign-in home, and the connect modals keep their inline sign-in panes. */}
      {connected.length > 0 && (
        <>
          <div className={GRP_H + " !mt-0"}>Connected · {connected.length}</div>
          <div className={GRP}>
            {connected.map((c) => (
              <button
                key={c.name}
                data-testid={`connector-${c.name}`}
                className={ROW + " w-full text-left hover:bg-paper/60"}
                onClick={() => onOpen(c.name)}
              >
                <ConnectorBadge connector={c} size={AVATAR.lg} title={c.title} />
                <span className="min-w-0 flex-1">
                  <span className="font-medium text-md">{c.title}</span>
                  <span className="block text-sm text-muted">{statusLine(c)}</span>
                </span>
                {healthChip(c, slack)}
                <Icon name="chevronRight" size={ICON.md} className="text-faint shrink-0" />
              </button>
            ))}
          </div>
        </>
      )}

      <div className={GRP_H}>Available</div>
      <div className={GRP}>
        {shown.map((c) => (
          /* The row navigates to the pre-connect detail page (§38); the pill
             stays the fast path straight into the modal. Two buttons side by side: the
             pill was a span with role=button inside the row's button, which a keyboard
             could never reach and a screen reader read as one control. */
          <div key={c.name} data-testid={`connector-${c.name}`} className={ROW + " hover:bg-paper/60"}>
            <button
              className="flex items-center gap-3 min-w-0 flex-1 self-stretch text-left"
              onClick={() => onOpen(c.name)}
            >
              <ConnectorBadge connector={c} size={AVATAR.lg} title={c.title} />
              <span className="min-w-0 flex-1">
                <span className="font-medium text-md">{c.title}</span>
                <span className="block text-sm text-muted truncate">{c.blurb}</span>
              </span>
            </button>
            <button
              className={PILL_QUIET}
              aria-label={`Connect ${c.title}`}
              onClick={() => setConnecting(c.name)}
            >
              Connect
            </button>
          </div>
        ))}
        {shown.length === 0 && (
          <div className={ROW + " text-sm text-muted"}>Nothing matches.</div>
        )}
      </div>
      {folds && (
        <div className={FOOT}>
          <button className="text-muted hover:text-ink" onClick={() => setShowAll(true)}>
            Show {available.length - AVAILABLE_FOLD} more
          </button>
        </div>
      )}

      {connectingC && (
        <AddConnectionModal
          c={connectingC}
          cloud={cloud}
          onClose={() => setConnecting(null)}
          onChanged={onChanged}
        />
      )}
    </div>
  );
}

export function statusLine(c: Connector): string {
  if (c.name === "slack" && c.mode === "relay") {
    const n = c.workspaces?.length ?? 0;
    // "relay" was the transport's name. The connect modal calls this mode "One click".
    return `${n} workspace${n === 1 ? "" : "s"} · one-click connection`;
  }
  if ((c.accounts?.length ?? 0) > 1) return `${c.accounts!.length} accounts`;
  if ((c.portals?.length ?? 0) > 1) return `${c.portals!.length} portals`;
  if (c.auth === "none") return "Built in";
  return c.account || "Connected";
}
