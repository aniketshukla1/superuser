import { useState } from "react";
import type { ApprovalDecision, Item } from "../../types";
import type { HumanLine } from "../../humanize";
import { FeedbackButtons, FeedbackReasons, useFeedbackVerdict } from "../Feedback";
import type { FeedbackReason } from "../../api";
import { Icon, ICON } from "../Icon";
import { BubbleMeta } from "./BubbleMeta";

/** The row under a reply — copy, read aloud, retry, the verdict, the time — and, once a
 *  thumbs-down is pressed, the reasons under it. One component so the verdict's state is
 *  shared by the two places it shows. */
export function ReplyMeta({
  text,
  ts,
  onRetry,
  rateable,
  sessionId,
  reasons,
  given,
  onGiven,
}: {
  text: string;
  ts?: number;
  onRetry?: () => void;
  rateable: boolean;
  sessionId?: string;
  reasons: FeedbackReason[];
  given?: "up" | "down";
  onGiven: (verdict: "up" | "down") => void;
}) {
  const fb = useFeedbackVerdict({ sessionId, messageAt: ts, onGiven });
  const rating = rateable && fb.ready;
  return (
    <>
      <BubbleMeta
        text={text}
        ts={ts}
        align="left"
        speakable
        onRetry={onRetry}
        extra={
          rating && !fb.picking ? (
            <FeedbackButtons
              given={given}
              busy={fb.busy}
              onUp={() => void fb.send("up")}
              onDown={() => fb.setPicking(true)}
            />
          ) : null
        }
      />
      {rating && fb.picking && (
        // Padded to clear the row above (see .feedback-under): the row still shows on hover.
        <div className="feedback-under">
          <FeedbackReasons
            reasons={reasons}
            busy={fb.busy}
            note={fb.note}
            onNote={fb.setNote}
            onPick={(id) => void fb.send("down", id)}
            onCancel={() => fb.setPicking(false)}
          />
        </div>
      )}
    </>
  );
}

// Reasoning-model thinking text (model-layer roadmap item 4): a quiet disclosure —
// collapsed by default, the trace one click away. `live` = still streaming (pulsing label);
// App renders that variant above the transcript, this one rides a finalized assistant item.
export function ThinkingBlock({ text, live }: { text: string; live?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="thinking">
      <button
        className="thinking-head"
        onClick={() => setOpen((v) => !v)}
        data-testid="thinking-toggle"
      >
        <Icon name="chevronDown" size={ICON.sm} className={"thinking-caret" + (open ? " open" : "")} />
        <span className={live ? "thinking-live" : undefined}>
          {live ? "Thinking…" : "Thought process"}
        </span>
      </button>
      {open && (
        <div className="thinking-body" data-testid="thinking-body">
          {text}
        </div>
      )}
    </div>
  );
}

export type ToolItem = Extract<Item, { kind: "tool" }>;
export type ApprovalItem = Extract<Item, { kind: "approval" }>;
export type AssistantItem = Extract<Item, { kind: "assistant" }>;
export type TurnItem = ToolItem | ApprovalItem | AssistantItem;

// TurnGroup (§33, absorbs §7's StepGroup): the whole user-message → final-answer span collapses
// as ONE disclosure — "N steps" — with the agent's narration (assistant text followed by more
// activity in the same turn) and humanized one-line steps interleaved inside. The final assistant
// text renders as a normal bubble OUTSIDE the group (see the flush logic in Transcript below).
// Approvals fold into their tool's row as a chip; an approval with no executed call (typically
// declined) keeps its own "Wanted to …" row. Raw args+result stay one click away per row.

export type TurnRow =
  | { type: "narr"; text: string }
  | { type: "step"; tool: ToolItem; approval?: ApprovalItem }
  | { type: "ask"; approval: ApprovalItem };

export function buildRows(items: TurnItem[]): TurnRow[] {
  // First pass: tool rows in order; then pair each resolved approval with the nearest
  // same-name tool that doesn't have one yet (approvals may stream before or after their call).
  const rows: TurnRow[] = items
    .filter((it): it is ToolItem | AssistantItem => it.kind !== "approval")
    // Thinking-only assistant items (no text) carry nothing narratable — skip the row.
    .filter((it) => it.kind !== "assistant" || it.text)
    .map((it) =>
      it.kind === "assistant" ? { type: "narr" as const, text: it.text } : { type: "step" as const, tool: it },
    );
  const approvals = items.filter((it): it is ApprovalItem => it.kind === "approval");
  for (const ap of approvals) {
    const at = items.indexOf(ap);
    let bestRow: Extract<TurnRow, { type: "step" }> | null = null;
    let bestDist = Infinity;
    for (let i = 0; i < items.length; i++) {
      const it = items[i];
      if (it.kind !== "tool" || it.name !== ap.name) continue;
      const row = rows.find((r) => r.type === "step" && r.tool === it) as
        | Extract<TurnRow, { type: "step" }>
        | undefined;
      if (!row || row.approval) continue;
      const dist = Math.abs(i - at);
      if (dist < bestDist) {
        bestRow = row;
        bestDist = dist;
      }
    }
    if (bestRow && ap.resolved !== "deny") bestRow.approval = ap;
    else {
      // No executed call to attach to (or it was declined) — the ask keeps its own row,
      // placed where the approval sat in the stream.
      const after = items.slice(0, at).filter((it) => it.kind !== "approval").length;
      rows.splice(after, 0, { type: "ask", approval: ap });
    }
  }
  return rows;
}

export function approvalChip(resolved: ApprovalDecision | undefined) {
  if (resolved === "deny")
    return <span className="text-xs px-1.5 rounded-full bg-dangerSoft text-danger shrink-0">✕ declined</span>;
  return (
    <span
      className="text-xs px-1.5 rounded-full bg-okSoft text-ok shrink-0"
      title={resolved ? `approved · ${resolved.replace(/_/g, " ")}` : "approved"}
    >
      ✓ approved
    </span>
  );
}

export function LineText({ line }: { line: HumanLine }) {
  return (
    <span className="min-w-0 text-md leading-prose">
      <span className="text-muted">{line.pre}</span>
      {line.obj && <span className="text-ink">{line.obj}</span>}
      {line.post && <span className="text-muted">{line.post}</span>}
    </span>
  );
}
