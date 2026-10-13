import { useEffect, useRef, useState } from "react";
import type { ApprovalDecision } from "../../types";
import { humanizeApprovalTitle } from "../../humanize";
import { Icon, ICON } from "../Icon";
import { FILE_WRITES, MessagePreview, PreviewBlock, SavePersonaPreview, SaveSkillPreview, TOOL_VERBS, TitleText, approvalActionLabels, permissionLines, scopeNote, shortArgs, type ApprovalItem } from "./SaveSkillPreview";
import { Buttons } from "./Buttons";

export function ApprovalCard({
  item,
  onApprove,
  runTask,
  compact = false,
}: {
  item: ApprovalItem;
  onApprove: (decision: ApprovalDecision) => void;
  // Present when this approval was raised inside an automation run — unlocks the
  // task-persistent "Allow every time" (in-app only, §25).
  runTask?: { id: string; title: string } | null;
  compact?: boolean;
}) {
  const [peek, setPeek] = useState(false);
  // The app is blocked on this decision, so it takes focus and announces itself. It
  // used to do neither: reaching Allow took ten Tab presses from wherever focus
  // happened to be, and nothing was in a live region, so a screen-reader user was
  // never told the run had stopped to ask them something (UX-REPORT §7).
  const cardRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (item.resolved) return;
    const first = cardRef.current?.querySelector<HTMLElement>("button");
    // A frame's delay: the card mounts mid-stream, and taking focus in the same tick
    // as the transcript's own scroll fights it.
    const t = window.setTimeout(() => first?.focus(), 60);
    return () => clearTimeout(t);
  }, [item.resolved]);
  const title = humanizeApprovalTitle(item.name, item.args);
  // The title is a three-part structure for styling; flatten it for the announcement.
  const plainTitle = [title.pre, title.obj, title.post].filter(Boolean).join("").trim();
  const scope = scopeNote(item.name, item.args, item.category);
  const grants = item.name === "create_scheduled_task" ? permissionLines(item.args) : [];
  // "requires approval" is the engine's default boilerplate — only surface a real reason.
  const reason = item.reason && item.reason !== "requires approval" ? item.reason : "";
  const offerStanding = !!(runTask && item.standingTarget);
  const dock = compact ? " approval-dock" : "";

  // §35 compact row: routine workspace writes — one line, preview expands inline from the
  // tool args. Standing/grant flows keep the full card (they carry §25 consent weight).
  const content = typeof item.args?.content === "string" ? item.args.content : "";
  if (FILE_WRITES.has(item.name) && !offerStanding && !grants.length && !item.resolved) {
    return (
      <div
        ref={cardRef}
        className={"approval approval-row" + dock}
        data-testid="approval-row"
        role="alertdialog"
        aria-label={`Permission needed: ${plainTitle}`}
      >
        {/* Announced as well as focused: `alertdialog` covers a reader that follows
            focus, the live region covers one that does not. */}
        <span className="sr-only" role="status" aria-live="polite">
          {`Permission needed: ${plainTitle}`}
        </span>
        <div className="approval-row-line">
          <TitleText line={title} />
          {content && (
            <button className="approval-peek" onClick={() => setPeek((v) => !v)}>
              preview {peek ? "▴" : "▾"}
            </button>
          )}
          <span className="spacer" />
          <Buttons item={item} onApprove={onApprove} runTask={runTask} primaryLabel="Allow" />
        </div>
        {peek && content && <PreviewBlock text={content} />}
        {reason && <div className="approval-reason">{reason}</div>}
      </div>
    );
  }

  return (
    <div
      ref={cardRef}
      className={"approval" + (scope.external ? " approval-external" : "") + dock}
      role="alertdialog"
      aria-label={`Permission needed: ${plainTitle}`}
    >
      <span className="sr-only" role="status" aria-live="polite">
        {`Permission needed: ${plainTitle}`}
      </span>
      <div className="approval-top">
        <div className="approval-heading">
          <span className="approval-ico" title={`Tool: ${item.name}`}>
            <Icon name="shield" size={ICON.md} />
          </span>
          <TitleText line={title} />
        </div>
        <span className={"approval-scope" + (scope.external ? " out" : "")}>{scope.text}</span>
      </div>

      {/* Tool-shaped previews — the proposal, not an args dump. */}
      {item.name === "run_shell" && item.args?.command && (
        <PreviewBlock text={String(item.args.command)} />
      )}
      {FILE_WRITES.has(item.name) && content && <PreviewBlock text={content} />}
      {item.name === "send_file" && (
        <>
          <span className="approval-filechip">
            <span className="ico">
              <Icon name="file" size={ICON.md} />
            </span>
            {String(item.args?.path ?? "").split("/").pop() || "file"}
            {item.args?.as_screenshot ? " · as a PNG screenshot" : ""}
          </span>
          {item.args?.comment && (
            <MessagePreview text={String(item.args.comment)} label="With the message" />
          )}
        </>
      )}
      {item.name === "send_message" && item.args?.text && (
        <MessagePreview text={String(item.args.text)} />
      )}
      {/* save_skill (SKILLS-SPEC §5.2): the arguments ARE the review surface. The same is
          true of an assistant, which is a standing role plus what it may use. */}
      {item.name === "save_skill" && <SaveSkillPreview args={item.args} />}
      {item.name === "save_persona" && <SavePersonaPreview args={item.args} />}
      {/* What goes into the page, before it does. */}
      {item.category === "browser" && item.args?.text && <MessagePreview text={String(item.args.text)} label="Typing" />}

      {grants.length > 0 && (
        <div className="approval-grants" data-testid="approval-grants">
          {grants.map((g, i) => (
            <div className="approval-grant" key={i} data-access={g.access}>
              <span className={"grant-mark" + (g.access === "write" ? " write" : "")}>
                {g.access === "write" ? "✓" : "·"}
              </span>
              <span className="grant-line">
                {TOOL_VERBS[g.tool] || g.tool} <code className="approval-tool">{g.target}</code>
                <span className="grant-note">
                  {g.access === "write" ? " — always allowed once you approve" : " — read-only"}
                </span>
              </span>
            </div>
          ))}
        </div>
      )}
      {/* Long-tail tools: no bespoke preview — fall back to the compact args line. */}
      {!FILE_WRITES.has(item.name) &&
        !["run_shell", "send_message", "send_file", "save_skill", "save_persona"].includes(item.name) &&
        item.category !== "browser" &&
        !grants.length &&
        shortArgs(item.args) && <div className="approval-rest">{shortArgs(item.args)}</div>}
      {reason && <div className="approval-reason">{reason}</div>}

      {item.resolved ? (
        <div className="resolved">Approved: {item.resolved.replace("_", " ")}</div>
      ) : (
        <Buttons
          item={item}
          onApprove={onApprove}
          runTask={runTask}
          primaryLabel={approvalActionLabels(item.name).allow}
          denyLabel={approvalActionLabels(item.name).deny}
        />
      )}
    </div>
  );
}
