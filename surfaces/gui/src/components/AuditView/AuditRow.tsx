import type { AuditEvent } from "../../api";
import { CARD } from "../../ui";
import { EVENT_LABEL } from "./ActionsSection";

export const STARTED_BY: Record<string, string> = {
  person: "you",
  approval: "your approval",
  schedule: "a schedule",
  watch: "a watch",
  message: "a message",
  subagent: "a subagent",
  api: "the API",
  startup: "startup",
};

export function AuditRow({ ev }: { ev: AuditEvent }) {
  // A first-version row carries none of this and keeps its original shape below.
  if (!ev.event) return <LegacyRow ev={ev} />;
  const d = ev.decision;
  const refused = ev.status === "refused";
  const wouldRefuse = !!d && !d.allowed && d.carried_out;
  const failed = !refused && ev.ok === false;
  const label = EVENT_LABEL[ev.event] ?? ev.event;
  const tool = ev.subject?.tool;
  return (
    <div className={CARD + " p-3.5"} data-testid="audit-row">
      <div className="flex items-center gap-2 flex-wrap">
        <span className={"text-sm font-medium " + (refused || failed ? "text-danger" : "text-ink")}>{label}</span>
        {tool && <span className="font-mono text-xs text-muted">{tool}</span>}
        {!tool && ev.target && <span className="font-mono text-xs text-muted break-all">{ev.target}</span>}
        {wouldRefuse && (
          <span className="text-xs px-1.5 rounded-full bg-warnSoft text-warnInk" title="Recording only — it went ahead">
            a boundary would refuse this
          </span>
        )}
        {ev.unwatched && (
          <span className="text-xs px-1.5 rounded-full bg-tealSoft text-tealInk" data-testid="audit-unwatched">
            nobody watching
          </span>
        )}
        {ev.subject?.tainted && (
          <span className="text-xs text-faint" title="The run had read content from outside this machine before this">
            after reading outside content
          </span>
        )}
        <span className="text-xs text-faint ml-auto">{new Date(ev.timestamp).toLocaleString()}</span>
      </div>
      {ev.detail && <div className="text-sm text-ink mt-1 break-words">{ev.detail}</div>}
      {d && !d.allowed && (d.rule || d.rule_name) && (
        <div className="text-xs text-muted mt-1 break-words" data-testid="audit-rule">
          {d.rule_name ? `Boundary “${d.rule_name}”` : "Boundary"} <code className="text-muted">{d.rule}</code>
          {d.reason ? ` — ${d.reason}` : ""}
        </div>
      )}
      <div className="text-xs text-faint mt-1">
        {[
          ev.persona || (ev.actor && ev.actor !== ev.session_id ? ev.actor : null),
          ev.initiator?.kind ? `started by ${STARTED_BY[ev.initiator.kind] ?? ev.initiator.kind}` : null,
          ev.session_id && ev.session_id !== "user" ? `session ${ev.session_id}` : null,
        ]
          .filter(Boolean)
          .join(" · ")}
      </div>
    </div>
  );
}

export function LegacyRow({ ev }: { ev: AuditEvent }) {
  return (
    <div className={CARD + " p-3.5"}>
      <div className="flex items-center gap-2 flex-wrap">
        <span className="font-mono text-sm font-medium text-ink">{ev.tool}</span>
        <span className="text-xs text-faint">
          {ev.connector || "tool"} · {ev.stage || ev.status || "event"} · {ev.timestamp}
        </span>
      </div>
      <div className="text-xs text-muted mt-0.5">
        session {ev.session_id || "-"} {ev.approval ? `· ${ev.approval}` : ""} {ev.status ? `· ${ev.status}` : ""}
      </div>
      {ev.resource && <div className="text-xs text-faint mt-0.5">resource: {ev.resource}</div>}
      {ev.args && Object.keys(ev.args).length > 0 && (
        <div className="font-mono text-xs text-muted mt-1.5 break-words">{formatAuditArgs(ev.args)}</div>
      )}
      {(ev.reason || ev.result_preview) && <div className="text-xs text-faint mt-1">{ev.reason || ev.result_preview}</div>}
    </div>
  );
}

export function formatAuditArgs(args: Record<string, any>) {
  return Object.entries(args)
    .map(([k, v]) => `${k}=${typeof v === "string" ? v : JSON.stringify(v)}`)
    .join("  ");
}
