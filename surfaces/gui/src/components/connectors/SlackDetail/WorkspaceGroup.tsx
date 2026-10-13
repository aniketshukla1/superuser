import { useState } from "react";
import { disallowUser, disconnectSlackWorkspace, type Connector, type SlackWorkspace, type Subscription } from "../../../api";
import { GRP, GRP_H, ROW, TAG_WARN, XBTN } from "../ui";
import { LABEL, initials } from "./SlackDetail";
import { PersonPicker } from "./PersonPicker";
import { ApprovalOwnersRow, ListeningRows, WaitingRow } from "./ApprovalOwnersRow";

export function WorkspaceGroup({
  c,
  w,
  subs,
  tokenOk,
  onChanged,
}: {
  c: Connector;
  w: SlackWorkspace;
  subs: Subscription[];
  tokenOk: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const parked = (c.unauthorized ?? []).filter((m) => m.team_id === w.team_id);
  const listening = subs.filter((s) => s.channel.startsWith(`slack:${w.team_id}/`));
  const empty = w.allowed_users.length === 0 && parked.length === 0 && listening.length === 0;

  const disconnect = async () => {
    setBusy(true);
    await disconnectSlackWorkspace(w.team_id);
    setBusy(false);
    onChanged();
  };

  return (
    <div data-testid={`slack-workspace-${w.team_id}`}>
      <div className={GRP_H + " flex items-center gap-2"}>
        <span>
          {/* Domain beats raw id as the differentiator (names can collide across
              workspaces; domains can't). The id stays reachable on hover. */}
          {w.account || w.team_id}{" "}
          <span className="font-normal text-faint" title={w.team_id}>
            · {w.domain || w.team_id}
          </span>
        </span>
        {!tokenOk && (
          <span className={TAG_WARN} data-testid={`token-warn-${w.team_id}`}>
            ⚠ Token revoked — reinstall
          </span>
        )}
      </div>
      <div className={GRP}>
        {empty ? (
          <>
            <div className={ROW}>
              <span className="min-w-0 flex-1 text-sm text-muted flex items-center gap-2 flex-wrap">
                <span>No one allowed yet — mentions of the bot show up here for your OK.</span>
                <PersonPicker teamId={w.team_id} allowed={[]} onChanged={onChanged} />
              </span>
              <DisconnectBtn teamId={w.team_id} busy={busy} onClick={disconnect} />
            </div>
            <ApprovalOwnersRow
              owners={w.approval_owner_ids ?? []}
              names={w.approval_owner_names}
              installerId={w.installer_user_id}
              installerName={w.installer_name}
              editable={false}
              onChanged={onChanged}
            />
          </>
        ) : (
          <>
            <PeopleRow
              allowed={w.allowed_users}
              names={w.allowed_user_names}
              protectedIds={w.approval_owner_ids}
              teamId={w.team_id}
              installerId={w.installer_user_id}
              installerName={w.installer_name}
              onRemove={(u) => disallowUser("slack", u, w.team_id).then(onChanged)}
              onChanged={onChanged}
            />
            <ApprovalOwnersRow
              owners={w.approval_owner_ids ?? []}
              names={w.approval_owner_names}
              installerId={w.installer_user_id}
              installerName={w.installer_name}
              editable={false}
              onChanged={onChanged}
            />
            {parked.map((m) => (
              <WaitingRow key={m.id} m={m} onChanged={onChanged} />
            ))}
            <ListeningRows subs={listening} onChanged={onChanged} />
            <div className={ROW}>
              <span className="flex-1" />
              <DisconnectBtn teamId={w.team_id} busy={busy} onClick={disconnect} />
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export function DisconnectBtn({ teamId, busy, onClick }: { teamId: string; busy: boolean; onClick: () => void }) {
  return (
    <button
      className="text-sm text-danger/80 hover:text-danger shrink-0"
      data-testid={`disconnect-workspace-${teamId}`}
      title="Stops relaying this workspace to this computer. The app stays installed in Slack."
      onClick={onClick}
      disabled={busy}
    >
      {busy ? "Disconnecting…" : "Disconnect workspace"}
    </button>
  );
}

export function PeopleRow({
  allowed,
  names,
  protectedIds,
  teamId,
  installerId,
  installerName,
  onRemove,
  onChanged,
}: {
  allowed: string[];
  names?: Record<string, string | null>;
  protectedIds?: string[];
  teamId: string | null; // null = manual flat list (directory queries as "default")
  installerId?: string; // authed_user — pre-added on managed connect (UX-027)
  installerName?: string;
  onRemove: (userId: string) => void;
  onChanged: () => void;
}) {
  // The installer's chip reads "you" — their name may still be unresolved (it's
  // fetched lazily for outbound attribution), so fall back to a literal "You".
  const label = (u: string) =>
    names?.[u] || (u === installerId ? installerName || "You" : u);
  return (
    <div className={ROW}>
      <span className={LABEL}>People</span>
      <span className="min-w-0 flex-1 flex flex-wrap items-center gap-1.5">
        {allowed.length === 0 && (
          <span className="text-sm text-faint">nobody yet — pick a name, or approve a waiting sender below</span>
        )}
        {allowed.map((u) => (
          <span
            key={u}
            className="inline-flex items-center gap-1.5 pl-1 pr-2 py-0.5 rounded-full bg-paper border border-line text-sm"
            title={`id ${u}`}
            data-testid={u === installerId ? "people-chip-you" : undefined}
          >
            <span className="w-5 h-5 rounded-full bg-accentSoft text-accent grid place-items-center text-xs font-bold">
              {initials(label(u))}
            </span>
            {label(u)}
            {u === installerId && <span className="text-xs text-faint">· you</span>}
            {protectedIds?.includes(u) ? (
              <span
                className="text-xs text-faint"
                title="Remove approval-owner access before removing this person."
              >
                · owner
              </span>
            ) : (
              <button className={XBTN} title="remove" onClick={() => onRemove(u)}>
                ×
              </button>
            )}
          </span>
        ))}
        <PersonPicker teamId={teamId} allowed={allowed} onChanged={onChanged} />
      </span>
    </div>
  );
}
