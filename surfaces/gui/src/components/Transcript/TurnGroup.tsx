import { useState } from "react";
import { shortArgs } from "../ApprovalCard";
import { humanizeAsk, humanizeTool } from "../../humanize";
import { Markdown } from "../Markdown";
import { LineText, approvalChip, buildRows, type ApprovalItem, type AssistantItem, type ToolItem, type TurnItem } from "./ReplyMeta";

export function StepRow({ tool, approval }: { tool: ToolItem; approval?: ApprovalItem }) {
  const [raw, setRaw] = useState(false);
  const running = tool.status === "…";
  // A boundary that held is not a tool that broke. The two are drawn apart because
  // red reads as "something went wrong", and a refusal is the thing the
  // user asked for working exactly as intended.
  const refused = tool.status === "refused";
  // Cut off is not failed: the run ended, or was stopped, before this call reported back.
  // Red would say something went wrong when someone simply changed their mind — the Activity
  // tab already draws that line, and this row used to say the opposite of it.
  const stopped = tool.status === "stopped";
  const failed = tool.status !== "ok" && !running && !refused && !stopped;
  return (
    <div>
      <div className="group flex items-baseline gap-2 px-2 py-0.5 rounded-lg hover:bg-paper" data-testid="turn-step">
        <span
          className={
            "w-3.5 text-center text-xs shrink-0 " +
            (failed ? "text-danger" : refused ? "text-warnInk" : running ? "text-accent" : stopped ? "text-faint" : "text-ok")
          }
        >
          {running ? <span className="spinner" data-testid="step-running" /> : "●"}
        </span>
        <LineText
          line={
            // A refused load must not read as a success — "Used skill:" is the trust line
            // (SKILLS-SPEC §4.1 #4), so a blocked attempt gets honest wording instead.
            tool.name === "load_skill" && tool.preview?.includes('"error"')
              ? { pre: "Tried skill: ", obj: String(tool.args?.name ?? ""), post: " — not available" }
              : humanizeTool(tool.name, tool.args)
          }
        />
        {approval && approvalChip(approval.resolved)}
        {!!tool.standingRule && (
          <span
            className="text-xs px-1.5 rounded-full bg-tealSoft text-tealInk shrink-0"
            data-testid="tool-standing-rule"
            title={`Auto-allowed by this automation's standing approval: ${tool.standingRule}. Revoke on its Automations page.`}
          >
            auto-allowed
          </span>
        )}
        {!!tool.hidden && (
          <span
            className="text-xs text-warnInk shrink-0"
            data-testid="tool-hidden-count"
            title="Removed by your privacy filters before the agent saw the results — agents get no trace of these."
          >
            {tool.hidden} hidden
          </span>
        )}
        {failed && <span className="text-xs text-danger shrink-0">{tool.status}</span>}
        {refused && (
          <span
            className="text-xs px-1.5 rounded-full bg-warnSoft text-warnInk shrink-0"
            data-testid="step-refused"
            title="Stopped by one of your boundaries before it happened. Settings ▸ Boundaries."
          >
            refused by a boundary
          </span>
        )}
        {stopped && <span className="text-xs text-faint shrink-0" data-testid="step-stopped">stopped</span>}
        {!running && (
          // Shown by focus as well as hover. A control only a mouse can make visible is still
          // in the tab order, and landing on something you cannot see is worse than it being
          // absent.
          <button
            className="ml-auto shrink-0 text-xs text-faint opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 cursor-pointer"
            onClick={() => setRaw((v) => !v)}
            aria-expanded={raw}
          >
            Details
          </button>
        )}
      </div>
      {raw && (
        <pre className="ml-8 mr-2 my-1 px-2.5 py-1.5 rounded-lg border border-line bg-paper font-mono text-xs leading-prose text-muted whitespace-pre-wrap break-words max-h-56 overflow-auto">
          {`${tool.name}  ${shortArgs(tool.args)}`}
          {tool.preview ? `\n→ ${tool.preview.length > 1500 ? tool.preview.slice(0, 1500) + "\n…" : tool.preview}` : ""}
        </pre>
      )}
    </div>
  );
}

export function TurnGroup({
  items,
  live,
  streamingText,
}: {
  items: TurnItem[];
  live?: boolean;
  // Sub-threshold streamed text belongs to THIS group (§33 ref #3): collapsed → it rides
  // the header as the live line; expanded → the small quiet line under the steps.
  streamingText?: string;
}) {
  // Turns start COLLAPSED, running or not (owner call 2026-07-14) — the header's live
  // line is the pulse; expanding is opt-in.
  const rows = buildRows(items);
  const tools = items.filter((it): it is ToolItem => it.kind === "tool");
  const running = live || tools.some((t) => t.status === "…");
  const [userToggle, setUserToggle] = useState<boolean | null>(null);
  const open = userToggle ?? false;
  const lastNarr = [...items].reverse().find((it): it is AssistantItem => it.kind === "assistant");
  const liveLine = streamingText || lastNarr?.text || "";

  const nSteps = rows.filter((r) => r.type !== "narr").length;
  const declined = items.filter((it) => it.kind === "approval" && it.resolved === "deny").length;
  const hiddenTotal = tools.reduce((n, t) => n + (t.hidden || 0), 0);
  const stepsLabel = `${nSteps} step${nSteps === 1 ? "" : "s"}`;

  return (
    <details className="stepgroup" open={open}>
      <summary
        className="stepgroup-head flex items-center gap-2 py-0.5 cursor-pointer select-none text-sm text-faint hover:text-muted"
        onClick={(e) => {
          e.preventDefault(); // drive open/closed from state, not the native toggle
          setUserToggle(!open);
        }}
      >
        <span className={"chev inline-block transition-transform" + (open ? " rotate-90" : "")}>›</span>
        <span>
          <span>{running ? `Running ${stepsLabel}…` : stepsLabel}</span>
          {declined > 0 && (
            <>
              {" · "}
              <span className="text-danger" data-testid="stepgroup-declined">
                {declined} declined
              </span>
            </>
          )}
          {hiddenTotal > 0 && (
            <>
              {" · "}
              <span className="text-warnInk" data-testid="stepgroup-hidden">
                {hiddenTotal} hidden by your filters
              </span>
            </>
          )}
        </span>
        {running && !open && liveLine && (
          <span className="min-w-0 flex-1 truncate" data-testid="turn-live-line">
            · {liveLine}
          </span>
        )}
      </summary>
      {open && (
        <div className="ml-1.5 mt-1 pl-2 border-l-2 border-line flex flex-col gap-0.5">
          {rows.map((row, i) =>
            row.type === "narr" ? (
              <div className="turn-narr px-2 py-1 text-md text-muted max-w-[60ch]" key={i} data-testid="turn-narration">
                <Markdown text={row.text} />
              </div>
            ) : row.type === "ask" ? (
              <div className="flex items-baseline gap-2 px-2 py-0.5" key={i} data-testid="turn-ask">
                <span className={"w-3.5 text-center text-xs shrink-0 " + (row.approval.resolved === "deny" ? "text-danger" : "text-ok")}>●</span>
                <LineText line={humanizeAsk(row.approval.name, row.approval.args)} />
                {approvalChip(row.approval.resolved)}
              </div>
            ) : (
              <StepRow tool={row.tool} approval={row.approval} key={i} />
            ),
          )}
          {streamingText && (
            <div
              className="turn-narr px-2 py-1 text-md text-muted max-w-[60ch]"
              data-testid="turn-live-stream"
            >
              <Markdown text={streamingText} />
              <span className="stream-cursor">▍</span>
            </div>
          )}
        </div>
      )}
    </details>
  );
}
