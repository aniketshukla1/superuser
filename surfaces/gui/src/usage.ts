// Token usage per session (OPE-42), as pure functions so the folding can be tested without
// the app. Each assistant message, and the assistant_message event, may carry a `usage`
// record: {model, input, output, cache_read, cache_write}. Servers from before it, and
// backends that don't count, send none, and then nothing here changes anything.

import type { SessionUsage, TurnUsage } from "./types";
import type { ConversationMessage } from "./api";

export const emptyUsage = (): SessionUsage => ({ byModel: {}, context: 0 });

/** A count off the wire: a positive finite number, otherwise 0. */
function count(value: unknown): number {
  const n = Number(value);
  return n > 0 && Number.isFinite(n) ? n : 0;
}

/** One turn's usage record folded into the session's totals. */
export function addTurnUsage(prev: SessionUsage, raw: any): SessionUsage {
  if (raw === null || typeof raw !== "object") return prev;
  const model: string | null = typeof raw.model === "string" && raw.model.length > 0 ? raw.model : null;
  const turn: TurnUsage = {
    model,
    input: count(raw.input),
    output: count(raw.output),
    cache_read: count(raw.cache_read),
    cache_write: count(raw.cache_write),
  };
  const key = turn.model ?? "unknown";
  const before = prev.byModel[key];
  const merged = {
    model: turn.model,
    input: (before?.input ?? 0) + turn.input,
    output: (before?.output ?? 0) + turn.output,
    cache_read: (before?.cache_read ?? 0) + turn.cache_read,
    cache_write: (before?.cache_write ?? 0) + turn.cache_write,
  };
  // The context figure is the prompt side of the latest request alone: every request
  // resends the whole history, so a running sum would count it over and over.
  const context = turn.input + turn.cache_read + turn.cache_write;
  return { byModel: { ...prev.byModel, [key]: merged }, context };
}

/** The totals rebuilt from a replayed transcript, when a session loads or is switched to. */
export function usageFromMessages(messages: ConversationMessage[]): SessionUsage {
  return (messages ?? [])
    .filter((m) => m.role === "assistant" && m.usage)
    .reduce((acc, m) => addTurnUsage(acc, m.usage), emptyUsage());
}

/** Every token the session used, across models and in both directions: the chip's headline. */
export function totalTokens(u: SessionUsage): number {
  let sum = 0;
  for (const t of Object.values(u.byModel)) sum += t.cache_read + t.cache_write + t.input + t.output;
  return sum;
}

function trimZeros(fixed: string): string {
  return fixed.includes(".") ? fixed.replace(/0+$/, "").replace(/\.$/, "") : fixed;
}

/** Short counts: 980 stays "980"; 12,400 is "12.4k", 982,000 "982k", 1,240,000 "1.24M". */
export function formatTokens(n: number): string {
  if (!(n > 0) || !Number.isFinite(n)) return "0";
  if (n < 1e3) return `${Math.round(n)}`;
  const [value, unit, places] = n < 1e6 ? [n / 1e3, "k", 1] : [n / 1e6, "M", 2];
  return value >= 100 ? `${Math.round(value)}${unit}` : `${trimZeros(value.toFixed(places))}${unit}`;
}
