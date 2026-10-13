import { useState } from "react";
import type { Automation, AutomationRun } from "../../api";
import { Icon, ICON } from "../Icon";
import { exactTime } from "../../when";
import { plural, runStatus } from "../../scheduleText";
import { plainText } from "../../plainText";
import { Markdown } from "../Markdown";
import { TRIGGER, fmt } from "./Props";

/** What each run produced, newest first, with the new ones counted and set apart. */
export function RunList({
  runs,
  seenMark,
  task,
  onOpenRun,
}: {
  runs: AutomationRun[];
  seenMark: number | null;
  task: Automation;
  onOpenRun: (sessionId: string, workspace: string, agent: string, task?: { id: string; title: string }) => void;
}) {
  // Which run is showing its full result. One at a time: this is a list you scan.
  const [expandedRun, setExpandedRun] = useState<string | null>(null);
  return (
    <>
      <div className="sa-sub">Runs</div>
      <div className="text-sm text-faint mb-2">
        What each run produced. Open one to see the steps it took, or to ask a follow-up.
      </div>
      {runs.length === 0 && <div className="dim">No runs yet.</div>}
      {/* "New" is said once, as a count, with a rule where the new runs end. It used to be a
          pill on every row — eight identical badges on eight rows, which marks nothing. */}
      {(() => {
        const fresh = seenMark === null ? 0 : runs.filter((r) => r.started_at > seenMark).length;
        return fresh > 0 ? (
          <div className="run-new-note" data-testid="run-new">
            {plural(fresh, "new run")} since you last looked
          </div>
        ) : null;
      })()}
      {runs.map((r, ri) => {
        const open = expandedRun === r.run_id;
        const isNew = seenMark !== null && r.started_at > seenMark;
        const prevNew = ri > 0 && seenMark !== null && runs[ri - 1].started_at > seenMark;
        const status = runStatus(r.status, r.started_at);
        return (
          <div key={r.run_id}>
          {!isNew && prevNew && <div className="run-divider">Earlier</div>}
          <div className={"sched-run open" + (isNew ? " is-new" : "")}>
            {/* Reading a run and leaving for its conversation are different intents,
                and the whole row used to be the second one. So the two lines of
                result you can see were bait: clicking them to read the rest threw
                you into a chat session per run, which is what "it opens a new tab"
                described. The row expands in place now; going to the conversation
                is the control at the end, on purpose. */}
            <button
              className="sched-run-row"
              aria-expanded={open}
              data-testid={`run-row-${r.run_id}`}
              onClick={() => setExpandedRun(open ? null : r.run_id)}
            >
              <span>
                <span title={exactTime(r.started_at)}>{fmt(r.started_at)}</span> ·{" "}
                <span
                  className={"run-" + status.tone}
                  title={status.tone === "stale" ? "This run stopped reporting — Superuser may have closed while it was running." : undefined}
                >
                  {status.label}
                </span>{" "}
                · {TRIGGER[r.trigger] ?? r.trigger}
                {r.artifacts.length > 0 && <span className="dim"> · {plural(r.artifacts.length, "file")}</span>}
              </span>
              <Icon name={open ? "chevronDown" : "chevronRight"} size={ICON.md} className="sched-run-chev" />
            </button>
            {/* Collapsed: the words of the result, two lines, ending in an ellipsis. Open: the
                result rendered. The preview used to print markdown source — asterisks and
                hashes — and cut off mid-word with nothing to say it had. */}
            {r.result_text &&
              (open ? (
                <div className="sched-run-result">
                  <Markdown text={r.result_text} />
                </div>
              ) : (
                <div className="sched-run-peek">{plainText(r.result_text)}</div>
              ))}
            {r.error && <div className="mcp-error">{r.error}</div>}
            {open &&
              (r.has_conversation === false ? (
                /* Older runs have a result and nothing behind it: the scheduler
                   wrote the files but never recorded the transcript, so opening
                   one landed on an empty chat. Say so instead of offering a door
                   onto nothing. */
                <div className="sched-run-noconv" data-testid={`run-noconv-${r.run_id}`}>
                  No transcript was recorded for this run — only what it produced, above.
                </div>
              ) : (
                r.session_id && (
                  <button
                    className="ctl sched-run-go"
                    data-testid={`run-open-${r.run_id}`}
                    onClick={() =>
                      onOpenRun(r.session_id, task.workspace, task.agent, { id: task.id, title: task.title })
                    }
                  >
                    Open the conversation ›
                  </button>
                )
              ))}
          </div>
          </div>
        );
      })}
    </>
  );
}
