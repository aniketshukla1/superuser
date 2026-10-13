import type { BoundaryReport } from "../../api";
import { when } from "./INPUT";

export function TestReport({ report }: { report: BoundaryReport }) {
  if (report.scanned === 0) {
    return (
      <div className="text-sm text-muted mt-3" data-testid="boundaries-report">
        There are no recorded actions to test against yet. Once your assistants have done some work, this shows what these
        rules would have changed.
      </div>
    );
  }
  const noun = (n: number) => `${n} action${n === 1 ? "" : "s"}`;
  const summary =
    report.would_refuse === 0 && report.would_allow === 0
      ? `Tested against the last ${noun(report.scanned)}: these rules decide every one of them the same way.`
      : `Tested against the last ${noun(report.scanned)}: ` +
        [
          report.would_refuse > 0 ? `${noun(report.would_refuse)} that went ahead would be refused` : "",
          report.would_allow > 0 ? `${noun(report.would_allow)} that were refused would be allowed` : "",
        ]
          .filter(Boolean)
          .join(", and ") +
        ".";
  return (
    <div className="mt-3" data-testid="boundaries-report">
      <div className="text-sm text-ink">{summary}</div>
      {report.changes.length > 0 && (
        <div className="mt-2 divide-y divide-line">
          {report.changes.map((c, i) => (
            <div key={i} className="py-2 flex items-baseline gap-2 text-sm" data-testid="boundaries-change">
              <span
                className={
                  "text-xs px-1.5 rounded-full shrink-0 " +
                  (c.would === "refused" ? "bg-dangerSoft text-danger" : "bg-okSoft text-ok")
                }
              >
                {c.would === "refused" ? "Would refuse" : "Would allow"}
              </span>
              <span className="min-w-0 flex-1 break-words text-ink">{c.what}</span>
              <span className="text-xs text-faint shrink-0">
                {c.rule_name || (c.rule ? c.rule : "")}
                {c.persona ? ` · ${c.persona}` : ""} · {when(c.at)}
              </span>
            </div>
          ))}
          {report.would_refuse + report.would_allow > report.changes.length && (
            <div className="py-2 text-xs text-faint">
              Showing the first {report.changes.length} of {report.would_refuse + report.would_allow}.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
