import type { Stats } from "../../api";

/**
 * What a new chat opens on.
 *
 * It used to open on the persona introducing itself — "I'm Coder. I work inside a
 * codebase…" — plus three starter chips. That is a greeting, and a greeting is only worth
 * screen space the first time; by the tenth new chat it is a wall between you and the
 * composer. This shows what the workspace has actually done instead, which is at least
 * different every time you look.
 *
 * The token count is real as of the usage work: the backend now asks OpenAI-compatible
 * providers for it (stream_options.include_usage), reads Ollama's own counts, and stores
 * input+output on the assistant message. It is shown only once something has actually been
 * measured — sessions from before that carry no figure, and a total that silently excludes
 * most of your history is worse than no total.
 */
export type Range = "all" | "30d" | "7d";
export type Tab = "overview" | "models";

export const RANGES: { id: Range; label: string }[] = [
  { id: "all", label: "All" },
  { id: "30d", label: "30d" },
  { id: "7d", label: "7d" },
];

export const compact = (n: number) =>
  n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 10_000 ? `${(n / 1000).toFixed(1)}k` : n.toLocaleString();

export const hour12 = (h: number) => (h === 0 ? "12 AM" : h < 12 ? `${h} AM` : h === 12 ? "12 PM" : `${h - 12} PM`);

/** How many week-columns actually fit in `width`, read from the same tokens the
 *  CSS lays them out with.
 *
 *  The span used to be a constant. 20 was chosen once, then 34 because 34 filled a
 *  680px card — and a number picked against one width is wrong at every other one:
 *  narrower than that and the grid scrolled sideways, wider and it left a gap. A
 *  heatmap has no correct length, so the honest rule is "as much history as there
 *  is room for". */
export function weeksThatFit(width: number): number {
  const px = (name: string, fallback: number) => {
    const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    const n = parseFloat(v);
    return Number.isFinite(n) && n > 0 ? n : fallback;
  };
  const pitch = px("--heat-cell", 18) + px("--heat-gap", 2);
  // +gap because the last column has no gutter after it.
  return Math.max(4, Math.floor((width + px("--heat-gap", 2)) / pitch));
}

/** Sunday-first grid of the last `weeks` weeks, oldest column first. */
export function heatmap(days: { date: string; count: number }[], weeks: number) {
  const counts = new Map(days.map((d) => [d.date, d.count]));
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  // Wind back to the Sunday that starts the earliest column, so every column is a full week
  // and rows line up with weekdays.
  const start = new Date(today);
  start.setDate(start.getDate() - today.getDay() - (weeks - 1) * 7);
  const cells: { key: string; count: number; future: boolean }[] = [];
  // EXACTLY `weeks * 7`, which is exactly `weeks` columns. It used to emit
  // `weeks * 7 + today.getDay() + 1` — a whole extra partial column — so a caller
  // asking for the number of columns that fit got one more than it asked for, and
  // that one hung off the edge. Harmless while the grid scrolled; a silent crop the
  // moment it stopped. The days after today are still generated and still hidden by
  // `.is-future`, which is what keeps the grid a rectangle rather than a staircase.
  for (let i = 0; i < weeks * 7; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    cells.push({ key, count: counts.get(key) ?? 0, future: d > today });
  }
  return cells;
}

/** A number nobody has intuition for, against one they might.
 *
 * Word counts are the standard published figures; ~1.3 tokens per word is the usual rough
 * conversion for English prose. It is an analogy, not a measurement, and the "~" says so. */
export const BOOKS: [string, number][] = [
  ["The Great Gatsby", 47_000 * 1.3],
  ["Pride and Prejudice", 122_000 * 1.3],
  ["War and Peace", 587_000 * 1.3],
];

/** Whatever `/v1/stats` sent, as a Stats with every field present — or null for nothing usable.
 *
 *  This panel is the first thing a new chat renders, so a throw here is not a broken card: it
 *  is the error boundary unloading the whole main area. A payload without `sessions` did
 *  exactly that (`undefined.toLocaleString`), and the hermetic suite's mock sends one, which
 *  is how every e2e test that begins on a new chat had been failing before it started. */
export function statsOf(raw: unknown): Stats | null {
  if (!raw || typeof raw !== "object") return null;
  const r = raw as Partial<Stats>;
  const num = (v: unknown) => (typeof v === "number" && Number.isFinite(v) ? v : 0);
  const arr = <T,>(v: unknown): T[] => (Array.isArray(v) ? (v as T[]) : []);
  return {
    sessions: num(r.sessions),
    messages: num(r.messages),
    tokens: num(r.tokens),
    priced_messages: num(r.priced_messages),
    active_days: num(r.active_days),
    current_streak: num(r.current_streak),
    longest_streak: num(r.longest_streak),
    peak_hour: typeof r.peak_hour === "number" ? r.peak_hour : null,
    favorite_model: typeof r.favorite_model === "string" ? r.favorite_model : null,
    favorite_agent: typeof r.favorite_agent === "string" ? r.favorite_agent : null,
    models: arr(r.models),
    agents: arr(r.agents),
    days: arr(r.days),
  };
}

/** Local and routing prefixes a stored model id carries: "custom:qwen2.5:7b-instruct". */
export const MODEL_PREFIXES = ["custom", "ollama", "openai", "anthropic", "gemini", "opencode", "codex", "claude-code", "groq", "together", "fireworks", "deepseek", "mistral", "xai", "openrouter"];

/** "assistant" → "Assistant": the stats name a persona by id. */
export const agentName = (id: string) => (id === "cowork" ? "Assistant" : id.charAt(0).toUpperCase() + id.slice(1).replace(/[-_]/g, " "));

export function comparison(tokens: number): string {
  // The biggest book you have beaten, so the multiplier stays a number you can hold. Below
  // the smallest, say the fraction instead of claiming "0x".
  const [name, size] = [...BOOKS].reverse().find(([, n]) => tokens >= n) ?? BOOKS[0];
  const times = tokens / size;
  return times >= 1
    ? `You've used ~${Math.round(times).toLocaleString()}\u00d7 more tokens than ${name}.`
    : `You've used about ${Math.round(times * 100)}% of ${name}, in tokens.`;
}
