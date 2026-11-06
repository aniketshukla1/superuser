import { Markdown } from "../components/Markdown";
import { ThinkingBlock, Transcript } from "../components/Transcript";
import { TurnStatus } from "../components/TurnStatus";
import { WorkspaceStats } from "../components/WorkspaceStats";
import { shortPersonaName } from "../personaScope";
import { streamMode } from "../streamGate";
import type { AppModel } from "./model/useAppModel";

/** The live parts under the transcript while a turn runs: the thinking trace, an answer that
 *  is streaming in, and the status line. */
function LiveTurn({ m, mode }: { m: AppModel; mode: string }) {
  const { compacting, reasoningStream, running, streaming, turnStartedAt, turnTokens, waitLabel } = m;
  return (
    <>
      {/* Thinking, for reasoning models: a quiet, collapsed block that streams the trace to anyone
          who opens it, and folds into the answer's disclosure once the message is final. */}
      {running && reasoningStream && !streaming && (
        <div className="transcript">
          <ThinkingBlock text={reasoningStream} live />
        </div>
      )}
      {streaming && mode === "answer" && (
        <div className="transcript">
          <div className="bubble-assistant">
            <Markdown text={streaming} />
            <span className="stream-cursor">▍</span>
          </div>
        </div>
      )}
      {/* Below everything the turn makes, for as long as it runs. It names what the turn is really
          waiting on, the tool in flight or the model, so it is never just decoration. */}
      {running && (
        <div className="transcript">
          <TurnStatus startedAt={turnStartedAt} tokens={turnTokens} label={compacting ? "Compacting context…" : waitLabel || undefined} />
        </div>
      )}
    </>
  );
}

/** The transcript, or the empty state of a new chat. */
export function ChatScroll({ m }: { m: AppModel }) {
  const { items, running, streaming } = m;
  const mode = streamMode(streaming, items, running);
  const openThread = (id: string) => {
    const thread = m.sessions.find((s) => s.session_id === id);
    if (thread) m.selectSession(thread.session_id, thread.workspace, thread.agent);
  };
  const nameOf = (id: string) => shortPersonaName(m.personas?.find((p) => p.id === id)?.name, id);
  return (
    <div className="main-scroll" ref={m.scrollRef} onScroll={m.handleScroll}>
      {m.idle ? (
        // A new chat opens on what this workspace has DONE, not on the assistant introducing
        // itself: an introduction and three starter chips earn their space once, and by the
        // tenth new chat they're a wall between you and the composer (owner's call). On a first
        // run it shows nothing at all, since an empty dashboard is worse than a blank page.
        <div className="hero">
          <WorkspaceStats />
        </div>
      ) : (
        <>
          <Transcript
            items={items}
            sessionId={m.sessionId}
            onOpenThread={openThread}
            onApprove={m.approve}
            running={running}
            onRetry={m.retry}
            onWatch={m.setLiveView}
            onOpenSettings={m.openSettings}
            personaName={nameOf}
            // §33: streamed text under the threshold lives INSIDE the running turn's group (in its
            // header when collapsed, as a quiet line when open), never as a loose paragraph.
            streamingText={mode === "quiet" ? streaming : undefined}
          />
          <LiveTurn m={m} mode={mode} />
        </>
      )}
    </div>
  );
}
