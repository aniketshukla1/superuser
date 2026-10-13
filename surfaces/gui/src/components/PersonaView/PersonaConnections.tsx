import type { PersonaDetail } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { labelFor, visualFor, type ConnectorMap } from "../../connectors/visuals";
import { shortPersonaName } from "../../personaScope";
import { AVATAR } from "../Icon";
import { Toggle } from "../Toggle";
import { SEC_H } from "../../ui";
import { BTN_ACCENT, BTN_BORDERED, TAG_CORE, TAG_MCP } from "./TAG_CORE";

/** What it should be connected to, and what new sessions get by default. */
export function PersonaConnections({
  detail,
  personaId,
  byName,
  toggleDefault,
  onOpenIntegrations,
}: {
  detail: PersonaDetail;
  personaId: string;
  byName: ConnectorMap;
  toggleDefault: (connector: string, next: boolean) => void;
  onOpenIntegrations?: () => void;
}) {
  return (
    <>
      {/* connections for full benefit (manifest recommends) */}
      {detail.recommends.length > 0 && (
        <section>
          <div className={`${SEC_H} mb-1`}>Connections for full benefit</div>
          <p className="text-sm text-muted mb-2.5">
            Declared by the persona — wire {shortPersonaName(detail.name, personaId)} into these
            to unlock its full workflow.
          </p>
          <div className="rounded-xl2 border border-line overflow-hidden">
            {detail.recommends.map((r, i) => {
              const isMcp = r.kind === "mcp";
              return (
                <div
                  className={
                    "flex items-center gap-3 p-3 bg-panel" + (i > 0 ? " border-t border-line" : "")
                  }
                  key={`${r.kind}:${r.ref}`}
                >
                  <ConnectorBadge connector={visualFor(r.ref, r.kind, byName)} size={AVATAR.lg} />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-md font-medium">{labelFor(r.ref, byName)}</span>
                      {isMcp ? (
                        <span className={TAG_MCP}>MCP</span>
                      ) : r.tier === "core" ? (
                        <span className={TAG_CORE}>core</span>
                      ) : null}
                    </div>
                    <div className="text-sm text-muted">{r.reason}</div>
                  </div>
                  {r.connected ? (
                    <span className="inline-flex items-center gap-1 text-xs text-ok shrink-0">
                      <span className="w-1.5 h-1.5 rounded-full bg-ok" />
                      connected
                    </span>
                  ) : (
                    <button
                      className={r.tier === "core" && !isMcp ? BTN_ACCENT : BTN_BORDERED}
                      onClick={onOpenIntegrations}
                    >
                      {isMcp ? "Add" : "Connect"}
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </section>
      )}

      {/* persona-default connections (persona → session default) */}
      {/* Optional-chained on purpose. This read is what took the whole screen
          down: the field is typed as a required array, the server never sent it,
          and `.length` on undefined threw inside render — so a missing field did
          not degrade the view, it unmounted it. The server sends it now; the
          guard stays, because a screen should survive a field going missing. */}
      {(detail.default_connections?.length ?? 0) > 0 && (
        <section>
          <div className={`${SEC_H} mb-1`}>New sessions get by default</div>
          <p className="text-sm text-muted mb-2.5">
            When you start a {shortPersonaName(detail.name, personaId)} session these are enabled
            automatically. You can still mute any of them per session.
          </p>
          <div className="space-y-1.5">
            {(detail.default_connections ?? []).map((c) => (
              <div
                className={
                  "flex items-center gap-3 p-2.5 rounded-xl2 border border-line bg-panel" +
                  (c.connected ? "" : " opacity-50")
                }
                key={c.connector}
              >
                <ConnectorBadge connector={visualFor(c.connector, "connector", byName)} size={AVATAR.lg} />
                <div className="flex-1 text-md font-medium">
                  {labelFor(c.connector, byName)}
                  {!c.connected && (
                    <span className="text-xs text-faint font-normal"> · connect to enable</span>
                  )}
                </div>
                <Toggle
                  checked={c.enabled}
                  disabled={!c.connected}
                  onChange={(next) => toggleDefault(c.connector, next)}
                  title={c.connected ? "On by default for new sessions" : "Connect this first"}
                />
              </div>
            ))}
          </div>
        </section>
      )}
    </>
  );
}
