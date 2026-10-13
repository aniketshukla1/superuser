import type { SessionConnections } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { labelFor, visualFor, type ConnectorMap } from "../../connectors/visuals";
import { AVATAR } from "../Icon";
import { SEC_H } from "../../ui";
import { BTN_ACCENT, BTN_BORDERED, TAG_CORE } from "./ScopePicker";

/** Connectors this assistant recommends and the session does not have yet. */
export function RecommendedList({
  recommended,
  byName,
  onConnect,
}: {
  recommended: SessionConnections["recommended"];
  byName: ConnectorMap;
  onConnect: (connector: string) => void;
}) {
  if (recommended.length === 0) return null;
  return (
    <div>
      <div className={`${SEC_H} mb-1.5`}>Recommended</div>
      <div className="space-y-1">
        {recommended.map((r) => (
          <div className="flex items-center gap-2 py-1" key={r.connector}>
            <ConnectorBadge connector={visualFor(r.connector, "connector", byName)} size={AVATAR.md} />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-1.5 text-sm font-medium leading-tight">
                <span className="truncate">{labelFor(r.connector, byName)}</span>
                {r.tier === "core" && <span className={TAG_CORE}>core</span>}
              </div>
              <div className="text-xs text-faint truncate" title={r.reason}>
                {r.reason}
              </div>
            </div>
            <button
              className={r.tier === "core" ? BTN_ACCENT : BTN_BORDERED}
              onClick={() => onConnect(r.connector)}
            >
              Connect
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
