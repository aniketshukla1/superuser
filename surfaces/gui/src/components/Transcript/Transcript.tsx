import { Fragment, type ReactNode } from "react";
import { Markdown } from "../Markdown";
import { ConnectorMessageCard } from "../ConnectorMessageCard";
import { Chart } from "../Chart";
import { DataTable } from "../DataTable";
import { FactCard } from "../FactCard";
import { ComputerTile } from "../ComputerCards";
import { GeneratedView } from "../GeneratedView";
import { ErrorCallout } from "../ErrorCallout";
import { brainFailure, noticeFailure } from "../../failure";
import { ReplyMeta, ThinkingBlock } from "./ReplyMeta";
import { TurnGroup } from "./TurnGroup";
import { retryAnchor, type Props } from "./visualsOf";
import { buildBlocks } from "./blocks";
import { ThreadLinks } from "./ThreadLinks";
import { UserBubble } from "./UserBubble";
import { useFeedback } from "./useFeedback";

export function Transcript({ items, running, streamingText, onRetry, onOpenThread, sessionId, onWatch, onOpenSettings, personaName }: Props) {
  const { reasons, given, setGiven } = useFeedback(sessionId);
  const { blocks, lastTurnIndex } = buildBlocks(items, running);

  // Where one exchange ends and the next begins. The transcript used a single gap
  // between every block, so a question sat as far from its own answer as from an
  // unrelated turn — the page read as a list of items rather than a conversation.
  // The base gap is now the WITHIN-exchange rhythm, and these get the space that
  // separates one exchange from the next.
  const startsExchange = (b: (typeof blocks)[number], i: number) => {
    if (i === 0) return false; // nothing above it to be separated from
    if (!("item" in b)) return false;
    return b.item.kind === "user" || b.item.kind === "connector";
  };
  // Counts user turns as they render, so TranscriptNav can address them. A running counter
  // rather than the block index: the navigator lists user turns only, so its Nth entry has
  // to land on the Nth question, not the Nth block.
  let userTurn = 0;
  return (
    <div className="transcript">
      {blocks.map((block, bi) => {
        // No day separators. A rule labelled "Today" drawn across a conversation you opened
        // today divides nothing, and the dated ones further up say what the sidebar already
        // says. `sep` stays as a null so the wrapper below keeps its shape.
        const sep = null;
        const opensExchange = startsExchange(block, bi);
        if ("turn" in block)
          return (
            <Fragment key={`b-${bi}`}>
              {sep}
            <TurnGroup
              items={block.turn}
              live={block.live}
              streamingText={block.live && bi === lastTurnIndex ? streamingText : undefined}
              key={bi}
            />
            </Fragment>
          );
        if ("chart" in block)
          return (
            <div className="chart-block" key={`b-${bi}`} data-testid="transcript-chart">
              <Chart spec={block.chart} />
            </div>
          );
        if ("table" in block)
          return (
            <div className="chart-block" key={`b-${bi}`} data-testid="transcript-table">
              <DataTable spec={block.table} />
            </div>
          );
        if ("card" in block)
          return (
            <div className="chart-block" key={`b-${bi}`} data-testid="transcript-card">
              <FactCard spec={block.card} />
            </div>
          );
        if ("view" in block)
          return (
            <div className="chart-block" key={`b-${bi}`} data-testid="transcript-view">
              <GeneratedView title={block.view.title} html={block.view.html} source={block.view.source} />
            </div>
          );
        if ("shot" in block)
          return (
            // Not in `.chart-block`: the tile is its own card, and a card inside a card
            // draws the frame twice.
            <div key={`b-${bi}`} data-testid="transcript-computer">
              <ComputerTile
                persona={block.shot.persona}
                frame={block.shot.frame}
                url={block.shot.url}
                title={block.shot.title}
                onWatch={onWatch}
              />
            </div>
          );
        // Non-turn blocks each render their own element; wrap so the separator can
        // ride along without touching every case.
        const { item } = block;
        const wrap = (el: ReactNode) => (
          <Fragment key={`b-${bi}`}>
            {sep}
            {el}
          </Fragment>
        );
        switch (item.kind) {
          case "connector":
            return <ConnectorMessageCard source={item.source} key={bi} />;
          case "user":
            return wrap(<UserBubble item={item} turn={userTurn++} opensExchange={opensExchange} />);
          case "assistant": {
            // Thinking-only item (stopped mid-reasoning): just the disclosure, no bubble.
            if (!item.text && item.reasoning)
              return (
                <div key={bi}>
                  <ThinkingBlock text={item.reasoning} />
                </div>
              );
            // A memory question that could not reach a model answers with its failure. That is
            // a failure, not an answer: no thumbs, no copy row, and a way to the fix.
            const failure = brainFailure(item.text);
            if (failure)
              return (
                <ErrorCallout
                  key={bi}
                  failure={failure}
                  onRetry={!running && bi === blocks.length - 1 ? onRetry : undefined}
                  onFix={onOpenSettings}
                />
              );
            return (
              <div className="group bubble-assistant" key={bi}>
                {/* Another assistant's reply: the person addressed it with `@id`. */}
                {item.persona && (
                  <div className="who" data-testid="reply-from">
                    {personaName?.(item.persona) ?? `@${item.persona}`}
                  </div>
                )}
                {item.reasoning && <ThinkingBlock text={item.reasoning} />}
                <Markdown text={item.text} />
                {/* Retry is pinned on the LAST reply only, and not while a turn is running —
                    the status line owns the bottom of the transcript until the turn ends. */}
                <ReplyMeta
                  text={item.text}
                  ts={item.ts}
                  onRetry={!running && bi === blocks.length - 1 ? onRetry : undefined}
                  // Not while the turn is still running: judging an answer that is still
                  // arriving is judging something the user has not read.
                  rateable={!running}
                  sessionId={sessionId}
                  reasons={reasons}
                  given={item.ts ? given[item.ts] : undefined}
                  onGiven={(v) => item.ts && setGiven((g) => ({ ...g, [item.ts as number]: v }))}
                />
              </div>
            );
          }
          case "dirreq":
            if (!item.resolved) return null;
            return (
              <div className="approval-inline" key={bi}>
                <span className={"status " + (item.resolved === "granted" ? "ok" : "denied")}>
                  {item.resolved === "granted" ? "✓" : "✕"}
                </span>
                <span>{item.resolved === "granted" ? "Granted folder access" : "Declined folder access"}</span>
                {item.path && <span className="dim">{item.path}</span>}
              </div>
            );
          case "planreq":
            if (!item.resolved) return null; // pending plan renders in the composer head
            return (
              <div className="bubble-assistant" key={bi}>
                <div className="who">proposed plan</div>
                <Markdown text={item.plan} />
                <div className="approval-inline">
                  <span className={"status " + (item.resolved === "approved" ? "ok" : "denied")}>
                    {item.resolved === "approved" ? "✓" : "✕"}
                  </span>
                  <span>{item.resolved === "approved" ? "Plan approved" : "Sent back with feedback"}</span>
                </div>
              </div>
            );
          case "threads":
            // The sub-threads this turn fanned out into. Reachable from the
            // conversation that created them, not only from the sidebar.
            return <ThreadLinks key={bi} threads={item.threads} onOpenThread={onOpenThread} />;
          case "notice":
            // A turn that died is a failure with a next step. Interruptions, model switches
            // and compaction markers are history, and stay the quiet centred line.
            if (item.retriable)
              return (
                <ErrorCallout
                  key={bi}
                  failure={noticeFailure(item.text)}
                  onRetry={!running && onRetry && block.i === retryAnchor(items) ? onRetry : undefined}
                  onFix={onOpenSettings}
                />
              );
            return (
              <div className={"notice " + (item.tone === "warn" ? "warn" : "")} key={bi}>
                {item.text}
              </div>
            );
          default:
            return null;
        }
      })}
    </div>
  );
}
