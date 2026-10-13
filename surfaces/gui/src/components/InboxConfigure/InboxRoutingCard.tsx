import { useEffect, useState } from "react";
import { getConnectors, getInboxRouting, getRecentChannels, setInboxBinding, type RecentChannel, type Connector } from "../../api";
import { ChannelPicker } from "../SubscriptionsChip";
import { Icon, ICON } from "../Icon";
import { CARD } from "../../ui";
import { DmRouteCard, SubscriptionsCard } from "./SubscriptionsCard";
import { UnroutedTable } from "./UnroutedTable";

// Inbox ▸ Configure (UX-DECISIONS §28): the former Connectors ▸ "Messaging routing" page,
// relocated whole — where inbox items go out (mirror channel), how inbound messages reach
// sessions (DM route, channel subscriptions), and the Unrouted dead-letter. Moving it here
// also deleted a duplication: the mirror channel used to be editable BOTH on this page and
// via an inline configurator on the Inbox list.
export const SELECT = "px-2.5 py-1.5 rounded-lg border border-line bg-paper text-md text-ink";

export function InboxConfigure() {
  return (
    <div data-testid="inbox-configure">
      {/* Side by side while each card has ~280px; stacked below that. A fixed two-column grid
          gave each card 284px at the 880px window and wrapped the channel field under its icon. */}
      <div className="inbox-config-grid">
        <InboxRoutingCard />
        <DmRouteCard />
      </div>
      <SubscriptionsCard />
      {/* Unrouted = delivery FAILURES ("messages that never reached you"), so it lives with
          the Inbox now (§28; previously with routing under Connectors, §26). */}
      <div className="mt-6" data-testid="unrouted-section">
        <h3 className="text-lg font-semibold mb-1">Unrouted</h3>
        <p className="text-sm text-muted mb-3">
          Messages nobody was set up to receive, and background runs that failed. They're kept
          here so nothing disappears without a trace.
        </p>
        <UnroutedTable />
      </div>
    </div>
  );
}

// Where an Unattended session's approvals/questions get mirrored as interactive buttons. Targets
// the "default" route (sessions fall back to it); pick a channel separate from any you subscribe to.
export function InboxRoutingCard() {
  const [recent, setRecent] = useState<RecentChannel[]>([]);
  const [connectors, setConnectors] = useState<Connector[]>([]);
  const [target, setTarget] = useState(""); // current default-binding address, e.g. "slack:C0123"
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = () => {
    getRecentChannels().then(setRecent).catch(() => setRecent([]));
    getConnectors().then(setConnectors).catch(() => setConnectors([]));
    getInboxRouting()
      .then((bs) => {
        const def = bs.find((b) => b.name === "default");
        setTarget(def?.channel ? `${def.channel}:${def.target}` : "");
      })
      .catch(() => setTarget(""));
  };
  useEffect(() => {
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, []);

  const save = async () => {
    const addr = draft.trim();
    if (!addr) return;
    // "slack:C0123" → channel="slack", target="C0123"; a bare id assumes slack.
    const [platform, id] = addr.includes(":") ? addr.split(":", 2) : ["slack", addr];
    const result = await setInboxBinding("default", platform, id);
    if (!result.ok) {
      setError(result.error || "Could not update Inbox routing.");
      return;
    }
    setError(null);
    setDraft("");
    load();
  };
  const clear = async () => {
    const result = await setInboxBinding("default", null, "");
    if (!result.ok) {
      setError(result.error || "Could not clear Inbox routing.");
      return;
    }
    setError(null);
    load();
  };

  const draftAddr = draft.trim();
  const [draftPlatform, draftTarget] = draftAddr.includes(":")
    ? draftAddr.split(":", 2)
    : ["slack", draftAddr];
  const slack = connectors.find((c) => c.name === "slack");
  const teamId =
    draftPlatform === "slack" && draftTarget.includes("/")
      ? draftTarget.split("/", 1)[0]
      : null;
  const owners =
    draftPlatform !== "slack"
      ? []
      : teamId
        ? slack?.workspaces?.find((w) => w.team_id === teamId)?.approval_owner_ids ?? []
        : slack?.approval_owner_ids ?? [];
  const missingSlackOwner =
    draftPlatform === "slack" && draftTarget.length > 0 && owners.length === 0;

  // Show the channel's NAME when the recent list knows it (raw address as the fallback/tooltip).
  const known = recent.find((c) => c.channel === target)?.name;

  return (
    <div className={CARD + " p-4"} data-testid="inbox-mirror-card">
      <div className="font-semibold text-md mb-1">Unattended approvals</div>
      <p className="text-sm text-muted mb-3">
        When a conversation runs unattended, its approvals are also posted to a channel, with
        Approve and Deny buttons. Now going to{" "}
        <strong className="text-ink font-medium" title={target || undefined}>
          {known ? `#${known}` : target || "in-app Inbox only"}
        </strong>
        .
      </p>
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-muted shrink-0">
          <Icon name="plug" size={ICON.md} />
        </span>
        <ChannelPicker value={draft} onChange={setDraft} recent={recent} onSubmit={save} />
        <button
          className="btn-primary sm"
          disabled={!draft.trim() || missingSlackOwner}
          onClick={save}
        >
          Set
        </button>
        {target && (
          <button className="btn sm ghost" onClick={clear} title="Stop posting approvals to this channel">
            Clear
          </button>
        )}
      </div>
      {missingSlackOwner && (
        <p className="text-xs text-warnInk mt-2">
          Choose an approval owner under Connectors › Slack before routing approvals here.
        </p>
      )}
      {error && <p className="text-xs text-warnInk mt-2">{error}</p>}
    </div>
  );
}
