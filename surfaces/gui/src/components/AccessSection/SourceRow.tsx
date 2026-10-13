import type { ConnectionScope, SessionConnections } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { labelFor, visualFor, type ConnectorMap } from "../../connectors/visuals";
import { Icon, ICON, AVATAR } from "../Icon";
import { Toggle } from "../Toggle";
import { ScopePicker } from "./ScopePicker";

type Conn = SessionConnections["connected"][number];

/** One connected source: its channels link, how far it may reach, and the per-session switch. */
export function SourceRow({
  c,
  byName,
  channelCount,
  onChannels,
  onScope,
  onToggle,
}: {
  c: Conn;
  byName: ConnectorMap;
  channelCount: number;
  onChannels: () => void;
  onScope: (next: ConnectionScope) => void;
  onToggle: (next: boolean) => void;
}) {
  return (
    <div className="flex items-center gap-2 py-1">
      <ConnectorBadge connector={visualFor(c.connector, "connector", byName)} size={AVATAR.md} />
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium leading-tight truncate">
          <span>{labelFor(c.connector, byName)}</span>
          {c.detail && <span className="text-faint font-normal"> · {c.detail}</span>}
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {byName[c.connector]?.channels && (
            <button
              className="inline-flex items-center gap-0.5 text-xs text-accent hover:underline"
              onClick={onChannels}
            >
              Channels · {channelCount}
              <Icon name="chevronRight" size={ICON.sm} />
            </button>
          )}
          {c.sendable && (
            <ScopePicker
              scope={c.scope ?? "write"}
              // A muted source has no reach to narrow; the control stays
              // visible so the setting is still legible, but inert.
              disabled={!c.enabled}
              onPick={onScope}
            />
          )}
        </div>
      </div>
      <Toggle
        checked={c.enabled}
        onChange={onToggle}
        title="Enabled for this session — tap to mute here"
      />
    </div>
  );
}
