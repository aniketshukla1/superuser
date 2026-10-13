import { WorkspaceStats } from "../components/WorkspaceStats";
import { Transcript, ThinkingBlock } from "../components/Transcript";
import { streamMode } from "../streamGate";
import { Markdown } from "../components/Markdown";
import { TurnStatus } from "../components/TurnStatus";
import type { AppModel } from "./model/useAppModel";
import { shortPersonaName } from "../personaScope";

/** The transcript, or the empty state for a new chat. */
export function ChatScroll({ m }: { m: AppModel }) {
  const { scrollRef, handleScroll, idle, items, sessionId, sessions, selectSession, approve,
      running, retry, setLiveView, openSettings, streaming, reasoningStream, turnStartedAt,
      turnTokens, compacting, waitLabel, personas } = m;
  return (
    <div className="main-scroll" ref={scrollRef} onScroll={handleScroll}>
              {idle ? (
                /* A new chat opens on what this workspace has DONE, not on the persona
                   introducing itself. The intro ("I'm Coder. I work inside a codebase…")
                   plus three starter chips was a greeting, and a greeting earns its space
                   once; by the tenth new chat it is a wall between you and the composer.
                   Owner ask — the same text was on every persona and on the cowork intro.

                   It renders nothing at all on a first run: an empty dashboard is worse
                   than the blank screen it replaced. */
                <div className="hero">
                  <WorkspaceStats />
                </div>
              ) : (
                <>
                  <Transcript
                    items={items}
                    sessionId={sessionId}
                    onOpenThread={(sid) => {
                      const t = sessions.find((x) => x.session_id === sid);
                      if (t) selectSession(t.session_id, t.workspace, t.agent);
                    }}
                    onApprove={approve}
                    running={running}
                    onRetry={retry}
                    onWatch={setLiveView}
                    onOpenSettings={openSettings}
                    personaName={(id) => shortPersonaName(personas?.find((p) => p.id === id)?.name, id)}
                    // §33 ref #3: sub-threshold streamed text renders INSIDE the live turn
                    // group (header when collapsed, quiet line when expanded) — never as a
                    // floating paragraph.
                    streamingText={streamMode(streaming, items, running) === "quiet" ? streaming : undefined}
                  />
                  {/* Live thinking (reasoning models): a quiet collapsed block that streams the
                      trace for anyone who expands it; folds into the answer's disclosure when
                      the message finalizes. */}
                  {running && reasoningStream && !streaming && (
                    <div className="transcript">
                      <ThinkingBlock text={reasoningStream} live />
                    </div>
                  )}
                  {streaming && streamMode(streaming, items, running) === "answer" && (
                    <div className="transcript">
                      <div className="bubble-assistant">
                        <Markdown text={streaming} />
                        <span className="stream-cursor">▍</span>
                      </div>
                    </div>
                  )}
                  {/* Under everything the turn is producing, for as long as it runs. The
                      label is whatever we are actually waiting on — the tool in flight, or
                      the model — so the line is never just decoration. */}
                  {running && (
                    <div className="transcript">
                      <TurnStatus
                        startedAt={turnStartedAt}
                        tokens={turnTokens}
                        label={compacting ? "Compacting context…" : waitLabel || undefined}
                      />
                    </div>
                  )}
                </>
              )}
            </div>
  );
}
