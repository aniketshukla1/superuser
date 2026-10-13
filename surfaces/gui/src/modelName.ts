// Turning a model id into something that fits on a chip.
//
// Two different jobs, and conflating them is what made the composer's model chip read
// "qwen2.5:7b-instruct · Custom (OpenAI-compatible)" — a string that is genuinely useful
// in a menu and absurd on a control 90px wide. `menuName` is the informative one;
// `chipName` is the one that has to survive a narrow window.

/** The colon in a model id means two different things and the difference matters:
 *  `anthropic:claude-opus-4-8` is provider:model, `gemma4:12b` is model:tag. Stripping
 *  everything before the colon turns the first into "claude-opus-4-8" and the second
 *  into "12b", which is not a model anyone has heard of. So the prefix is only removed
 *  when it is actually a provider — and the list of those comes from the server. */
export function stripProvider(id: string, providers?: string[]): string {
  const at = id.indexOf(":");
  if (at <= 0) return id;
  const head = id.slice(0, at).toLowerCase();
  if (!providers?.some((p) => p.toLowerCase() === head)) return id;
  return id.slice(at + 1);
}

/** The context window a model's own label already states ("llama3.2:latest · Ollama · 131k
 *  context" -> 131000). The verified matrix (`model_context_windows`) covers cloud models;
 *  a model discovered on this Mac has its window in the label and nowhere else, and the
 *  usage panel was saying "this model's window isn't known" beside a picker that named it. */
export function contextFromLabel(label?: string): number | undefined {
  const m = label?.match(/(\d+(?:\.\d+)?)\s*([kKmM])\s+context/);
  if (!m) return undefined;
  const n = Number(m[1]) * (m[2].toLowerCase() === "m" ? 1_000_000 : 1_000);
  return Number.isFinite(n) && n > 0 ? n : undefined;
}

/** The full label: what the server calls it, vendor and all. For menu rows, where the
 *  vendor is the thing telling two similarly-named models apart. */
export function menuName(id: string, labels?: Record<string, string>): string {
  return labels?.[id] || id;
}

/** The short label: for the chip, where there is room for a name and nothing else.
 *
 *  Three cuts, in order, each removing something the chip does not need:
 *    "qwen2.5:7b-instruct · Custom (OpenAI-compatible)"  the curated label
 *    "qwen2.5:7b-instruct"                               …minus the vendor
 *    "qwen2.5"                                           …minus the variant tag
 *
 *  The tag is the last to go and the biggest win. It is also the one that loses real
 *  information — 7b and 72b are different models — which is why the full id stays in
 *  the tooltip and every menu row keeps its whole name. The chip is for recognising
 *  what you already picked, not for picking. */
export function chipName(
  id: string,
  labels?: Record<string, string>,
  providers?: string[],
): string {
  const label = labels?.[id];
  // The curated labels are "<name> · <vendor>". You chose the model; you know where it
  // came from.
  const named = label ? label.split(" · ")[0].trim() : id;
  const base = stripProvider(named, providers);
  // Everything from the tag separator on. `:latest` disappears with the rest of it.
  const cut = base.indexOf(":");
  const name = cut > 0 ? base.slice(0, cut) : base;
  return name || base || id;
}
