import { AVATAR } from "../../Icon";
import { useEffect, useState } from "react";
import { getGithubStatus, getSubscriptions, type GithubStatus, type Subscription } from "../../../api";
import { ConnectorBadge } from "../../../connectors/ConnectorIcon";
import { AddConnectionModal } from "../AddConnectionModal";
import type { DetailProps } from "../ConnectorsSection";
import { ToolsDisclosure } from "../ToolsDisclosure";
import { FOOT, GRP, GRP_H, PILL_ACCENT, ROW } from "../ui";
import { InstallationGroup, ListeningRows } from "./InstallationGroup";

// The GitHub detail page (github-relay-spec §8), the Slack page's shape: one
// group per App INSTALLATION (the allow-list scope) — People (sender logins
// allowed to trigger work) · Waiting (parked mentions) · per-installation
// disconnect — plus a page-level Listening group (a subscription names a repo
// thread, which the GUI can't map back to an installation). Adding an
// installation goes through the ONE entry point: header button → modal.

export const LABEL = "text-sm text-muted w-24 shrink-0";

/** The relay status line, one honest layer at a time (the Slack rule). */
export function relayHealth(gh: GithubStatus | null): { dot: string; text: string } {
  if (!gh) return { dot: "bg-ok", text: "Live · managed relay" };
  if (!gh.signed_in)
    return { dot: "bg-warnInk", text: "Sign-in needed — relaying is paused" };
  if (gh.relay.state === "offline")
    return { dot: "bg-faint/60", text: "Offline — can't reach the relay" };
  if (gh.relay.state === "reconnecting")
    return { dot: "bg-warnInk", text: "Reconnecting to the relay…" };
  return { dot: "bg-ok", text: "Live · managed relay" };
}

export function GithubDetail({ c, cloud, onChanged }: DetailProps) {
  const [adding, setAdding] = useState(false);
  const [subs, setSubs] = useState<Subscription[]>([]);
  const [status, setStatus] = useState<GithubStatus | null>(null);
  const load = () => {
    getSubscriptions().then(setSubs).catch(() => setSubs([]));
    getGithubStatus().then(setStatus).catch(() => setStatus(null));
  };
  useEffect(() => {
    load();
  }, [c.name]);

  const relay = c.mode === "relay";
  const installations = c.installations ?? [];
  const changed = () => {
    onChanged();
    load();
  };
  const listening = subs.filter((s) => s.channel.startsWith("github:"));

  return (
    <div data-testid="github-installations">
      <div className="flex items-center gap-3.5 mb-5">
        <ConnectorBadge connector={c} size={AVATAR.xl} title="GitHub" />
        <div className="min-w-0 flex-1">
          <h2 className="text-xl font-semibold tracking-tight leading-tight">GitHub</h2>
          <div className="text-sm text-muted flex items-center gap-1.5">
            {c.connected ? (
              <>
                <span
                  className={
                    "w-2 h-2 rounded-full " + (relay ? relayHealth(status).dot : "bg-ok")
                  }
                />
                <span data-testid="github-mode-badge">
                  {relay
                    ? relayHealth(status).text
                    : "Connected · personal access token"}
                </span>
              </>
            ) : (
              <span>Not connected</span>
            )}
          </div>
        </div>
        {(relay || !c.connected) && (
          <button
            className={PILL_ACCENT}
            data-testid="add-installation-btn"
            onClick={() => setAdding(true)}
          >
            ＋ Add installation
          </button>
        )}
      </div>

      {!c.connected && (
        <div className={GRP}>
          <div className={ROW + " text-sm text-muted"}>
            One @ocw-agent App, installed per account or org — you pick the repos on
            GitHub; each installation keeps its own allow-list.
            {cloud?.signed_in ? "" : " One-click needs cloud sign-in; a PAT works without it."}
          </div>
        </div>
      )}

      {relay &&
        installations.map((inst) => (
          <InstallationGroup
            key={inst.installation_id}
            c={c}
            inst={inst}
            tokenOk={status?.installs?.[inst.installation_id]?.token_ok !== false}
            onChanged={changed}
          />
        ))}

      {/* Manual PAT: request/response tools only — no inbound triggers. */}
      {c.connected && !relay && (
        <div className={GRP} data-testid="github-manual-card">
          <div className={ROW + " text-sm text-muted"}>
            Personal access token · tools only. Install the GitHub App to let
            @-mentions and the agent label reach this computer.
          </div>
        </div>
      )}

      {relay && listening.length > 0 && (
        <>
          <div className={GRP_H}>Listening</div>
          <div className={GRP}>
            <ListeningRows subs={listening} onChanged={changed} />
          </div>
        </>
      )}

      <ToolsDisclosure c={c} />
      {c.connected && relay && (
        <div className={FOOT + " mt-2"}>
          Triggers: @ocw-agent mentions and the “ocw-agent” label. The agent replies as
          ocw-agent[bot].
        </div>
      )}

      {adding && (
        <AddConnectionModal
          c={c}
          cloud={cloud}
          title="Add an installation"
          onClose={() => setAdding(false)}
          onChanged={changed}
        />
      )}
    </div>
  );
}
