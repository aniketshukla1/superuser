import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { getStats, type Stats } from "../../api";
import { stripProvider } from "../../modelName";
import { MODEL_PREFIXES, RANGES, agentName, compact, comparison, heatmap, hour12, statsOf, weeksThatFit, type Range, type Tab } from "./statsOf";

export function WorkspaceStats() {
  const [range, setRange] = useState<Range>("all");
  const [tab, setTab] = useState<Tab>("overview");
  const [stats, setStats] = useState<Stats | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let alive = true;
    getStats(range).then((s) => {
      if (!alive) return;
      setStats(statsOf(s));
      setLoaded(true);
    });
    return () => {
      alive = false;
    };
  }, [range]);

  // The grid's own width decides how much history it shows. Starts at 0 so nothing
  // renders until it has been measured — a first paint at a guessed width would
  // draw one span and immediately redraw at another.
  const heatRef = useRef<HTMLDivElement>(null);
  const [heatWidth, setHeatWidth] = useState(0);
  useLayoutEffect(() => {
    const el = heatRef.current;
    if (!el) return;
    const measure = () => setHeatWidth(el.clientWidth);
    measure();
    // Guarded: jsdom has no ResizeObserver, and a component that throws in a test
    // environment will throw wherever else lacks it. The first measurement still
    // happens; only following a resize is lost.
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [stats]);
  const cells = useMemo(
    () => (heatWidth ? heatmap(stats?.days ?? [], weeksThatFit(heatWidth)) : []),
    [stats, heatWidth],
  );
  // Whether any figure has actually been measured. Everything written before the usage
  // work carries none, and a breakdown of zeros is not a breakdown.
  const measured = (stats?.priced_messages ?? 0) > 0;
  const busiest = useMemo(() => Math.max(1, ...cells.map((c) => c.count)), [cells]);

  // Nothing yet, and nothing to say about it. An empty dashboard on a first run is worse
  // than the blank screen it replaced.
  if (loaded && (!stats || stats.sessions === 0)) return null;
  if (!stats) return null;

  const days = (n: number) => `${n} day${n === 1 ? "" : "s"}`;
  const cards: { label: string; value: string; title?: string }[] = [
    { label: "Sessions", value: stats.sessions.toLocaleString() },
    { label: "Messages", value: compact(stats.messages) },
    // Hidden until something has been measured. Before the usage work nothing recorded a
    // figure, so on existing history this reads 0 — and a zero here looks like a claim
    // rather than an absence.
    ...(stats.priced_messages > 0
      ? [{ label: "Tokens", value: compact(stats.tokens) }]
      : []),
    { label: "Active days", value: stats.active_days.toLocaleString() },
    // One streak card, and only when there is a streak to report. "Current streak 0d" on the
    // screen a new chat opens to reads as a scold, which no work tool should be doing.
    ...(stats.current_streak > 1
      ? [{ label: "Streak", value: days(stats.current_streak) }]
      : stats.longest_streak > 1
        ? [{ label: "Longest streak", value: days(stats.longest_streak) }]
        : []),
    { label: "Busiest hour", value: stats.peak_hour === null ? "—" : hour12(stats.peak_hour) },
    // The stored id is "custom:qwen2.5:7b-instruct"; the card had room for "custom:…" and
    // nothing of the model. The routing prefix goes; the full id stays in the tooltip.
    {
      label: "Most used model",
      value: stats.favorite_model ? stripProvider(stats.favorite_model, MODEL_PREFIXES) : "—",
      title: stats.favorite_model ?? undefined,
    },
    { label: "Most used assistant", value: stats.favorite_agent ? agentName(stats.favorite_agent) : "—" },
  ];

  return (
    <div className="wstats" data-testid="workspace-stats">
      <div className="wstats-head">
        <div className="wstats-tabs" role="tablist" aria-label="Statistics view">
          {(["overview", "models"] as Tab[]).map((t) => (
            <button
              key={t}
              role="tab"
              aria-selected={tab === t}
              className={"wstats-tab" + (tab === t ? " is-on" : "")}
              onClick={() => setTab(t)}
            >
              {t === "overview" ? "Overview" : "Models"}
            </button>
          ))}
        </div>
        <div className="wstats-ranges" role="group" aria-label="Time range">
          {RANGES.map((r) => (
            <button
              key={r.id}
              className={"wstats-range" + (range === r.id ? " is-on" : "")}
              aria-pressed={range === r.id}
              onClick={() => setRange(r.id)}
            >
              {r.label}
            </button>
          ))}
        </div>
      </div>

      {tab === "overview" ? (
        <>
          <div className="wstats-grid">
            {cards.map((c) => (
              <div className="wstats-card" key={c.label}>
                <div className="wstats-card-label">{c.label}</div>
                <div className="wstats-card-value" title={c.title ?? c.value}>
                  {c.value}
                </div>
              </div>
            ))}
          </div>
          {/* The grid said nothing about itself: no unit, no scale. A caption and a legend are
              the two things that make seventeen columns of squares a chart. */}
          <div className="wstats-heat-head" aria-hidden>
            <span>Sessions per day{cells.length ? `, last ${Math.round(cells.length / 7)} weeks` : ""}</span>
            <span className="wstats-legend">
              Fewer
              {[0, 1, 2, 3].map((l) => (
                <span key={l} className="wstats-cell" data-level={l} />
              ))}
              More
            </span>
          </div>
          <div className="wstats-heat" ref={heatRef} role="img" aria-label={`${stats.active_days} active days`}>
            {cells.map((c) => (
              <span
                key={c.key}
                className={"wstats-cell" + (c.future ? " is-future" : "")}
                // Four steps, not a continuous ramp: a gradient across the whole span
                // makes "some" and "a lot" indistinguishable at this size.
                data-level={c.count === 0 ? 0 : c.count >= busiest ? 3 : c.count > busiest / 2 ? 2 : 1}
                title={`${c.key} — ${c.count} session${c.count === 1 ? "" : "s"}`}
              />
            ))}
          </div>
          {measured && <div className="wstats-note">{comparison(stats.tokens)}</div>}
        </>
      ) : (
        <div className="wstats-models">
          {stats.models.length === 0 ? (
            <div className="wstats-empty">No model recorded on these sessions yet.</div>
          ) : (
            stats.models.map((m) => {
              // Share of tokens when any are known, of sessions otherwise. A bar labelled
              // as a percentage of one thing while measuring another is just wrong.
              const total = stats.models.reduce((n, x) => n + (measured ? x.tokens : x.sessions), 0) || 1;
              const share = ((measured ? m.tokens : m.sessions) / total) * 100;
              return (
                <div className="wstats-model" key={m.model}>
                  <span className="wstats-model-name" title={m.model}>
                    {m.model}
                  </span>
                  <span className="wstats-model-bar">
                    <span style={{ width: `${Math.max(2, Math.round(share))}%` }} />
                  </span>
                  <span className="wstats-model-detail">
                    {measured
                      ? `${compact(m.input)} in · ${compact(m.output)} out`
                      : `${m.sessions} session${m.sessions === 1 ? "" : "s"}`}
                  </span>
                  <span className="wstats-model-n">{share.toFixed(1)}%</span>
                </div>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}
