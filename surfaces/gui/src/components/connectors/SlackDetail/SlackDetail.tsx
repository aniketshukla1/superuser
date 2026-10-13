import { AVATAR } from "../../Icon";
import { useEffect, useState } from "react";
import { disallowUser, getSubscriptions, type SlackStatus, type Subscription } from "../../../api";
import { ConnectorBadge } from "../../../connectors/ConnectorIcon";
import { AddConnectionModal } from "../AddConnectionModal";
import type { DetailProps } from "../ConnectorsSection";
import { SlackHowItWorks } from "../SlackHowItWorks";
import { ToolsDisclosure } from "../ToolsDisclosure";
import { FOOT, GRP, GRP_H, PILL_ACCENT, ROW } from "../ui";
import { PeopleRow, WorkspaceGroup } from "./WorkspaceGroup";
import { ApprovalOwnersRow, ListeningRows, WaitingRow } from "./ApprovalOwnersRow";

// The Slack detail page (UX-DECISIONS §21): one group per connected workspace —
// People (allow-list) · Waiting (parked senders) · Listening (session ↔ channel) ·
// Disconnect — because Slack ids are workspace-scoped, everything is filed under
// the workspace it belongs to. Adding a workspace goes through the ONE entry
// point: the header button → AddConnectionModal (One click | Manual).

/** Two-letter initials for a person chip. */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

export const LABEL = "text-sm text-muted w-24 shrink-0";

/** The relay status line, one honest layer at a time: sign-in → socket → live.
 * Dot color + text; never a synthetic "Slack is down" claim. */
export function relayHealth(slack: SlackStatus | null): { dot: string; text: string } {
  if (!slack) return { dot: "bg-ok", text: "Live · managed relay" };
  if (!slack.signed_in)
    return { dot: "bg-warnInk", text: "Sign-in needed — relaying is paused" };
  if (slack.relay.state === "offline")
    return { dot: "bg-faint/60", text: "Offline — can't reach the relay" };
  if (slack.relay.state === "reconnecting")
    return { dot: "bg-warnInk", text: "Reconnecting to the relay…" };
  return { dot: "bg-ok", text: "Live · managed relay" };
}

export function SlackDetail({ c, cloud, slack, onChanged }: DetailProps) {
  const [adding, setAdding] = useState(false);
  const [subs, setSubs] = useState<Subscription[]>([]);
  const loadSubs = () => getSubscriptions().then(setSubs).catch(() => setSubs([]));
  useEffect(() => {
    loadSubs();
  }, [c.name]);

  const relay = c.mode === "relay";
  const workspaces = c.workspaces ?? [];
  const changed = () => {
    onChanged();
    loadSubs();
  };

  return (
    <div data-testid="slack-workspaces">
      <div className="flex items-center gap-3.5 mb-5">
        <ConnectorBadge connector={c} size={AVATAR.xl} title="Slack" />
        <div className="min-w-0 flex-1">
          <h2 className="text-xl font-semibold tracking-tight leading-tight">Slack</h2>
          <div className="text-sm text-muted flex items-center gap-1.5">
            {c.connected ? (
              <>
                <span
                  className={
                    "w-2 h-2 rounded-full " + (relay ? relayHealth(slack).dot : "bg-ok")
                  }
                />
                <span data-testid="slack-mode-badge">
                  {relay
                    ? relayHealth(slack).text
                    : "Connected · Socket Mode (manual tokens)"}
                </span>
              </>
            ) : (
              <span>Not connected</span>
            )}
          </div>
        </div>
        {relay || !c.connected ? (
          <button className={PILL_ACCENT} data-testid="add-workspace-btn" onClick={() => setAdding(true)}>
            ＋ Add workspace
          </button>
        ) : null}
      </div>

      {!c.connected && (
        <div className={GRP}>
          <div className={ROW + " text-sm text-muted"}>
            One @ocw app, installed per workspace — each keeps its own allow-list.
            {cloud?.signed_in ? "" : " One-click needs cloud sign-in; Manual works without it."}
          </div>
        </div>
      )}

      {/* UX-027: post-connect orientation — status line + animated how-it-works
          carousel (collapsible; collapsed state is the local "seen" flag). */}
      {relay && workspaces.length > 0 && <SlackHowItWorks workspaces={workspaces} />}

      {relay &&
        workspaces.map((w) => (
          <WorkspaceGroup
            key={w.team_id}
            c={c}
            w={w}
            subs={subs}
            tokenOk={slack?.teams?.[w.team_id]?.token_ok !== false}
            onChanged={changed}
          />
        ))}

      {/* Manual Socket Mode: one workspace, the flat allow-list (unchanged semantics). */}
      {c.connected && !relay && (
        <div data-testid="slack-manual-card">
          <div className={GRP_H}>{c.account || "workspace"} <span className="font-normal text-faint">· manual tokens</span></div>
          <div className={GRP}>
            <PeopleRow
              allowed={c.allowed_users}
              names={c.allowed_user_names}
              protectedIds={c.approval_owner_ids}
              teamId={null}
              onRemove={(u) => disallowUser("slack", u).then(changed)}
              onChanged={changed}
            />
            <ApprovalOwnersRow
              owners={c.approval_owner_ids ?? []}
              names={c.approval_owner_names}
              editable
              onChanged={changed}
            />
            {(c.unauthorized ?? [])
              .filter((m) => !m.team_id)
              .map((m) => (
                <WaitingRow key={m.id} m={m} onChanged={changed} />
              ))}
            <ListeningRows
              subs={subs.filter((s) => s.channel.startsWith("slack:") && !s.channel.includes("/"))}
              onChanged={changed}
            />
          </div>
        </div>
      )}

      <ToolsDisclosure c={c} />
      {c.connected && (
        <div className={FOOT + " mt-2"}>Names come from Slack automatically. IDs show on hover.</div>
      )}

      {adding && (
        <AddConnectionModal
          c={c}
          cloud={cloud}
          title="Add a workspace"
          onClose={() => setAdding(false)}
          onChanged={changed}
        />
      )}
    </div>
  );
}
