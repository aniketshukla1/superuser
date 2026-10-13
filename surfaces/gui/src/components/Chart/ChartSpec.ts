/**
 * A chart, from a JSON spec the agent wrote.
 *
 * WHY NOT VEGA-LITE (or A2UI). Vega-Lite is the obvious choice — models know its grammar
 * cold. It costs ~1.5MB of bundle, and its rendering is its own: fonts, axis colours, grid
 * weights, all decided by a theme that is not this app's. Every token in this interface was
 * argued over; a chart that ignores all of them reads as pasted in from another program.
 *
 * A2UI is a general agent-to-UI protocol. Charts through a general UI tree means the agent
 * hand-assembles axes and ticks, which is a worse job for it than naming a chart type.
 *
 * So: a small grammar, rendered here, in the app's own tokens. Narrower than Vega-Lite by
 * design — five chart types cover what an agent is asked for — and the schema lives in the
 * tool description so the model writes it correctly the first time.
 *
 * Plain SVG and no `dangerouslySetInnerHTML`: a chart spec is model output derived from
 * whatever the agent read, which may be a web page. It is data, and it is rendered as data.
 */
export type ChartSpec = {
  type: "bar" | "line" | "area" | "scatter" | "pie";
  title?: string;
  /** The insight, in the agent's words. A chart nobody explains is a picture. */
  note?: string;
  x?: { field: string; label?: string };
  y?: { field: string; label?: string };
  /** Named series for multi-line/multi-bar. Omit for one, taken from `y`. */
  series?: { field: string; label?: string }[];
  /** Where the numbers came from. Required of the agent and checked against what its run
   *  actually read (core::gates::grounding), then written into the file, so a chart can
   *  still answer "where did this come from?" long after the conversation is gone. */
  source?: string;
  data: Record<string, string | number | null>[];
};

/** Validate an already-parsed value. Returns null for anything that is not a chart we can
 *  draw — the caller falls back to showing the raw JSON, which is more use than an error. */
export function asChartSpec(v: unknown): ChartSpec | null {
  if (!v || typeof v !== "object") return null;
  const c = v as ChartSpec;
  if (!["bar", "line", "area", "scatter", "pie"].includes(c.type)) return null;
  if (!Array.isArray(c.data) || c.data.length === 0) return null;
  // A chart with no y and no series has nothing to plot; Chart() returns null for it,
  // which would leave an empty block in the transcript instead of the raw JSON.
  if (!c.series?.length && !c.y) return null;
  return c;
}

/** As above, from JSON text (the panel reads `.chart.json` files). */
export function parseChart(text: string): ChartSpec | null {
  try {
    return asChartSpec(JSON.parse(text));
  } catch {
    return null;
  }
}

export const PALETTE = ["var(--accent)", "#6FCF97", "#F2C94C", "#EB5757", "#56CCF2", "#BB6BD9"];
export const num = (v: unknown): number => {
  const n = typeof v === "number" ? v : Number(String(v ?? "").replace(/[, ]/g, ""));
  return Number.isFinite(n) ? n : 0;
};

/** A tick scale that lands on round numbers rather than on whatever the data maximum is.
 *
 *  The last tick is the first round number AT OR ABOVE the maximum, because it is also the
 *  top of the plot. The scale used to stop at the last tick below it (600 for a maximum of
 *  694), and every value above that was drawn off the top of the chart. */
export function ticks(max: number, count = 4): number[] {
  if (max <= 0) return [0];
  const raw = max / count;
  const mag = Math.pow(10, Math.floor(Math.log10(raw)));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? mag * 10;
  const out: number[] = [];
  for (let v = 0; ; v += step) {
    out.push(v);
    if (v >= max - step * 0.001) break;
  }
  return out;
}

export const fmt = (n: number): string =>
  Math.abs(n) >= 1_000_000
    ? `${(n / 1_000_000).toFixed(1)}M`
    : Math.abs(n) >= 1000
      ? `${(n / 1000).toFixed(n % 1000 === 0 ? 0 : 1)}k`
      : String(Math.round(n * 100) / 100);
