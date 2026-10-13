import { useMemo } from "react";
import { PALETTE, fmt, num, ticks, type ChartSpec } from "./ChartSpec";

export function Chart({ spec }: { spec: ChartSpec }) {
  const series = useMemo(
    () => spec.series?.length ? spec.series : spec.y ? [{ field: spec.y.field, label: spec.y.label }] : [],
    [spec],
  );
  const xField = spec.x?.field ?? Object.keys(spec.data[0] ?? {})[0] ?? "";
  const rows = spec.data;

  if (series.length === 0) return null;

  if (spec.type === "pie") {
    const field = series[0].field;
    const total = rows.reduce((n, r) => n + num(r[field]), 0) || 1;
    let angle = -Math.PI / 2;
    return (
      <figure className="chart">
        {spec.title && <figcaption className="chart-title">{spec.title}</figcaption>}
        <div className="chart-pie-wrap">
          <svg viewBox="0 0 120 120" className="chart-pie" role="img" aria-label={spec.title || "Pie chart"}>
            {rows.map((r, i) => {
              const frac = num(r[field]) / total;
              const a0 = angle;
              angle += frac * Math.PI * 2;
              const [x0, y0] = [60 + 52 * Math.cos(a0), 60 + 52 * Math.sin(a0)];
              const [x1, y1] = [60 + 52 * Math.cos(angle), 60 + 52 * Math.sin(angle)];
              return (
                <path
                  key={i}
                  d={`M60 60 L${x0} ${y0} A52 52 0 ${frac > 0.5 ? 1 : 0} 1 ${x1} ${y1} Z`}
                  fill={PALETTE[i % PALETTE.length]}
                />
              );
            })}
          </svg>
          <ul className="chart-legend">
            {rows.map((r, i) => (
              <li key={i}>
                <span className="chart-swatch" style={{ background: PALETTE[i % PALETTE.length] }} />
                <span className="chart-legend-name">{String(r[xField] ?? "")}</span>
                <span className="chart-legend-val">{fmt(num(r[field]))}</span>
              </li>
            ))}
          </ul>
        </div>
        {spec.note && <p className="chart-note">{spec.note}</p>}
        <ChartSource source={spec.source} />
      </figure>
    );
  }

  // Cartesian: bar, line, area, scatter share one frame.
  const W = 520;
  const H = 240;
  const PAD = { l: 46, r: 12, t: 10, b: 30 };
  const plotW = W - PAD.l - PAD.r;
  const plotH = H - PAD.t - PAD.b;
  const max = Math.max(1, ...rows.flatMap((r) => series.map((s) => num(r[s.field]))));
  const yTicks = ticks(max);
  const top = yTicks[yTicks.length - 1];
  const yOf = (v: number) => PAD.t + plotH - (v / top) * plotH;
  const band = plotW / rows.length;
  const xOf = (i: number) => PAD.l + band * i + band / 2;

  return (
    <figure className="chart">
      {spec.title && <figcaption className="chart-title">{spec.title}</figcaption>}
      <svg viewBox={`0 0 ${W} ${H}`} className="chart-svg" role="img" aria-label={spec.title || "Chart"}>
        {yTicks.map((t) => (
          <g key={t}>
            <line className="chart-grid" x1={PAD.l} x2={W - PAD.r} y1={yOf(t)} y2={yOf(t)} />
            <text className="chart-axis" x={PAD.l - 6} y={yOf(t) + 3} textAnchor="end">
              {fmt(t)}
            </text>
          </g>
        ))}
        {series.map((s, si) => {
          const colour = PALETTE[si % PALETTE.length];
          if (spec.type === "bar") {
            const bw = Math.max(2, (band * 0.62) / series.length);
            return rows.map((r, i) => {
              const v = num(r[s.field]);
              return (
                <rect
                  key={`${si}-${i}`}
                  x={xOf(i) - (bw * series.length) / 2 + bw * si}
                  y={yOf(v)}
                  width={bw}
                  height={Math.max(0, plotH + PAD.t - yOf(v))}
                  fill={colour}
                  rx={2}
                >
                  <title>{`${String(r[xField] ?? "")}: ${fmt(v)}`}</title>
                </rect>
              );
            });
          }
          const pts = rows.map((r, i) => `${xOf(i)},${yOf(num(r[s.field]))}`).join(" ");
          if (spec.type === "scatter") {
            return rows.map((r, i) => (
              <circle key={`${si}-${i}`} cx={xOf(i)} cy={yOf(num(r[s.field]))} r={3} fill={colour}>
                <title>{`${String(r[xField] ?? "")}: ${fmt(num(r[s.field]))}`}</title>
              </circle>
            ));
          }
          return (
            <g key={si}>
              {spec.type === "area" && (
                <polygon
                  points={`${PAD.l},${PAD.t + plotH} ${pts} ${PAD.l + plotW},${PAD.t + plotH}`}
                  fill={colour}
                  opacity={0.16}
                />
              )}
              <polyline points={pts} fill="none" stroke={colour} strokeWidth={1.8} />
            </g>
          );
        })}
        {rows.map((r, i) => (
          // Every label on a dense axis is a smear; one in N stays readable.
          i % Math.ceil(rows.length / 8) === 0 ? (
            <text key={i} className="chart-axis" x={xOf(i)} y={H - 10} textAnchor="middle">
              {String(r[xField] ?? "").slice(0, 10)}
            </text>
          ) : null
        ))}
      </svg>
      {series.length > 1 && (
        <ul className="chart-legend chart-legend-row">
          {series.map((s, i) => (
            <li key={s.field}>
              <span className="chart-swatch" style={{ background: PALETTE[i % PALETTE.length] }} />
              <span className="chart-legend-name">{s.label || s.field}</span>
            </li>
          ))}
        </ul>
      )}
      {spec.note && <p className="chart-note">{spec.note}</p>}
      <ChartSource source={spec.source} />
    </figure>
  );
}

/** Where the numbers came from, said quietly under the chart.
 *
 *  Small and grey rather than hidden in a tooltip: the whole argument for making the
 *  agent name a source is that the reader can see it without asking. A chart whose
 *  provenance you have to hover to find is a chart nobody checks. */
export function ChartSource({ source }: { source?: string }) {
  if (!source?.trim()) return null;
  return (
    <p className="chart-source" data-testid="chart-source">
      Source: {source.trim()}
    </p>
  );
}
