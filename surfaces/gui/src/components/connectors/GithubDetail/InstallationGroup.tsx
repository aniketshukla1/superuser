import { useState } from "react";
import { disallowUser, disconnectGithubInstallation, resolveUnauthorized, unsubscribeChannel, type Connector, type GithubInstallation, type ParkedMessage, type Subscription } from "../../../api";
import { GRP, GRP_H, PILL_ACCENT, PILL_LINE, ROW, TAG_WARN, XBTN } from "../ui";
import { LABEL } from "./GithubDetail";

export function InstallationGroup({
  c,
  inst,
  tokenOk,
  onChanged,
}: {
  c: Connector;
  inst: GithubInstallation;
  tokenOk: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const parked = (c.unauthorized ?? []).filter((m) => m.team_id === inst.installation_id);
  const empty = inst.allowed_users.length === 0 && parked.length === 0;

  const disconnect = async () => {
    setBusy(true);
    await disconnectGithubInstallation(inst.installation_id);
    setBusy(false);
    onChanged();
  };

  return (
    <div data-testid={`github-install-${inst.installation_id}`}>
      <div className={GRP_H + " flex items-center gap-2"}>
        <span>
          {inst.account_login}{" "}
          <span className="font-normal text-faint" title={`installation ${inst.installation_id}`}>
            · {inst.repo_selection === "all" ? "all repos" : "selected repos"}
          </span>
        </span>
        {!tokenOk && (
          <span className={TAG_WARN} data-testid={`token-warn-${inst.installation_id}`}>
            ⚠ Installation revoked — reinstall
          </span>
        )}
      </div>
      <div className={GRP}>
        {empty ? (
          <div className={ROW}>
            <span className="min-w-0 flex-1 text-sm text-muted">
              No one allowed yet — @ocw-agent mentions show up here for your OK.
            </span>
            <DisconnectBtn id={inst.installation_id} busy={busy} onClick={disconnect} />
          </div>
        ) : (
          <>
            <PeopleRow
              allowed={inst.allowed_users}
              installationId={inst.installation_id}
              onChanged={onChanged}
            />
            {parked.map((m) => (
              <WaitingRow key={m.id} m={m} onChanged={onChanged} />
            ))}
            <div className={ROW}>
              <span className="flex-1" />
              <DisconnectBtn id={inst.installation_id} busy={busy} onClick={disconnect} />
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export function DisconnectBtn({ id, busy, onClick }: { id: string; busy: boolean; onClick: () => void }) {
  return (
    <button
      className="text-sm text-danger/80 hover:text-danger shrink-0"
      data-testid={`disconnect-install-${id}`}
      title="Stops relaying this installation to this computer. The App stays installed on GitHub."
      onClick={onClick}
      disabled={busy}
    >
      {busy ? "Disconnecting…" : "Disconnect installation"}
    </button>
  );
}

export function PeopleRow({
  allowed,
  installationId,
  onChanged,
}: {
  allowed: string[];
  installationId: string;
  onChanged: () => void;
}) {
  return (
    <div className={ROW}>
      <span className={LABEL}>People</span>
      <span className="min-w-0 flex-1 flex flex-wrap items-center gap-1.5">
        {allowed.length === 0 && (
          <span className="text-sm text-faint">nobody yet — approve a waiting sender below</span>
        )}
        {allowed.map((login) => (
          <span
            key={login}
            className="inline-flex items-center gap-1.5 pl-2 pr-2 py-0.5 rounded-full bg-paper border border-line text-sm"
          >
            {/* GitHub logins ARE the readable identity — no resolution needed. */}
            @{login}
            <button
              className={XBTN}
              title="remove"
              onClick={() => disallowUser("github", login, installationId).then(onChanged)}
            >
              ×
            </button>
          </span>
        ))}
      </span>
    </div>
  );
}

export function WaitingRow({ m, onChanged }: { m: ParkedMessage; onChanged: () => void }) {
  const act = async (action: "dismiss" | "allow" | "allow_deliver") => {
    await resolveUnauthorized("github", m.id, action);
    onChanged();
  };
  return (
    <div className={ROW + " bg-warnSoft/25"} data-testid={`waiting-${m.id}`}>
      <span className={LABEL}>Waiting</span>
      <span className="min-w-0 flex-1">
        <span className="font-medium text-md">@{m.user_name || m.user_id}</span>{" "}
        <span className="text-sm text-muted">in {m.chat_name || m.chat_id}</span>
        <span className="block text-sm text-muted truncate">“{m.text}”</span>
      </span>
      <button
        className={PILL_ACCENT + " !py-1"}
        data-testid={`parked-allow-deliver-${m.id}`}
        title="Allow the sender and deliver this mention now"
        onClick={() => act("allow_deliver")}
      >
        Allow & deliver
      </button>
      <button
        className={PILL_LINE + " !py-1"}
        data-testid={`parked-allow-${m.id}`}
        title="Allow the sender; this mention is discarded"
        onClick={() => act("allow")}
      >
        Allow
      </button>
      <button className={XBTN + " px-1"} data-testid={`parked-dismiss-${m.id}`} title="Dismiss" onClick={() => act("dismiss")}>
        ×
      </button>
    </div>
  );
}

export function ListeningRows({ subs, onChanged }: { subs: Subscription[]; onChanged: () => void }) {
  return (
    <div className={ROW} data-testid="listening-github">
      <span className={LABEL}>Listening</span>
      <span className="min-w-0 flex-1 space-y-1">
        {subs.map((s) => (
          <span key={s.session_id + s.channel} className="flex items-center gap-2 text-sm">
            <span className="font-medium truncate" title={s.session_id}>
              {s.session_title || s.session_id}
            </span>
            <span className="text-faint">←</span>
            <span className="text-muted truncate" title={s.channel}>
              {s.channel.replace(/^github:/, "")}
            </span>
            <button
              className={XBTN + " ml-auto"}
              title="Unsubscribe this session"
              onClick={async () => {
                await unsubscribeChannel(s.session_id, s.channel);
                onChanged();
              }}
            >
              ×
            </button>
          </span>
        ))}
      </span>
    </div>
  );
}
