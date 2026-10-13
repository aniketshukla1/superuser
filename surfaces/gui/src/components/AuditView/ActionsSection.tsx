import { useEffect, useState } from "react";
import { getAudit, getSettings, setAuditRetention, verifyAudit, type AuditEvent, type AuditIntegrity } from "../../api";
import { notifyError } from "../../notify";
import { CARD } from "../../ui";
import { BTN_ACCENT, BTN_BORDERED, INPUT, RETENTION, SHOW, type Show } from "./AuditView";
import { AuditRow } from "./AuditRow";

export function ActionsSection() {
  const [events, setEvents] = useState<AuditEvent[] | null>(null);
  const [show, setShow] = useState<Show>("all");
  const [nobodyWatching, setNobodyWatching] = useState(false);
  const [sessionFilter, setSessionFilter] = useState("");
  const [toolFilter, setToolFilter] = useState("");
  const [integrity, setIntegrity] = useState<AuditIntegrity | null | "checking">("checking");
  const [retention, setRetention] = useState<number | null>(null);

  const refresh = (next: { show?: Show; nobodyWatching?: boolean } = {}) => {
    const s = next.show ?? show;
    const q = SHOW.find((x) => x.key === s)?.query ?? {};
    return getAudit({
      limit: 150,
      session_id: sessionFilter.trim() || undefined,
      tool: toolFilter.trim() || undefined,
      nobody_watching: next.nobodyWatching ?? nobodyWatching,
      ...q,
    })
      .then(setEvents)
      .catch(() => setEvents([]));
  };

  const check = () => {
    setIntegrity("checking");
    verifyAudit()
      .then(setIntegrity)
      .catch(() => setIntegrity(null));
  };

  useEffect(() => {
    void refresh();
    check();
    getSettings()
      .then((s) => setRetention(s.audit_retention_days ?? 0))
      .catch(() => setRetention(0));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const changeRetention = async (days: number) => {
    if (retention !== null && days > 0 && (retention === 0 || days < retention)) {
      const ok = window.confirm(`Delete every row older than ${days} days now, and keep only ${days} days from here on?`);
      if (!ok) return;
    }
    const res = await setAuditRetention(days).catch(() => null);
    if (!res?.ok) {
      notifyError("Couldn't change how long the trail is kept.", res?.error);
      return;
    }
    setRetention(res.audit_retention_days ?? days);
    void refresh();
    check();
  };

  return (
    <>
      <div className={CARD + " p-3.5 mb-4 flex items-center gap-3 flex-wrap"} data-testid="audit-integrity">
        <IntegrityLine integrity={integrity} />
        <button className={BTN_BORDERED} disabled={integrity === "checking"} onClick={check}>
          Check again
        </button>
        {retention !== null && (
          <select
            className={INPUT}
            aria-label="How long the trail is kept"
            data-testid="audit-retention"
            value={retention}
            onChange={(e) => void changeRetention(Number(e.target.value))}
          >
            {RETENTION.map((r) => (
              <option key={r.days} value={r.days}>
                {r.label}
              </option>
            ))}
          </select>
        )}
      </div>

      <div className="flex items-center gap-2 flex-wrap mb-4">
        <select
          className={INPUT}
          aria-label="Show"
          data-testid="audit-show"
          value={show}
          onChange={(e) => {
            const s = e.target.value as Show;
            setShow(s);
            void refresh({ show: s });
          }}
        >
          {SHOW.map((s) => (
            <option key={s.key} value={s.key}>
              {s.label}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-2 text-sm text-ink">
          <input
            type="checkbox"
            data-testid="audit-nobody-watching"
            checked={nobodyWatching}
            onChange={(e) => {
              setNobodyWatching(e.target.checked);
              void refresh({ nobodyWatching: e.target.checked });
            }}
          />
          Nobody watching
        </label>
        <input className={INPUT} placeholder="session id" value={sessionFilter} onChange={(e) => setSessionFilter(e.target.value)} />
        <input className={INPUT} placeholder="tool" value={toolFilter} onChange={(e) => setToolFilter(e.target.value)} />
        <button className={BTN_ACCENT} onClick={() => void refresh()}>
          Filter
        </button>
      </div>

      {events === null ? (
        <div className="text-sm text-muted">Loading…</div>
      ) : events.length === 0 ? (
        <div className={CARD + " p-4 text-md text-muted"}>
          {show === "all" && !nobodyWatching ? "No audit events yet." : "Nothing matches these filters."}
        </div>
      ) : (
        <div className="space-y-2" data-testid="audit-rows">
          {events.map((ev) => (
            <AuditRow ev={ev} key={ev.id} />
          ))}
        </div>
      )}
    </>
  );
}

export function IntegrityLine({ integrity }: { integrity: AuditIntegrity | null | "checking" }) {
  if (integrity === "checking") return <span className="text-sm text-muted flex-1 min-w-0">Checking the trail…</span>;
  if (!integrity) return <span className="text-sm text-muted flex-1 min-w-0">Couldn't check the trail.</span>;
  if (!integrity.intact) {
    return (
      <span className="text-sm text-danger flex-1 min-w-0" data-testid="audit-integrity-broken">
        The trail doesn&rsquo;t add up. {integrity.problem}
      </span>
    );
  }
  const legacy = integrity.legacy > 0 ? ` ${integrity.legacy} older rows predate the check.` : "";
  return (
    <span className="text-sm text-ink flex-1 min-w-0" data-testid="audit-integrity-ok">
      {integrity.linked === 0
        ? "Nothing recorded since the trail started checking itself."
        : `Intact — each of ${integrity.linked} rows follows the one before it, and none has been changed.`}
      <span className="text-muted">{legacy}</span>
    </span>
  );
}

export const EVENT_LABEL: Record<string, string> = {
  "action.allowed": "Action",
  "action.refused": "Refused",
  "action.failed": "Failed",
  "action.stopped": "Stopped",
  "action.parked": "Waiting for approval",
  "action.declined": "Not done",
  "assistant.declined": "Assistant declined",
  "turn.stalled": "Model went quiet",
  "policy.loaded": "Boundaries at startup",
  "policy.changed": "Boundaries changed",
  "audit.wiped": "Trail erased",
  "audit.retention": "Old rows removed",
  // First-version kinds, still written for events with no decision behind them.
  sync: "Synced",
  connect: "Connection",
  send: "Sent",
  exec: "Ran",
  approve: "Approved",
  deny: "Denied",
  persona: "Assistant changed",
  memory: "Memory changed",
  connector: "Connector changed",
  settings: "Setting changed",
  schedule: "Scheduled",
  watch: "Watch set",
  external: "Connected app",
};
