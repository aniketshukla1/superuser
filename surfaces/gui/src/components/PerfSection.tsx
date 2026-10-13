import { useEffect, useState } from "react";
import { addPerfRoutine, checkPerf, getPerf, setPerf, type PerfCheck, type PerfSetup } from "../api";
import { CARD } from "../ui";

// Product Performance's settings.
//
// Investigates a slowdown in the connected Prometheus — live and read-only — and says where it
// is, with the numbers and links. Here: whether Prometheus is connected, the signals the weekly
// health report watches (checkable on the spot, so a wrong query shows up now rather than in
// Monday's report), and that report's routine.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

const EXAMPLE =
  "Checkout p95 latency: histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket{handler=\"/checkout\"}[5m])))\n" +
  "Error rate: sum(rate(http_requests_total{code=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m]))";

export function PerfSection() {
  const [perf, setPerfState] = useState<PerfSetup | null>(null);
  const [signals, setSignals] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [checks, setChecks] = useState<PerfCheck[] | null>(null);

  const load = () =>
    getPerf().then((p) => {
      setPerfState(p);
      if (p) setSignals(p.signals);
    });
  useEffect(() => {
    load();
  }, []);

  const save = async () => {
    setBusy(true);
    setNotice(null);
    setChecks(null);
    const res = await setPerf({ perf_signals: signals.trim() });
    setBusy(false);
    if (res.ok === false) {
      setNotice(res.error ?? "Couldn't save that.");
      return;
    }
    await load();
    setNotice("Saved.");
  };

  const check = async () => {
    setBusy(true);
    setNotice(null);
    const res = await checkPerf();
    setBusy(false);
    if (!res.ok) {
      setNotice(res.error ?? "Couldn't check them.");
      return;
    }
    setChecks(res.results ?? []);
  };

  const addRoutine = async () => {
    setBusy(true);
    setNotice(null);
    const res = await addPerfRoutine();
    setBusy(false);
    if (!res.ok) {
      setNotice(res.error ?? "Couldn't add the routine.");
      return;
    }
    await load();
    setNotice(res.created ? "Added: a health report every Monday at 9:30." : "It's already set up.");
  };

  if (!perf) return null;
  const unsaved = signals.trim() !== perf.signals.trim();

  return (
    <section className={`${CARD} p-4 mt-6`} data-testid="perf">
      <div className="text-md font-medium">Product Performance</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        Investigates a slowdown in your metrics — where it is, by how much, since when — with the numbers, a chart
        and links, facts kept apart from guesses. It reads Prometheus live and never changes an alert or a setting.
      </p>

      <div className="text-xs text-muted mb-3" data-testid="perf-connection">
        {perf.connected
          ? `Reading ${perf.account || "Prometheus"}.`
          : "Connect Prometheus under sidebar ▸ More ▸ Connectors ▸ Prometheus — Thanos, Mimir, VictoriaMetrics and Grafana Cloud work too."}
      </div>

      <label className="text-sm font-medium text-ink" htmlFor="perf-signals">
        Signals to watch
      </label>
      <textarea
        id="perf-signals"
        className={`${INPUT} mt-1 mb-1 min-h-[88px] font-mono text-sm`}
        placeholder={EXAMPLE}
        value={signals}
        spellCheck={false}
        onChange={(e) => setSignals(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        A name, a colon, then its PromQL — one a line. The weekly health report compares each one with the week before.
      </div>

      {checks && (
        <ul className="text-xs mb-3 space-y-1" data-testid="perf-checks">
          {checks.length === 0 ? (
            <li className="text-muted">No signals to check yet.</li>
          ) : (
            checks.map((c) => (
              <li key={c.name} className={c.ok ? "text-muted" : "text-danger"}>
                {c.ok
                  ? `${c.name}: ${c.series === 0 ? "no series right now — fine if it only exists when something's wrong" : `works (${c.series} series)`}`
                  : `${c.name}: ${c.error}`}
              </li>
            ))
          )}
        </ul>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <button className={BTN} disabled={busy} onClick={save}>
          {busy ? "Working…" : "Save"}
        </button>
        {perf.connected && perf.count > 0 && !unsaved && (
          <button className={BTN_BORDERED} disabled={busy} onClick={check}>
            Check signals
          </button>
        )}
        {perf.routine ? (
          <span className="text-xs text-muted">A health report runs Mondays at 9:30.</span>
        ) : (
          <button className={BTN_BORDERED} disabled={busy} onClick={addRoutine}>
            Add a weekly health report
          </button>
        )}
        {notice ? (
          <span className="text-xs text-muted" role="status">
            {notice}
          </span>
        ) : null}
      </div>
    </section>
  );
}
