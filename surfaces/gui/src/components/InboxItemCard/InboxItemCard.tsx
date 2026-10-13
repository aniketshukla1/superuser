import type { ReactNode } from "react";
import type { InboxItem } from "../../api";
import { humanizeApprovalTitle, type HumanLine } from "../../humanize";
import { approvalActionLabels, PreviewBlock, SavePersonaPreview, SaveSkillPreview, scopeNote, TitleText } from "../ApprovalCard";
import { Icon, ICON, type IconName } from "../Icon";
import { parkedApproval, primaryArg, requesterLabel, secondaryArgs } from "../../inboxText";
import { exactTime, timeAgo } from "../../when";
import { BTN_ACCENT, BTN_BORDERED, BTN_PRIMARY, BTN_QUIET, KIND } from "./KIND";
import { QuestionAnswer } from "./QuestionAnswer";

export function InboxItemCard({
  item,
  onResolve,
  chip,
  compact,
}: {
  item: InboxItem;
  onResolve: (id: string, resolution: string) => void;
  chip?: ReactNode; // optional "go to session" affordance (shown in the Inbox list, not inline)
  compact?: boolean;
}) {

  // §35: parked approvals wear the SAME humanized dress as the live card — one decision, one
  // dialect. Rows parked before tool+arguments were stored in `data` (and MCP calls) are read
  // back out of their title and body by `parkedApproval`, so they get the dress too instead of
  // a heading that named the tool twice over a block of doubly-escaped JSON.
  const parked = parkedApproval(item);
  const tool = parked?.tool || "";
  const headline: HumanLine | null = parked
    ? parked.server
      ? { pre: "Use ", obj: parked.server === tool ? parked.server : `${parked.server} › ${tool}` }
      : tool
        ? humanizeApprovalTitle(tool, parked.args)
        : null
    : null;
  const scope = parked
    ? parked.server
      ? { text: `goes through the ${parked.server} tool server`, external: true }
      : scopeNote(tool, parked.args)
    : null;
  // A proposed skill or assistant has a review surface of its own, so neither is reduced to
  // one argument and a list of the rest.
  const reviewed = tool === "save_skill" || tool === "save_persona";
  const main = parked && !reviewed ? primaryArg(parked.args) : null;
  const rest = parked && !reviewed ? secondaryArgs(parked.args, main?.key) : [];
  const kind = KIND[item.kind] ?? { label: item.kind, icon: "inbox" as IconName };

  return (
    <div
      className={
        compact
          ? "max-w-3xl mx-auto mb-2.5 rounded-xl2 border border-lineStrong bg-panel px-4 py-3.5 shadow-[0_8px_24px_rgba(0,0,0,0.08)]"
          : "mb-2.5 rounded-xl2 border border-line bg-panel px-3.5 py-3"
      }
      data-testid="inbox-item"
    >
      {/* What it is, who is asking, and how long it has been waiting — none of which the
          card said before. An approval a day old is a different decision from one raised a
          minute ago by the conversation you just left. */}
      <div className="inbox-item-meta">
        <Icon name={kind.icon} size={ICON.sm} />
        <span className="inbox-item-kind">{kind.label}</span>
        <span aria-hidden>·</span>
        <span className="truncate">{requesterLabel(item)}</span>
        {item.created_at && (
          <>
            <span aria-hidden>·</span>
            <span className="shrink-0" title={exactTime(item.created_at)}>
              {timeAgo(item.created_at)}
            </span>
          </>
        )}
      </div>
      {headline ? (
        <div className="flex items-start justify-between gap-3 mt-1">
          <TitleText line={headline} />
          {scope && (
            <span className={"text-xs whitespace-nowrap pt-0.5 " + (scope.external ? "text-warnInk" : "text-faint")}>
              {scope.text}
            </span>
          )}
        </div>
      ) : (
        <div className="text-lg font-semibold mt-1 leading-snug">{item.title}</div>
      )}
      {tool === "save_skill" && parked ? (
        // Parked skill proposals wear the same review surface as the live card (§5.2).
        <SaveSkillPreview args={parked.args} />
      ) : tool === "save_persona" && parked ? (
        // And a parked assistant wears the assistant's, for the same reason: one decision,
        // one dialect, wherever it is met.
        <SavePersonaPreview args={parked.args} />
      ) : main ? (
        // A message is approved by reading it — all of it: `primaryArg` hands over the whole
        // text, and a reply the email bot drafted is usually longer than the summary line.
        <PreviewBlock text={main.text} mono={main.mono} />
      ) : !parked && item.body ? (
        <div className="text-md text-muted mt-1 whitespace-pre-wrap">{item.body}</div>
      ) : null}
      {rest.length > 0 && (
        <div className="inbox-item-args" data-testid="inbox-item-args">
          {rest.map((line) => (
            <div key={line}>{line}</div>
          ))}
        </div>
      )}
      {chip}
      {item.kind === "approval" ? (
        <div className="flex items-center gap-2 mt-2.5 flex-wrap">
          {/* One treatment for every approval: an outlined Allow, not a filled one. A column of
              identical filled buttons is the shape that gets clicked without reading. */}
          <button className={BTN_ACCENT} onClick={() => onResolve(item.id, "allow")}>
            {approvalActionLabels(tool || undefined).allow}
          </button>
          {/* Task-persistent standing grant (§25) — present only when the approval was
              raised inside an automation run AND the call can carry a tool+target rule.
              In-app only by construction: Slack mirrors render Approve/Deny buttons. */}
          {item.data?.task_id && item.data?.standing_target && (
            <button
              className={BTN_BORDERED}
              title={`Always allow against ${item.data.standing_target} for “${item.data.task_title || "this automation"}” — revoke any time on its Automations page`}
              onClick={() => onResolve(item.id, "always_task")}
            >
              Allow every time
            </button>
          )}
          <button className={BTN_QUIET} onClick={() => onResolve(item.id, "deny")}>
            {approvalActionLabels(tool || undefined).deny}
          </button>
        </div>
      ) : item.kind === "question" ? (
        <QuestionAnswer item={item} onResolve={onResolve} />
      ) : item.kind === "directory" ? (
        <div className="flex items-center gap-2 mt-2.5">
          <button
            className={BTN_PRIMARY}
            disabled={!item.data?.path}
            title={item.data?.path || "No folder was suggested"}
            onClick={() =>
              onResolve(
                item.id,
                JSON.stringify({ granted: true, path: item.data?.path || "", writable: !!item.data?.writable }),
              )
            }
          >
            {item.data?.path ? "Grant" : "Grant (no folder)"}
          </button>
          <button className={BTN_BORDERED} onClick={() => onResolve(item.id, JSON.stringify({ granted: false }))}>
            Deny
          </button>
        </div>
      ) : item.kind === "plan" ? (
        <div className="flex items-center gap-2 mt-2.5">
          <button
            className={BTN_PRIMARY}
            onClick={() => onResolve(item.id, JSON.stringify({ approved: true, mode: "interactive" }))}
          >
            Approve
          </button>
          <button
            className={BTN_BORDERED}
            onClick={() => onResolve(item.id, JSON.stringify({ approved: false, feedback: "" }))}
          >
            Reject
          </button>
        </div>
      ) : (
        <div className="flex items-center gap-2 mt-2.5">
          <button className={BTN_BORDERED} onClick={() => onResolve(item.id, "seen")}>
            Dismiss
          </button>
        </div>
      )}
    </div>
  );
}
