import type { EgressRequest } from "../../api";
import { exactTime, timeAgo } from "../../when";

/** A status is a result; a refusal is the switch working. Neither is a failure unless the
 *  server said so. */
export function statusClass(r: EgressRequest): string {
  if (r.blocked) return "net-status refused";
  if (r.status === null) return "net-status";
  return "net-status" + (r.status >= 400 ? " bad" : "");
}

export function RequestRow({ r }: { r: EgressRequest }) {
  return (
    <div className="net-request">
      <span className={statusClass(r)}>{r.blocked ? "refused" : (r.status ?? "—")}</span>
      <span className="net-method">{r.method}</span>
      <span className="net-url">
        {r.host}
        <span className="text-faint">{r.path}</span>
      </span>
      <span className="net-when" title={exactTime(r.at)}>
        {r.purpose} · {timeAgo(r.at)}
      </span>
    </div>
  );
}
