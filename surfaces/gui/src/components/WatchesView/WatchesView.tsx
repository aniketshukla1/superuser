import { useEffect, useState } from "react";
import { createWatch, getWatches, tickWatches, type WatchesState } from "../../api";
import { Icon, ICON } from "../Icon";
import { PanelHead } from "../IntegrationsView";
import { CARD } from "../../ui";
import { exactTime } from "../../when";
import { DECISION, INPUT, relative, type Frame } from "./AutomationsSurface";
import { WatchRow } from "./WatchRow";

export function WatchesView({ frame = (c) => c }: { frame?: Frame }) {
  const [state, setState] = useState<WatchesState | null>(null);
  const [question, setQuestion] = useState("");
  const [sources, setSources] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const refresh = () => getWatches().then(setState).catch(() => setState(null));
  useEffect(() => {
    refresh();
    // Slow poll: this page changes on the loop's interval, not on interaction.
    const h = setInterval(refresh, 15000);
    return () => clearInterval(h);
  }, []);

  const add = async () => {
    const q = question.trim();
    if (!q) return;
    setBusy(true);
    await createWatch(
      q,
      sources
        .split(",")
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean),
    );
    setBusy(false);
    setQuestion("");
    setSources("");
    refresh();
  };

  const checkNow = async () => {
    setBusy(true);
    setNotice(null);
    const r = await tickWatches();
    setBusy(false);
    setNotice(
      !r
        ? "Couldn't run a check."
        : r.evaluated === 0
          ? "Nothing has changed since the last look, so there was nothing to evaluate."
          : `Evaluated ${r.evaluated}: ${r.acted} answered, ${r.escalated} sent to your Inbox${r.failed ? `, ${r.failed} failed` : ""}.`,
    );
    refresh();
  };

  const watches = state?.watches ?? [];
  const activity = state?.activity ?? [];

  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">
          {frame(
            <>
              <div className="flex items-start gap-3">
                <div className="flex-1 min-w-0">
                  <PanelHead
                    title="Watches"
                    sub="Standing questions, checked against what changed. A watch costs nothing on a quiet day — it only asks the local model when there's something new to look at."
                  />
                </div>
                <button className="btn shrink-0" disabled={busy || watches.length === 0} onClick={checkNow}>
                  <Icon name="refresh" size={ICON.md} />
                  {busy ? "Checking…" : "Check now"}
                </button>
              </div>

              {state && !state.enabled && (
                <div className="text-sm text-faint flex items-center gap-1.5 mb-4">
                  <Icon name="clock" size={ICON.sm} className="shrink-0" />
                  <span>
                    The background loop is switched off, so watches are never evaluated on their own.
                    “Check now” still works.
                  </span>
                </div>
              )}
              {notice && (
                <div className="text-sm text-muted mb-4" role="status">
                  {notice}
                </div>
              )}

              {/* Add — one line, because a watch is one sentence. */}
              <div className={CARD + " p-3 mb-5"}>
                {/* Wraps rather than squeezing: at a narrow width the fixed-width
                    sources field used to crush the question down to a few characters,
                    which is the field that actually matters. */}
                <div className="flex items-center gap-2 flex-wrap">
                  <input
                    className={INPUT + " basis-full sm:basis-0 min-w-[240px]"}
                    data-testid="watch-question"
                    aria-label="What to watch for"
                    placeholder="What should it watch for?"
                    value={question}
                    onChange={(e) => setQuestion(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && add()}
                  />
                  <input
                    className={INPUT + " min-w-[150px]"}
                    data-testid="watch-sources"
                    aria-label="Sources to watch (optional)"
                    placeholder="gmail, slack (optional)"
                    value={sources}
                    onChange={(e) => setSources(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && add()}
                  />
                  <button
                    className="btn-primary shrink-0"
                    data-testid="watch-add"
                    disabled={busy || !question.trim()}
                    onClick={add}
                  >
                    Add watch
                  </button>
                </div>
                <p className="text-sm text-faint mt-2">
                  For example, “anything urgent from a customer”. Leave the sources empty to watch
                  everything. Answers appear below; anything needing a real decision goes to your Inbox
                  rather than happening on its own.
                </p>
              </div>

              {watches.length === 0 ? (
                <div className="text-md text-muted mb-6">
                  No watches yet. They're for the questions you'd otherwise re-ask every morning —
                  “anything urgent in my inbox”, “did any of my repos break”.
                </div>
              ) : (
                <div className="space-y-2 mb-7">
                  {watches.map((w) => (
                    <WatchRow key={w.id} w={w} onChanged={refresh} />
                  ))}
                </div>
              )}

              <h3 className="text-md font-semibold mb-1">Activity</h3>
              <p className="text-sm text-muted mb-3">
                Every evaluation, including the ones that decided to do nothing.
              </p>
              {activity.length === 0 ? (
                <div className="text-md text-muted">
                  Nothing yet. A line appears here each time something changes and a watch looks at it.
                </div>
              ) : (
                <div className="space-y-2">
                  {activity.map((a) => (
                    <div key={a.id} className={CARD + " p-3"} data-testid="watch-activity">
                      <div className="flex items-baseline gap-2">
                        <span className={"text-sm font-medium " + DECISION[a.decision].cls}>
                          {DECISION[a.decision].label}
                        </span>
                        <span className="text-sm text-faint truncate">{a.question}</span>
                        <span className="text-xs text-faint ml-auto shrink-0" title={exactTime(a.at)}>
                          {relative(a.at)}
                        </span>
                      </div>
                      {a.evidence && <div className="text-sm text-faint mt-1">{a.evidence}</div>}
                      {a.reason && <div className="text-sm text-muted mt-1">{a.reason}</div>}
                      {a.note && (
                        <pre className="text-sm text-ink mt-2 whitespace-pre-wrap font-sans">{a.note}</pre>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </>,
          )}
        </div>
      </div>
    </main>
  );
}
